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
	"bytes"
	"embed"
	"encoding/json"
	"fmt"
	"strings"
	"text/template"
)

// templates contains the configs and scripts of the components.
//
//go:embed templates/*
var templates embed.FS

// execute renders a template of the directory 'templates' with the given data. Beside the
// common functions of text/template, the templates can use the functions of templateFuncs.
func (r *renderer) execute(name string, data any) (string, error) {
	tmpl, err := template.New(name).Funcs(r.templateFuncs()).ParseFS(templates, "templates/"+name)
	if err != nil {
		return "", fmt.Errorf("failed to parse the template '%s': %w", name, err)
	}

	var out bytes.Buffer
	if err := tmpl.Execute(&out, data); err != nil {
		return "", fmt.Errorf("failed to render the template '%s': %w", name, err)
	}
	return out.String(), nil
}

// templateFuncs returns the functions, which the templates can use.
func (r *renderer) templateFuncs() template.FuncMap {
	return template.FuncMap{
		"quote":           tomlString,
		"json":            jsonString,
		"upper":           strings.ToUpper,
		"mikoAddress":     r.mikoAddress,
		"publicAddress":   r.publicAddress,
		"internalAddress": r.internalAddress,
		"api":             r.apiConfig,
		"mysql":           r.mysqlConfig,
	}
}

// apiConfig returns the config-group of the api of a component, which only listens on localhost,
// because it is only reachable over its tls-termination.
func (r *renderer) apiConfig(name string) string {
	p := portsOf(name)
	return fmt.Sprintf(`[api]
public_ip = "127.0.0.1"
public_port = %d
internal_ip = "127.0.0.1"
internal_port = %d`, p.Public, p.Internal)
}

// mysqlConfig returns the config-group of the mysql-database of a component. The value
// 'database_type = "mysql"' has to be set separately in the root of the config.
func (r *renderer) mysqlConfig(name string) string {
	database := r.database(name)
	return fmt.Sprintf(`# the password is read from the env-variable AINARI_MYSQL_PASSWORD
[mysql]
host = %s
port = %d
user = %s
database = %s`, tomlString(r.mysqlHost()), r.Spec.MySQL.Port, tomlString(database.User), tomlString(database.Database))
}

// tomlString returns a value as basic string of TOML, with quotes.
func tomlString(value string) string {
	var out strings.Builder
	out.WriteByte('"')
	for _, c := range value {
		switch {
		case c == '"' || c == '\\':
			out.WriteByte('\\')
			out.WriteRune(c)
		case c < 0x20 || c == 0x7f:
			fmt.Fprintf(&out, "\\u%04X", c)
		default:
			out.WriteRune(c)
		}
	}
	out.WriteByte('"')
	return out.String()
}

// jsonString returns a value as string of JSON, with quotes.
func jsonString(value string) (string, error) {
	encoded, err := json.Marshal(value)
	return string(encoded), err
}
