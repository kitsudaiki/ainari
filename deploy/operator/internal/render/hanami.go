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

// hanami handles the virtual machines and their networks.
func (r *renderer) hanami() ([]client.Object, error) {
	spec := &r.Spec.Hanami
	config, err := r.execute("hanami.toml", r)
	if err != nil {
		return nil, err
	}

	env := []corev1.EnvVar{
		envSecret("SAKURA_REGISTRATION_KEY", secrets.SakuraRegistrationKey),
	}
	if spec.Network.MLSEncryption {
		// signs the membership-grants of the MLS-groups of the networks
		env = append(env, envSecret("MLS_GRANT_SIGNING_KEY", secrets.MLSGrantSigningKey))
	}

	return r.backendObjects(backend{
		name:        "hanami",
		replicas:    spec.Replicas,
		image:       spec.Image,
		pullPolicy:  spec.ImagePullPolicy,
		rustLog:     "debug",
		probePort:   portsOf("hanami").Public,
		externalAPI: true,
		domain:      spec.Domain,
		config:      configMap("hanami-config", map[string]string{"hanami.toml": config}),
		env:         env,
	})
}
