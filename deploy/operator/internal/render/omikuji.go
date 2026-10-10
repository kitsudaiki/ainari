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

// omikuji collects the error-messages and audit-logs of all components.
func (r *renderer) omikuji() ([]client.Object, error) {
	spec := &r.Spec.Omikuji
	config, err := r.execute("omikuji.toml", r)
	if err != nil {
		return nil, err
	}

	return r.backendObjects(backend{
		name:       "omikuji",
		replicas:   spec.Replicas,
		image:      spec.Image,
		pullPolicy: spec.ImagePullPolicy,
		rustLog:    "debug",
		probePort:  portsOf("omikuji").Internal,
		// the admins read the logs over the external api
		externalAPI: true,
		domain:      spec.Domain,
		config:      configMap("omikuji-config", map[string]string{"omikuji.toml": config}),
	})
}
