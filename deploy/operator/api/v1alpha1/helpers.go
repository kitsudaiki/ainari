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
		{"omikuji", &d.Omikuji},
		{"isekai", &d.Isekai},
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
