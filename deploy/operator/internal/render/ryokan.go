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
	corev1 "k8s.io/api/core/v1"
	"sigs.k8s.io/controller-runtime/pkg/client"

	"github.com/kitsudaiki/ainari/deploy/operator/internal/secrets"
)

// ryokan handles the images of the virtual machines, which are stored by onsen. It runs as
// statefulset, so every pod has a fixed name, which the onsen-pods reach it over within the
// wireguard-tunnel.
func (r *renderer) ryokan() ([]client.Object, error) {
	spec := &r.Spec.Ryokan
	config, err := r.execute("ryokan.toml", r)
	if err != nil {
		return nil, err
	}

	b := backend{
		name:        "ryokan",
		replicas:    spec.Replicas,
		image:       spec.Image,
		pullPolicy:  spec.ImagePullPolicy,
		rustLog:     "debug,h2=info",
		probePort:   portsOf("ryokan").Internal,
		externalAPI: true,
		domain:      spec.Domain,
		statefulSet: true,
		config:      configMap("ryokan-config", map[string]string{"ryokan.toml": config}),
		env: []corev1.EnvVar{
			envSecret("ONSEN_REGISTRATION_KEY", secrets.OnsenRegistrationKey),
		},
		securityContext: &corev1.SecurityContext{
			SeccompProfile: &corev1.SeccompProfile{Type: corev1.SeccompProfileTypeRuntimeDefault},
			Capabilities: &corev1.Capabilities{
				Add: []corev1.Capability{"NET_ADMIN", "SYS_MODULE"},
			},
		},
	}
	if r.Spec.Global.WireGuard.Enabled {
		b.env = append(b.env, envPodName())
		b.volumes = wireGuardVolumes("ryokan")
		b.volumeMounts = []corev1.VolumeMount{wireGuardMount()}
		b.sidecars = []corev1.Container{r.wireGuardSidecar(spec.Image, spec.ImagePullPolicy)}
	}

	objects, err := r.backendObjects(b)
	if err != nil {
		return nil, err
	}
	return append(objects, headlessService("ryokan", "ryokan", r.wireGuardServicePorts()...)), nil
}
