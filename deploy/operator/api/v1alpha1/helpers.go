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

package v1alpha1

import "fmt"

// ComponentDatabase is the database of a component on the mysql-server.
// +kubebuilder:object:generate=false
type ComponentDatabase struct {
	// name of the component, for example 'miko'
	Component string
	Database  string
	User      string
}

// ByComponent returns the databases of all components in a fixed order. The names of the
// database and the user are the name of the component, if they are not set.
func (d *MySQLDatabasesSpec) ByComponent() []ComponentDatabase {
	specs := []struct {
		component string
		spec      *MySQLDatabaseSpec
	}{
		{"miko", &d.Miko},
		{"hanami", &d.Hanami},
		{"ryokan", &d.Ryokan},
		{"omamori", &d.Omamori},
		{"izakaya", &d.Izakaya},
	}

	databases := make([]ComponentDatabase, 0, len(specs))
	for _, s := range specs {
		database := ComponentDatabase{
			Component: s.component,
			Database:  s.spec.Database,
			User:      s.spec.User,
		}
		if database.Database == "" {
			database.Database = s.component
		}
		if database.User == "" {
			database.User = s.component
		}
		databases = append(databases, database)
	}
	return databases
}

// WireGuardMember is a pod, which takes part in the wireguard-tunnel.
// +kubebuilder:object:generate=false
type WireGuardMember struct {
	// name of the component, for example 'onsen'
	Component string
	// ordinal of the pod within the statefulset of the component
	Ordinal int32
}

// PodName returns the name of the pod of the member.
func (m WireGuardMember) PodName() string {
	return fmt.Sprintf("%s-%d", m.Component, m.Ordinal)
}

// WireGuardComponents are the components, whose pods take part in the wireguard-tunnel, in a
// fixed order.
var WireGuardComponents = []string{"onsen", "ryokan", "sakura"}

// WireGuardMembers returns all pods, which take part in the wireguard-tunnel, or nothing, if it
// is disabled.
func (s *AinariSpec) WireGuardMembers() []WireGuardMember {
	if !s.Global.WireGuard.Enabled {
		return nil
	}
	replicas := map[string]int32{
		"onsen":  s.Onsen.Replicas,
		"ryokan": s.Ryokan.Replicas,
		"sakura": s.Sakura.Replicas,
	}

	var members []WireGuardMember
	for _, component := range WireGuardComponents {
		for ordinal := int32(0); ordinal < replicas[component]; ordinal++ {
			members = append(members, WireGuardMember{Component: component, Ordinal: ordinal})
		}
	}
	return members
}
