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

// onsen stores the images of ryokan. Every pod registers itself at ryokan with its own address,
// over which ryokan and sakura reach it with grpc, so it has no tls-termination.
func (r *renderer) onsen() ([]client.Object, error) {
	config, err := r.execute("onsen.toml", r)
	if err != nil {
		return nil, err
	}

	return []client.Object{
		configMap("onsen-config", map[string]string{"onsen.toml": config}),
		headlessService("onsen", "onsen", append([]corev1.ServicePort{
			servicePort("grpc", onsenGRPCPort, onsenGRPCPort),
		}, r.wireGuardServicePorts()...)...),
		r.onsenStatefulSet(),
	}, nil
}

// onsenStatefulSet returns the statefulset of onsen. Every pod has its own volume with its
// images.
func (r *renderer) onsenStatefulSet() *appsv1.StatefulSet {
	spec := &r.Spec.Onsen

	main := corev1.Container{
		Name:            "onsen",
		Image:           spec.Image,
		ImagePullPolicy: r.pullPolicy(spec.ImagePullPolicy),
		// The config contains the address of this pod, which is only known, when it starts. With
		// wireguard it is the address of the pod within the tunnel, which is read from the config
		// of its interface.
		Command: []string{"/bin/sh", "-c",
			`WIREGUARD_ADDRESS="$(sed -n 's|^Address = \([0-9.]*\)/.*|\1|p' /etc/wireguard/wg0.conf 2> /dev/null)"; ` +
				`sed -e "s/@HOSTNAME@/$HOSTNAME/g" -e "s/@WIREGUARD_ADDRESS@/$WIREGUARD_ADDRESS/g" /etc/ainari/templates/onsen.toml > /tmp/onsen.toml && ` +
				`exec /home/ainari/start_onsen.sh`},
		Env: []corev1.EnvVar{
			envValue("CONFIG_FILE", "/tmp/onsen.toml"),
			envValue("RUST_LOG", "debug,h2=info"),
			envInternalAPIKey(),
			envSecret("ONSEN_REGISTRATION_KEY", secrets.OnsenRegistrationKey),
			envPodName(),
		},
		Ports: []corev1.ContainerPort{{Name: "grpc", ContainerPort: onsenGRPCPort}},
		VolumeMounts: []corev1.VolumeMount{
			{Name: "data", MountPath: "/etc/ainari/onsen/"},
			{Name: "onsen-config", MountPath: "/etc/ainari/templates", ReadOnly: true},
		},
		SecurityContext: &corev1.SecurityContext{
			Capabilities: &corev1.Capabilities{
				Add: []corev1.Capability{"NET_ADMIN", "SYS_MODULE"},
			},
		},
	}
	var sidecars []corev1.Container
	volumes := []corev1.Volume{configMapVolume("onsen-config", "onsen-config")}
	if r.Spec.Global.WireGuard.Enabled {
		main.VolumeMounts = append(main.VolumeMounts, wireGuardMount())
		volumes = append(volumes, wireGuardVolumes("onsen")...)
		sidecars = append(sidecars, r.wireGuardSidecar(spec.Image, spec.ImagePullPolicy))
	}

	return &appsv1.StatefulSet{
		ObjectMeta: metav1.ObjectMeta{Name: "onsen", Labels: appLabels("onsen")},
		Spec: appsv1.StatefulSetSpec{
			ServiceName:         "onsen",
			Replicas:            &spec.Replicas,
			PodManagementPolicy: appsv1.ParallelPodManagement,
			Selector:            &metav1.LabelSelector{MatchLabels: appLabels("onsen")},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{Labels: appLabels("onsen")},
				Spec: corev1.PodSpec{
					Affinity:       r.affinity("onsen", "onsen-node"),
					InitContainers: []corev1.Container{r.waitForMiko()},
					Containers:     append([]corev1.Container{main}, sidecars...),
					Volumes:        volumes,
				},
			},
			VolumeClaimTemplates: []corev1.PersistentVolumeClaim{
				*persistentVolumeClaim("data", r.storageClass(spec.StorageClass), spec.StorageSize),
			},
		},
	}
}
