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

// izakaya coordinates the MLS-groups of the gateways. It is only needed for the encryption of
// the networks.
func (r *renderer) izakaya() ([]client.Object, error) {
	if !r.Spec.Hanami.Network.MLSEncryption {
		return nil, nil
	}
	spec := &r.Spec.Izakaya
	config, err := r.execute("izakaya.toml", r)
	if err != nil {
		return nil, err
	}

	return r.backendObjects(backend{
		name:       "izakaya",
		replicas:   spec.Replicas,
		image:      spec.Image,
		pullPolicy: spec.ImagePullPolicy,
		rustLog:    "debug",
		probePort:  portsOf("izakaya").Internal,
		domain:     spec.Domain,
		config:     configMap("izakaya-config", map[string]string{"izakaya.toml": config}),
	})
}
