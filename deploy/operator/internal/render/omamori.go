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
	"sigs.k8s.io/controller-runtime/pkg/client"
)

// omamori stores the secrets of the users.
func (r *renderer) omamori() ([]client.Object, error) {
	spec := &r.Spec.Omamori
	config, err := r.execute("omamori.toml", r)
	if err != nil {
		return nil, err
	}

	return r.backendObjects(backend{
		name:        "omamori",
		replicas:    spec.Replicas,
		image:       spec.Image,
		pullPolicy:  spec.ImagePullPolicy,
		rustLog:     "debug",
		probePort:   portsOf("omamori").Internal,
		externalAPI: true,
		domain:      spec.Domain,
		// the config contains the key, with which omamori encrypts the stored secrets
		config: secret("omamori-config", map[string]string{"omamori.toml": config}),
	})
}
