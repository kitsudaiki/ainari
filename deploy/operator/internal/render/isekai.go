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

// isekai backs up and restores virtual machines and whole projects. It stores no data itself, the
// backups are stored on the onsen of backupStorage, which is rendered together with onsen.
func (r *renderer) isekai() ([]client.Object, error) {
	spec := &r.Spec.Isekai
	config, err := r.execute("isekai.toml", r)
	if err != nil {
		return nil, err
	}

	return r.backendObjects(backend{
		name:       "isekai",
		replicas:   spec.Replicas,
		image:      spec.Image,
		pullPolicy: spec.ImagePullPolicy,
		rustLog:    r.rustLog("debug"),
		probePort:  portsOf("isekai").Internal,
		// the users create and restore their backups over the external api
		externalAPI: true,
		domain:      spec.Domain,
		config:      configMap("isekai-config", map[string]string{"isekai.toml": config}),
	})
}
