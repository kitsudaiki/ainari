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
	"k8s.io/apimachinery/pkg/api/resource"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"

	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// onsenStorage is a statefulset of onsen: the one for the images of ryokan or the one for the
// backups of isekai. Both use the image of onsen and register themselves at ryokan, where an
// admin marks the pods of the backups as backup-targets.
type onsenStorage struct {
	// name of the statefulset, its headless service and its pods
	name         string
	replicas     int32
	storageClass string
	storageSize  resource.Quantity
}

// onsenTemplateData is the data of the template of the config of onsen.
type onsenTemplateData struct {
	*renderer
	// headless service of the statefulset, over which the pods are reachable
	Service string
}

// onsen stores the images of ryokan and, with its second statefulset, the backups of isekai. Every
// pod registers itself at ryokan with its own address, over which ryokan and sakura reach it with
// grpc, so it has no tls-termination.
func (r *renderer) onsen() ([]client.Object, error) {
	backups := &r.Spec.Isekai.BackupStorage

	var objects []client.Object
	for _, storage := range []onsenStorage{
		{"onsen", r.Spec.Onsen.Replicas, r.Spec.Onsen.StorageClass, r.Spec.Onsen.StorageSize},
		{"onsen-backup", backups.Replicas, backups.StorageClass, backups.StorageSize},
	} {
		config, err := r.execute("onsen.toml", onsenTemplateData{renderer: r, Service: storage.name})
		if err != nil {
			return nil, err
		}
		configName := storage.name + "-config"
		objects = append(objects,
			configMap(configName, map[string]string{"onsen.toml": config}),
			headlessService(storage.name, storage.name, append([]corev1.ServicePort{
				servicePort("grpc", onsenGRPCPort, onsenGRPCPort),
			}, r.wireGuardServicePorts()...)...),
			r.onsenStatefulSet(storage, configName),
		)
	}
	return objects, nil
}

// onsenStatefulSet returns a statefulset of onsen. Every pod has its own volume with its files.
func (r *renderer) onsenStatefulSet(storage onsenStorage, configName string) *appsv1.StatefulSet {
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
			envValue("RUST_LOG", r.rustLog("debug,h2=info")),
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
	volumes := []corev1.Volume{configMapVolume("onsen-config", configName)}
	if r.Spec.Global.WireGuard.Enabled {
		main.VolumeMounts = append(main.VolumeMounts, wireGuardMount())
		volumes = append(volumes, wireGuardVolumes(storage.name)...)
		sidecars = append(sidecars, r.wireGuardSidecar(spec.Image, spec.ImagePullPolicy))
	}

	return &appsv1.StatefulSet{
		ObjectMeta: metav1.ObjectMeta{Name: storage.name, Labels: appLabels(storage.name)},
		Spec: appsv1.StatefulSetSpec{
			ServiceName:         storage.name,
			Replicas:            &storage.replicas,
			PodManagementPolicy: appsv1.ParallelPodManagement,
			Selector:            &metav1.LabelSelector{MatchLabels: appLabels(storage.name)},
			Template: corev1.PodTemplateSpec{
				ObjectMeta: metav1.ObjectMeta{Labels: appLabels(storage.name)},
				Spec: corev1.PodSpec{
					Affinity:       r.affinity(storage.name, storage.name+"-node"),
					InitContainers: []corev1.Container{r.waitForMiko()},
					Containers:     append([]corev1.Container{main}, sidecars...),
					Volumes:        volumes,
				},
			},
			VolumeClaimTemplates: []corev1.PersistentVolumeClaim{
				*persistentVolumeClaim("data", r.storageClass(storage.storageClass), storage.storageSize),
			},
		},
	}
}
