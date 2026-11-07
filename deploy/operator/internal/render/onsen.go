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
	// port of the tls-termination of the internal api of every pod
	Port int32
}

// onsen stores the images of ryokan and, with its second statefulset, the backups of isekai. Every
// pod registers itself at ryokan with the address of its own tls-termination, over which ryokan
// and sakura reach its internal api, because a file is always stored on one specific pod.
func (r *renderer) onsen() ([]client.Object, error) {
	backups := &r.Spec.Isekai.BackupStorage

	var objects []client.Object
	for _, storage := range []onsenStorage{
		{"onsen", r.Spec.Onsen.Replicas, r.Spec.Onsen.StorageClass, r.Spec.Onsen.StorageSize},
		{"onsen-backup", backups.Replicas, backups.StorageClass, backups.StorageSize},
	} {
		config, err := r.execute("onsen.toml", onsenTemplateData{
			renderer: r, Service: storage.name, Port: internalTLSPort,
		})
		if err != nil {
			return nil, err
		}
		nginx, err := r.nginxConfig(nginxParams{
			Listen: internalTLSPort, Upstream: portsOf("onsen").Internal, LargeFiles: true,
		})
		if err != nil {
			return nil, err
		}
		configName := storage.name + "-config"
		objects = append(objects,
			configMap(configName, map[string]string{"onsen.toml": config}),
			configMap(nginxConfigName(storage.name), map[string]string{"internal.conf": nginx}),
			headlessService(storage.name, storage.name, servicePort("tls", internalTLSPort, internalTLSPort)),
			// every pod is reached over its own name within the headless service
			r.certificate(storage.name, "", "*."+storage.name),
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
		// the config contains the address of this pod, which is only known, when it starts
		Command: []string{"/bin/sh", "-c",
			`sed "s/@HOSTNAME@/$HOSTNAME/g" /etc/ainari/templates/onsen.toml > /tmp/onsen.toml && ` +
				`exec /home/ainari/start_onsen.sh`},
		Env: []corev1.EnvVar{
			envValue("CONFIG_FILE", "/tmp/onsen.toml"),
			envValue("RUST_LOG", r.rustLog("debug,h2=info")),
			envInternalAPIKey(),
			envSecret("ONSEN_REGISTRATION_KEY", secrets.OnsenRegistrationKey),
		},
		VolumeMounts: []corev1.VolumeMount{
			{Name: "data", MountPath: "/etc/ainari/onsen/"},
			{Name: "onsen-config", MountPath: "/etc/ainari/templates", ReadOnly: true},
		},
		ReadinessProbe: probe(curlReady(portsOf("onsen").Public), readiness),
		LivenessProbe:  probe(curlReady(portsOf("onsen").Public), liveness),
	}
	// tls-termination of the internal api, which ryokan and sakura reach
	tls := r.tlsSidecar("tls-termination", nginxConfigName(storage.name), "internal.conf", tlsCertsVolume)
	tls.Ports = []corev1.ContainerPort{{Name: "tls", ContainerPort: internalTLSPort}}

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
					Containers:     []corev1.Container{main, tls},
					Volumes: append([]corev1.Volume{
						configMapVolume("onsen-config", configName),
					}, tlsVolumes(storage.name)...),
				},
			},
			VolumeClaimTemplates: []corev1.PersistentVolumeClaim{
				*persistentVolumeClaim("data", r.storageClass(storage.storageClass), storage.storageSize),
			},
		},
	}
}
