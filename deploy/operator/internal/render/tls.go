// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
//
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package render

import (
	"fmt"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

// Every component talks https over a nginx-sidecar with a certificate of cert-manager, which is
// signed by one CA, so a client only has to trust this CA.

const (
	// issuer, which signs the certificates of all components
	caIssuerName = "ainari-ca-issuer"
	// secret of the CA, which cert-manager creates, if no own CA is configured
	defaultCASecret = "ainari-ca"

	// names of the volumes of a nginx-sidecar
	tlsCertsVolume = "tls-certs"
)

// certManagerObject returns an empty object of cert-manager. cert-manager is not imported as a
// library, so its objects are created as unstructured ones.
func certManagerObject(kind, name string) *unstructured.Unstructured {
	obj := &unstructured.Unstructured{}
	obj.SetAPIVersion("cert-manager.io/v1")
	obj.SetKind(kind)
	obj.SetName(name)
	return obj
}

// issuer returns the issuer, which signs the certificates of all components. Without an own CA,
// cert-manager creates a self-signed one.
func (r *renderer) issuer() ([]client.Object, error) {
	var objects []client.Object
	caSecret := r.Spec.Global.Certificates.CASecret

	if caSecret == "" {
		caSecret = defaultCASecret

		selfSigned := certManagerObject("Issuer", "selfsigned-issuer")
		selfSigned.Object["spec"] = map[string]any{"selfSigned": map[string]any{}}

		ca := certManagerObject("Certificate", "ainari-ca")
		ca.Object["spec"] = map[string]any{
			"isCA":       true,
			"commonName": "ainari-ca",
			"secretName": caSecret,
			// 10 years
			"duration":   "87600h",
			"privateKey": map[string]any{"algorithm": "ECDSA", "size": int64(256)},
			"issuerRef":  map[string]any{"name": "selfsigned-issuer"},
		}
		objects = append(objects, selfSigned, ca)
	}

	issuer := certManagerObject("Issuer", caIssuerName)
	issuer.Object["spec"] = map[string]any{"ca": map[string]any{"secretName": caSecret}}
	return append(objects, issuer), nil
}

// certificate returns the certificate of the tls-termination of a component, which is stored in
// the secret tlsSecretName(name). It is valid for the given services within the namespace, the
// domain of the ingress and the names and addresses of global.certificates, like 127.0.0.1 in
// the kind-setup.
func (r *renderer) certificate(name, domain string, services ...string) *unstructured.Unstructured {
	var dnsNames []any
	for _, service := range services {
		dnsNames = append(dnsNames, fmt.Sprintf("%s.%s.svc.cluster.local", service, r.Namespace))
	}
	if r.Spec.Global.Ingress.Enabled {
		dnsNames = append(dnsNames, domain)
	}
	for _, extra := range r.Spec.Global.Certificates.ExtraDNSNames {
		dnsNames = append(dnsNames, extra)
	}

	spec := map[string]any{
		"secretName": tlsSecretName(name),
		"issuerRef":  map[string]any{"name": caIssuerName},
		"dnsNames":   dnsNames,
	}
	if addresses := r.Spec.Global.Certificates.ExtraIPAddresses; len(addresses) > 0 {
		var ipAddresses []any
		for _, address := range addresses {
			ipAddresses = append(ipAddresses, address)
		}
		spec["ipAddresses"] = ipAddresses
	}

	cert := certManagerObject("Certificate", name+"-cert")
	cert.Object["spec"] = spec
	return cert
}

// nginxParams are the parameters of the template of a nginx-sidecar.
type nginxParams struct {
	// port, where nginx listens with tls
	Listen int32
	// port of the component on localhost
	Upstream int32
	// domain, to which redirects of the component are rewritten, if set
	RedirectDomain string
	// additionally redirects http on port 8080 to https
	HTTPRedirect bool
}

// nginxConfig returns the config of a nginx-sidecar, which terminates tls and forwards the
// requests to a port of the component within the pod.
func (r *renderer) nginxConfig(params nginxParams) (string, error) {
	return r.execute("nginx-sidecar.conf", params)
}

// apiNginxConfigMap returns the configmap with the configs of the two nginx-sidecars of a
// component, which has an internal and an external api. The internal one listens on 8443 for
// '<component>-tls-service' and forwards to the internal port of the component, the external
// one listens on 9443 for '<component>-external' and the ingress and forwards to the public port
// of the component.
func (r *renderer) apiNginxConfigMap(name string) (*corev1.ConfigMap, error) {
	p := portsOf(name)
	internal, err := r.nginxConfig(nginxParams{Listen: internalTLSPort, Upstream: p.Internal})
	if err != nil {
		return nil, err
	}
	external, err := r.nginxConfig(nginxParams{Listen: externalTLSPort, Upstream: p.Public})
	if err != nil {
		return nil, err
	}
	return configMap(nginxConfigName(name), map[string]string{
		"internal.conf": internal,
		"external.conf": external,
	}), nil
}

// tlsSidecar returns a nginx-sidecar, which uses the config with the given key of the configmap
// in the volume nginxVolume and the certificate in the volume certVolume.
func (r *renderer) tlsSidecar(name, nginxVolume, key, certVolume string) corev1.Container {
	return corev1.Container{
		Name:  name,
		Image: r.Spec.Global.NginxImage,
		VolumeMounts: []corev1.VolumeMount{
			{Name: certVolume, MountPath: "/etc/nginx/certs"},
			{Name: nginxVolume, MountPath: "/etc/nginx/nginx.conf", SubPath: key},
		},
		Resources: corev1.ResourceRequirements{
			// Without requests, kubernetes reserves the limits for the sidecar, which would block
			// half a cpu per sidecar, although the tls-termination needs only a fraction of it.
			Requests: corev1.ResourceList{
				corev1.ResourceMemory: resource.MustParse("32Mi"),
				corev1.ResourceCPU:    resource.MustParse("50m"),
			},
			Limits: corev1.ResourceList{
				corev1.ResourceMemory: resource.MustParse("128Mi"),
				corev1.ResourceCPU:    resource.MustParse("500m"),
			},
		},
	}
}

// apiTLSSidecars returns the two nginx-sidecars of a component with an internal and an external
// api, see apiNginxConfigMap.
func (r *renderer) apiTLSSidecars(name string) []corev1.Container {
	return []corev1.Container{
		r.tlsSidecar("tls-termination", nginxConfigName(name), "internal.conf", tlsCertsVolume),
		r.tlsSidecar("external-tls-termination", nginxConfigName(name), "external.conf", tlsCertsVolume),
	}
}

// tlsVolumes returns the volumes of the nginx-sidecars of a component: its certificate and the
// configmap with the configs of nginx.
func tlsVolumes(name string) []corev1.Volume {
	return []corev1.Volume{
		secretVolume(tlsCertsVolume, tlsSecretName(name)),
		configMapVolume(nginxConfigName(name), nginxConfigName(name)),
	}
}

// configMap returns a configmap with the given data.
func configMap(name string, data map[string]string) *corev1.ConfigMap {
	return &corev1.ConfigMap{
		ObjectMeta: metav1.ObjectMeta{Name: name},
		Data:       data,
	}
}

// secret returns an opaque secret with the given data.
func secret(name string, data map[string]string) *corev1.Secret {
	converted := make(map[string][]byte, len(data))
	for key, value := range data {
		converted[key] = []byte(value)
	}
	return &corev1.Secret{
		ObjectMeta: metav1.ObjectMeta{Name: name},
		Type:       corev1.SecretTypeOpaque,
		Data:       converted,
	}
}
