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

	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// sakura runs the virtual machines. Every pod is a sakura-host with a torii in front of it, which
// owns the TAP-devices of its virtual machines. Every pod is reached over its own name within
// the headless service 'sakura'.
func (r *renderer) sakura() ([]client.Object, error) {
	config, err := r.execute("sakura.toml", r)
	if err != nil {
		return nil, err
	}
	nginxConfig, err := r.apiNginxConfigMap("sakura")
	if err != nil {
		return nil, err
	}

	return []client.Object{
		configMap("sakura-config", map[string]string{"sakura.toml": config}),
		nginxConfig,
		headlessService("sakura", "sakura", append([]corev1.ServicePort{
			servicePort("tls", internalTLSPort, internalTLSPort),
			servicePort("external-tls", externalTLSPort, externalTLSPort),
			servicePort("torii-tls", toriiTLSPort, toriiTLSPort),
		}, r.wireGuardServicePorts()...)...),
		r.certificate("sakura", r.Spec.Sakura.Domain, "*.sakura"),
		r.sakuraStatefulSet(),
	}, nil
}

// sakuraStatefulSet returns the statefulset of the sakura-hosts.
func (r *renderer) sakuraStatefulSet() *appsv1.StatefulSet {
	spec := &r.Spec.Sakura
	privileged := true

	// The directories of the pod on the node are created by the kubelet as root, but sakura runs
	// as a normal user, so they are prepared for it first. The owner is set by name, so it fits
	// every variant of the image.
	rootUser := int64(0)
	prepareHostData := corev1.Container{
		Name:            "prepare-host-data",
		Image:           spec.Image,
		ImagePullPolicy: r.pullPolicy(spec.ImagePullPolicy),
		SecurityContext: &corev1.SecurityContext{RunAsUser: &rootUser},
		Command: []string{"/bin/sh", "-c",
			"mkdir -p /host-data/torii /host-data/sakura/virtual_machines && " +
				"chown ainari:ainari /host-data/sakura /host-data/sakura/virtual_machines"},
		Env:          []corev1.EnvVar{envPodName()},
		VolumeMounts: []corev1.VolumeMount{{Name: "host-data", MountPath: "/host-data", SubPathExpr: "$(POD_NAME)"}},
	}

	// the gateway in front of this sakura-host, which owns the TAP-devices of its virtual
	// machines
	torii := r.toriiContainer("/etc/ainari/templates/torii-vmm.toml")
	torii.Env = append(torii.Env,
		envValue("DEFAULT_GATEWAY_HOST", fmt.Sprintf("torii-public.%s.svc.cluster.local", r.Namespace)),
		envPodName(),
	)
	// The database of the gateway lies in the directory of the pod on the node, so a recreated
	// pod restores its TAP-devices, routes, packet-filters and MLS-state from it.
	torii.VolumeMounts = append([]corev1.VolumeMount{
		{Name: "host-data", MountPath: "/etc/ainari/torii/", SubPathExpr: "$(POD_NAME)/torii"},
	}, torii.VolumeMounts...)
	torii.ReadinessProbe = probe(httpsGet("/v1alpha/is_ready", toriiTLSPort), readiness)
	torii.LivenessProbe = probe(httpsGet("/v1alpha/is_ready", toriiTLSPort), liveness)

	sakura := corev1.Container{
		Name:            "sakura",
		Image:           spec.Image,
		ImagePullPolicy: r.pullPolicy(spec.ImagePullPolicy),
		// the config contains the address of this pod, which is only known, when it starts
		Command: []string{"/bin/sh", "-c",
			"mkdir -p /etc/ainari/sakura/virtual_machines && " +
				`sed "s/@HOSTNAME@/$HOSTNAME/g" /etc/ainari/templates/sakura.toml > /tmp/sakura.toml && ` +
				"exec /home/ainari/start_sakura.sh"},
		// Kubernetes has no way to hand single devices like /dev/kvm and /dev/net/tun to a
		// container. Sakura still runs as a normal user, only cloud-hypervisor carries
		// CAP_NET_ADMIN as a file-capability.
		SecurityContext: &corev1.SecurityContext{Privileged: &privileged},
		Env: []corev1.EnvVar{
			envValue("CONFIG_FILE", "/tmp/sakura.toml"),
			envValue("RUST_LOG", "debug,h2=info"),
			envInternalAPIKey(),
			envSecret("SAKURA_REGISTRATION_KEY", secrets.SakuraRegistrationKey),
			envPodName(),
		},
		VolumeMounts: []corev1.VolumeMount{
			// the database of sakura and the disks of the virtual machines lie in the directory
			// of the pod on the node
			{Name: "host-data", MountPath: "/etc/ainari/sakura/", SubPathExpr: "$(POD_NAME)/sakura"},
			{Name: "sakura-config", MountPath: "/etc/ainari/templates", ReadOnly: true},
		},
		ReadinessProbe: probe(curlReady(portsOf("sakura").Public), readiness),
		LivenessProbe:  probe(curlReady(portsOf("sakura").Public), liveness),
	}

	// tls-termination of the torii in front of this sakura-host. Hanami reaches it on the same
	// port as the one of the gateway at the edge.
	toriiTLS := r.tlsSidecar("torii-tls-termination", nginxConfigName("torii"), "nginx.conf", "torii-tls-certs")
	toriiTLS.Ports = []corev1.ContainerPort{{ContainerPort: toriiTLSPort}}

	directoryOrCreate := corev1.HostPathDirectoryOrCreate
	volumes := []corev1.Volume{
		configMapVolume("sakura-config", "sakura-config"),
		hostPathVolume("host-data", spec.HostDataPath, &directoryOrCreate),
		secretVolume("torii-tls-certs", tlsSecretName("torii")),
		configMapVolume(nginxConfigName("torii"), nginxConfigName("torii")),
	}
	volumes = append(volumes, toriiVolumes()...)
	volumes = append(volumes, tlsVolumes("sakura")...)
	var wireGuardSync []corev1.Container
	if r.Spec.Global.WireGuard.Enabled {
		sakura.VolumeMounts = append(sakura.VolumeMounts, wireGuardMount())
		volumes = append(volumes, wireGuardVolumes("sakura")...)
		wireGuardSync = append(wireGuardSync, r.wireGuardSidecar(spec.Image, spec.ImagePullPolicy))
	}

	// tls-termination of the internal api (8443) for hanami and of the external api (9443), which
	// the proxy-port of the torii at the edge forwards the clients to
	containers := append([]corev1.Container{torii, sakura}, r.apiTLSSidecars("sakura")...)
	containers = append(containers, toriiTLS)
	containers = append(containers, wireGuardSync...)

	return &appsv1.StatefulSet{
		ObjectMeta: metav1.ObjectMeta{Name: "sakura", Labels: appLabels("sakura")},
		Spec: appsv1.StatefulSetSpec{
			ServiceName:         "sakura",
			Replicas:            &spec.Replicas,
			PodManagementPolicy: appsv1.ParallelPodManagement,
			Selector:            &metav1.LabelSelector{MatchLabels: appLabels("sakura")},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{Labels: appLabels("sakura")},
				Spec: corev1.PodSpec{
					Affinity:       r.affinity("sakura", "sakura-node"),
					InitContainers: []corev1.Container{r.waitForMiko(), prepareHostData},
					// sakura runs as a normal user and opens the virtual machines over /dev/kvm
					SecurityContext: &corev1.PodSecurityContext{SupplementalGroups: []int64{spec.KVMGID}},
					Containers:      containers,
					Volumes:         volumes,
				},
			},
		},
	}
}
