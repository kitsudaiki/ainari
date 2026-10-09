#!/bin/bash
set -e

# escapes a value for a string-literal of mysql, where backslash and quote are special
sql_escape() {
    local value="${1//\\/\\\\}"
    echo "${value//\'/\\\'}"
}
{{- range .Spec.MySQL.Databases.ByComponent }}

mysql --protocol=socket -u root -p"${MYSQL_ROOT_PASSWORD}" <<EOSQL
CREATE DATABASE IF NOT EXISTS {{ .Database }};
CREATE USER IF NOT EXISTS '{{ .User }}'@'%' IDENTIFIED BY '$(sql_escape "${{ upper .Component }}_PASSWORD")';
GRANT ALL PRIVILEGES ON {{ .Database }}.* TO '{{ .User }}'@'%';
EOSQL
{{- end }}
