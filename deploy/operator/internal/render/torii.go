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

	appsv1 "k8s.io/api/apps/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"
)

// toriiPublicApp is the app of the gateway at the edge of the network. The gateways in front of
// the sakura-hosts run in the pods of sakura.
const toriiPublicApp = "torii-public"

// torii returns the gateway at the edge of the network and the configs, which are shared with
// the gateways in front of the sakura-hosts.
func (r *renderer) torii() ([]client.Object, error) {
	publicConfig, err := r.execute("torii-public.toml", r)
	if err != nil {
		return nil, err
	}
	vmmConfig, err := r.execute("torii-vmm.toml", r)
	if err != nil {
		return nil, err
	}
	startScript, err := r.execute("start_torii.sh", r)
	if err != nil {
		return nil, err
	}
	// Every torii listens on 8444, also the one in the pod of a sakura-host, where 8443 already
	// belongs to sakura.
	nginxConfig, err := r.nginxConfig(nginxParams{
		Listen:         toriiTLSPort,
		Upstream:       portsOf("torii").Internal,
		RedirectDomain: r.Spec.Torii.Domain,
	})
	if err != nil {
		return nil, err
	}
	spec := &r.Spec.Torii

	objects := []client.Object{
		configMap("torii-config", map[string]string{
			"torii-public.toml": publicConfig,
			"torii-vmm.toml":    vmmConfig,
		}),
		configMap("torii-scripts", map[string]string{"start_torii.sh": startScript}),
		configMap(nginxConfigName("torii"), map[string]string{"nginx.conf": nginxConfig}),
		persistentVolumeClaim("torii-pvc", r.storageClass(spec.StorageClass), spec.StorageSize),
		r.toriiDeployment(),
		// Resolves to the address of the pod itself, which hanami compares with the address of
		// the torii of every sakura-host.
		headlessService(toriiPublicApp, toriiPublicApp, servicePort("tls", toriiTLSPort, toriiTLSPort)),
		tlsService("torii", toriiPublicApp, toriiTLSPort),
		r.toriiGatewayService(),
		r.certificate("torii", r.Spec.Torii.Domain, tlsServiceName("torii"), toriiPublicApp, "*.sakura"),
	}
	return append(objects, r.ingress("torii", r.Spec.Torii.Domain, tlsServiceName("torii"), internalTLSPort)...), nil
}

// toriiContainer returns the container of a torii with the given config, which is shared by the
// gateway at the edge and the ones in front of the sakura-hosts.
func (r *renderer) toriiContainer(configTemplate string) corev1.Container {
	spec := &r.Spec.Torii
	privileged := true
	return corev1.Container{
		Name:            "torii",
		Image:           spec.Image,
		ImagePullPolicy: r.pullPolicy(spec.ImagePullPolicy),
		Command:         []string{"/bin/bash", "/etc/ainari/scripts/start_torii.sh"},
		// the eBPF-datapath is attached to the interfaces of the pod and the TAP-devices are
		// created over /dev/net/tun
		SecurityContext: &corev1.SecurityContext{Privileged: &privileged},
		Env: []corev1.EnvVar{
			envValue("RUST_LOG", "debug"),
			envValue("CONFIG_TEMPLATE", configTemplate),
			envInternalAPIKey(),
		},
		VolumeMounts: []corev1.VolumeMount{
			{Name: "torii-config", MountPath: "/etc/ainari/templates", ReadOnly: true},
			{Name: "torii-scripts", MountPath: "/etc/ainari/scripts", ReadOnly: true},
			{Name: "debugfs", MountPath: "/sys/kernel/debug", ReadOnly: true},
		},
	}
}

// toriiVolumes returns the volumes of the mounts of toriiContainer.
func toriiVolumes() []corev1.Volume {
	return []corev1.Volume{
		configMapVolume("torii-config", "torii-config"),
		configMapVolume("torii-scripts", "torii-scripts"),
		hostPathVolume("debugfs", "/sys/kernel/debug", nil),
	}
}

// toriiDeployment returns the gateway at the edge of the network.
func (r *renderer) toriiDeployment() *appsv1.Deployment {
	spec := &r.Spec.Torii

	torii := r.toriiContainer("/etc/ainari/templates/torii-public.toml")
	if spec.Public.WaitForIface != "" {
		// the uplink is moved into the pod from the outside, so torii waits for it and resolves
		// the next hop on it, before it programs its routes
		torii.Env = append(torii.Env,
			envValue("WAIT_FOR_IFACE", spec.Public.WaitForIface),
			envValue("UPLINK_NEXT_HOP", spec.Public.UplinkNextHop),
		)
	}
	torii.VolumeMounts = append([]corev1.VolumeMount{
		{Name: "data-volume", MountPath: "/etc/ainari/torii/"},
	}, torii.VolumeMounts...)

	// Torii only listens on localhost and its image has no curl, so it is asked over its
	// tls-termination. The gateway waits for its uplink, before it starts, which can take a while.
	ready := httpsGet("/v1alpha/is_ready", toriiTLSPort)
	torii.StartupProbe = &corev1.Probe{ProbeHandler: ready, PeriodSeconds: 5, TimeoutSeconds: 2, FailureThreshold: 120}
	torii.ReadinessProbe = probe(ready, probeTiming{period: 10, failures: 3})
	torii.LivenessProbe = probe(ready, probeTiming{period: 20, failures: 3})

	volumes := append(toriiVolumes(),
		corev1.Volume{
			Name: "data-volume",
			VolumeSource: corev1.VolumeSource{
				PersistentVolumeClaim: &corev1.PersistentVolumeClaimVolumeSource{ClaimName: "torii-pvc"},
			},
		},
	)
	volumes = append(volumes, tlsVolumes("torii")...)

	return &appsv1.Deployment{
		ObjectMeta: metav1.ObjectMeta{Name: toriiPublicApp, Labels: appLabels(toriiPublicApp)},
		Spec: appsv1.DeploymentSpec{
			Replicas: &spec.Replicas,
			// the uplink belongs to exactly one pod, so the old one has to be gone first
			Strategy: appsv1.DeploymentStrategy{Type: appsv1.RecreateDeploymentStrategyType},
			Selector: &metav1.LabelSelector{MatchLabels: appLabels(toriiPublicApp)},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{Labels: appLabels(toriiPublicApp)},
				Spec: corev1.PodSpec{
					Affinity: r.affinity(toriiPublicApp, "torii-node"),
					Containers: []corev1.Container{
						torii,
						r.tlsSidecar("tls-termination", nginxConfigName("torii"), "nginx.conf", tlsCertsVolume),
					},
					Volumes: volumes,
				},
			},
		},
	}
}

// toriiGatewayService returns the service, which publishes the proxy-ports of the gateway at the
// edge.
func (r *renderer) toriiGatewayService() *corev1.Service {
	spec := &r.Spec.Torii
	external := r.Spec.Global.ExternalServices
	withNodePorts := spec.ServiceType == corev1.ServiceTypeNodePort && external.Enabled

	var servicePorts []corev1.ServicePort
	for port := spec.PortRange.Start; port <= spec.PortRange.End; port++ {
		servicePort := servicePort(fmt.Sprintf("p-%d", port), port, port)
		if withNodePorts {
			servicePort.NodePort = port + external.NodePortOffset
		}
		servicePorts = append(servicePorts, servicePort)
	}

	return &corev1.Service{
		ObjectMeta: metav1.ObjectMeta{Name: "torii-gateway-service"},
		Spec: corev1.ServiceSpec{
			Type:     spec.ServiceType,
			Selector: appLabels(toriiPublicApp),
			Ports:    servicePorts,
		},
	}
}
