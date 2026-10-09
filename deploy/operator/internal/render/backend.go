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
	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"

	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// backend describes a component, which stores its state in the mysql-server and runs as a
// deployment or statefulset behind its nginx-sidecars: miko, hanami, ryokan, omamori and izakaya.
type backend struct {
	name       string
	replicas   int32
	image      string
	pullPolicy corev1.PullPolicy

	// value of RUST_LOG
	rustLog string
	// port of the readiness-endpoint of the component on localhost
	probePort int32
	// Only the other components talk to a component without an external api, so it has neither
	// an external service nor an ingress.
	externalAPI bool
	// domain of the ingress, which the certificate is also valid for
	domain string

	// Config of the component in the key '<name>.toml', which is mounted to
	// '/etc/ainari/<name>.toml'. Either a configmap or a secret, if it contains a secret value.
	config client.Object

	// Runs the component as statefulset behind the headless service '<name>', so every pod has a
	// fixed name, instead of a deployment.
	statefulSet bool

	// additional env-variables, volumes and mounts of the container of the component
	env             []corev1.EnvVar
	volumes         []corev1.Volume
	volumeMounts    []corev1.VolumeMount
	securityContext *corev1.SecurityContext
	// additional sidecars beside the nginx-sidecars
	sidecars []corev1.Container
}

// backendObjects returns all objects of a backend-component.
func (r *renderer) backendObjects(b backend) ([]client.Object, error) {
	nginxConfig, err := r.backendNginxConfigMap(b)
	if err != nil {
		return nil, err
	}

	objects := []client.Object{
		b.config,
		nginxConfig,
		r.backendWorkload(b),
		tlsService(b.name, b.name, internalTLSPort),
		r.certificate(b.name, b.domain, tlsServiceName(b.name)),
	}
	if b.externalAPI {
		port := portsOf(b.name).Public
		objects = append(objects, r.externalService(b.name, port, externalTLSPort))
		objects = append(objects, r.ingress(b.name, b.domain, externalServiceName(b.name), port)...)
	}
	return objects, nil
}

// backendNginxConfigMap returns the configs of the nginx-sidecars of a backend-component.
func (r *renderer) backendNginxConfigMap(b backend) (*corev1.ConfigMap, error) {
	if b.externalAPI {
		return r.apiNginxConfigMap(b.name)
	}
	internal, err := r.nginxConfig(nginxParams{Listen: internalTLSPort, Upstream: portsOf(b.name).Internal})
	if err != nil {
		return nil, err
	}
	return configMap(nginxConfigName(b.name), map[string]string{"internal.conf": internal}), nil
}

// backendWorkload returns the deployment or statefulset of a backend-component.
func (r *renderer) backendWorkload(b backend) client.Object {
	meta := metav1.ObjectMeta{Name: b.name, Labels: appLabels(b.name)}
	selector := &metav1.LabelSelector{MatchLabels: appLabels(b.name)}

	if b.statefulSet {
		return &appsv1.StatefulSet{
			ObjectMeta: meta,
			Spec: appsv1.StatefulSetSpec{
				ServiceName:         b.name,
				Replicas:            &b.replicas,
				PodManagementPolicy: appsv1.ParallelPodManagement,
				Selector:            selector,
				Template:            r.backendPodTemplate(b),
			},
		}
	}
	return &appsv1.Deployment{
		ObjectMeta: meta,
		Spec: appsv1.DeploymentSpec{
			Replicas: &b.replicas,
			Strategy: singleNodeRollout(),
			Selector: selector,
			Template: r.backendPodTemplate(b),
		},
	}
}

// backendPodTemplate returns the pods of a backend-component.
func (r *renderer) backendPodTemplate(b backend) corev1.PodTemplateSpec {
	configVolume := b.name + "-config"
	configFile := b.name + ".toml"

	env := []corev1.EnvVar{
		envSecret("AINARI_MYSQL_PASSWORD", secrets.MySQLPassword(&r.Spec.MySQL, b.name)),
		envValue("RUST_LOG", b.rustLog),
		envInternalAPIKey(),
	}

	main := corev1.Container{
		Name:            b.name,
		Image:           b.image,
		ImagePullPolicy: r.pullPolicy(b.pullPolicy),
		Env:             append(env, b.env...),
		VolumeMounts: append([]corev1.VolumeMount{
			fileMount(configVolume, "/etc/ainari/"+configFile, configFile),
		}, b.volumeMounts...),
		SecurityContext: b.securityContext,
		ReadinessProbe:  probe(curlReady(b.probePort), readiness),
		LivenessProbe:   probe(curlReady(b.probePort), liveness),
	}

	containers := []corev1.Container{main}
	if b.externalAPI {
		containers = append(containers, r.apiTLSSidecars(b.name)...)
	} else {
		containers = append(containers, r.tlsSidecar("tls-termination", nginxConfigName(b.name), "internal.conf", tlsCertsVolume))
	}
	containers = append(containers, b.sidecars...)

	volumes := []corev1.Volume{configObjectVolume(configVolume, b.config)}
	volumes = append(volumes, tlsVolumes(b.name)...)
	volumes = append(volumes, b.volumes...)

	return corev1.PodTemplateSpec{
		ObjectMeta: metav1.ObjectMeta{Labels: appLabels(b.name)},
		Spec: corev1.PodSpec{
			Affinity:       r.affinity(b.name, b.name+"-node"),
			InitContainers: []corev1.Container{r.waitForMysql()},
			Containers:     containers,
			Volumes:        volumes,
		},
	}
}

// configObjectVolume returns a volume with the content of a configmap or a secret.
func configObjectVolume(name string, config client.Object) corev1.Volume {
	if _, ok := config.(*corev1.Secret); ok {
		return secretVolume(name, config.GetName())
	}
	return configMapVolume(name, config.GetName())
}
