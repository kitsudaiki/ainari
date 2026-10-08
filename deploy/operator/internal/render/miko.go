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

// miko handles the users, their tokens and the endpoints of the other components.
func (r *renderer) miko() ([]client.Object, error) {
	spec := &r.Spec.Miko
	config, err := r.execute("miko.toml", r)
	if err != nil {
		return nil, err
	}

	return r.backendObjects(backend{
		name:        "miko",
		replicas:    spec.Replicas,
		image:       spec.Image,
		pullPolicy:  spec.ImagePullPolicy,
		rustLog:     "debug",
		probePort:   portsOf("miko").Internal,
		externalAPI: true,
		domain:      spec.Domain,
		config:      configMap("miko-config", map[string]string{"miko.toml": config}),
		env: []corev1.EnvVar{
			envValue("AINARI_ADMIN_ID", spec.Admin.ID),
			envValue("AINARI_ADMIN_NAME", spec.Admin.Name),
			envSecret("AINARI_ADMIN_PASSPHRASE", secrets.MikoAdminPassphrase),
		},
		volumes:      []corev1.Volume{secretVolume("token-key", secrets.MikoTokenKey.Secret)},
		volumeMounts: []corev1.VolumeMount{fileMount("token-key", "/etc/ainari/token_key", secrets.MikoTokenKey.Key)},
	})
}
