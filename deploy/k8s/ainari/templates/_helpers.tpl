{{/*
Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

Licensed under the Apache License, Version 2.0 (the "License")
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/}}

{{/*
Image of a component.
Usage: {{ include "ainari.image" .Values.miko }}
*/}}
{{- define "ainari.image" -}}
{{ .docker.repository }}:{{ .docker.tag }}
{{- end }}

{{/*
Pull-policy of a component. The global value overwrites the one of the component, so a local
setup can switch all of them at once.
Usage: {{ include "ainari.pullPolicy" (list $ .Values.miko) }}
*/}}
{{- define "ainari.pullPolicy" -}}
{{- $root := index . 0 -}}
{{- $component := index . 1 -}}
{{ default $component.docker.pull_policy $root.Values.global.image.pull_policy }}
{{- end }}

{{/*
Storage-class of a component, which is again overwritten by the global value.
Usage: {{ include "ainari.storageClass" (list $ .Values.miko) }}
*/}}
{{- define "ainari.storageClass" -}}
{{- $root := index . 0 -}}
{{- $component := index . 1 -}}
{{ default $component.storage.storage_class $root.Values.global.storage_class }}
{{- end }}

{{/*
Address, which the other components use to reach miko.
*/}}
{{- define "ainari.mikoAddress" -}}
https://miko-tls-service.{{ .Release.Namespace }}.svc.cluster.local:8443
{{- end }}

{{/*
Public address of a component, which miko hands out to the clients. An explicitly configured
address wins, otherwise it is the domain of the ingress.
Usage: {{ include "ainari.publicAddress" (list $ "hanami") }}
*/}}
{{- define "ainari.publicAddress" -}}
{{- $root := index . 0 -}}
{{- $component := index $root.Values (index . 1) -}}
{{- if $component.api.public_address -}}
{{ $component.api.public_address }}
{{- else -}}
https://{{ $component.api.domain }}:443
{{- end -}}
{{- end }}

{{/*
Internal address of a component, which miko hands out to the other components. It is the
tls-termination in front of the component.
Usage: {{ include "ainari.internalAddress" (list $ "hanami") }}
*/}}
{{- define "ainari.internalAddress" -}}
https://{{ index . 1 }}-tls-service.{{ (index . 0).Release.Namespace }}.svc.cluster.local:8443
{{- end }}

{{/*
Affinity of a component. On a real cluster every component runs on the nodes, which carry its
label, and never twice on the same node. A local cluster has only one node, so both rules are
switched off there.
Usage: {{ include "ainari.affinity" (list $ "hanami" "hanami-node") }}
*/}}
{{- define "ainari.affinity" -}}
{{- $root := index . 0 -}}
{{- $app := index . 1 -}}
{{- $nodeLabel := index . 2 -}}
{{- if $root.Values.global.strict_scheduling }}
affinity:
  nodeAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      nodeSelectorTerms:
      - matchExpressions:
        - key: {{ $nodeLabel }}
          operator: In
          values:
          - "true"
  podAntiAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
    - labelSelector:
        matchExpressions:
        - key: app
          operator: In
          values:
          - {{ $app }}
      topologyKey: kubernetes.io/hostname
{{- end }}
{{- end }}

{{/*
Service of the external api of a component. It always exists, because the ingress forwards to it
as well, but it only publishes the ports outside of the cluster, if global.external_services is
enabled, otherwise it has the type ClusterIP. With the type NodePort, the node-port is the port itself plus a fixed
offset, so the kind-setup can map it back to the original port on the host. With the type
LoadBalancer, the port itself is published, for example by the servicelb of k3s on every node.
Every port is given as pair of the published port and the port of the pod, which is the
tls-termination in front of the component.
Usage: {{ include "ainari.externalService" (list $ "hanami" (list (list 11418 8443))) }}
*/}}
{{- define "ainari.externalService" -}}
{{- $root := index . 0 -}}
{{- $app := index . 1 -}}
{{- $ports := index . 2 -}}
{{- $external := $root.Values.global.external_services -}}
{{- $type := ternary $external.type "ClusterIP" $external.enabled -}}
apiVersion: v1
kind: Service
metadata:
  name: {{ $app }}-external
  labels:
    app: {{ $app }}
spec:
  type: {{ $type }}
  selector:
    app: {{ $app }}
  ports:
  {{- range $pair := $ports }}
  {{- $port := index $pair 0 }}
    - name: p-{{ $port }}
      protocol: TCP
      port: {{ $port }}
      targetPort: {{ index $pair 1 }}
      {{- if eq $type "NodePort" }}
      nodePort: {{ add $port $external.node_port_offset }}
      {{- end }}
  {{- end }}
{{- end }}

{{/*
Certificate of the tls-termination of a component, signed by the CA of the chart. It is valid for
the service of the component within the cluster, the domain of the ingress and the names and
addresses of global.certificates, like 127.0.0.1 in the kind-setup.
Usage: {{ include "ainari.certificate" (list $ "hanami" (list "hanami-tls-service")) }}
The last argument is the list of names within the namespace, which are completed into full names
like 'hanami-tls-service.<namespace>.svc.cluster.local'.
*/}}
{{- define "ainari.certificate" -}}
{{- $root := index . 0 -}}
{{- $name := index . 1 -}}
{{- $component := index $root.Values $name -}}
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: {{ $name }}-cert
spec:
  secretName: {{ $name }}-tls-secret
  issuerRef:
    name: ainari-ca-issuer
  dnsNames:
  {{- range $host := index . 2 }}
    - {{ printf "%s.%s.svc.cluster.local" $host $root.Release.Namespace | quote }}
  {{- end }}
  {{- if $root.Values.global.ingress.enabled }}
    - {{ $component.api.domain | quote }}
  {{- end }}
  {{- range $root.Values.global.certificates.extra_dns_names }}
    - {{ . | quote }}
  {{- end }}
  {{- with $root.Values.global.certificates.extra_ip_addresses }}
  ipAddresses:
  {{- range . }}
    - {{ . | quote }}
  {{- end }}
  {{- end }}
{{- end }}

{{/*
Address of the mysql-server, which is either the service of the deployed server or the configured
external one.
*/}}
{{- define "ainari.mysqlHost" -}}
{{- if .Values.mysql.deploy -}}
mysql.{{ .Release.Namespace }}.svc.cluster.local
{{- else -}}
{{ required "mysql.host is required, if mysql.deploy is disabled!" .Values.mysql.host }}
{{- end -}}
{{- end }}

{{/*
Database, user and password of a component on the mysql-server. The names are used within the
init-script of the server, so they are restricted to letters, digits and underscores.
Usage: {{ $db := include "ainari.mysqlDatabase" (list $ "hanami") | fromYaml }}
*/}}
{{- define "ainari.mysqlDatabase" -}}
{{- $root := index . 0 -}}
{{- $name := index . 1 -}}
{{- $db := index $root.Values.mysql.databases $name -}}
{{- range $key := list "database" "user" -}}
{{- if not (regexMatch "^[a-zA-Z0-9_]+$" (index $db $key | toString)) -}}
{{- fail (printf "mysql.databases.%s.%s may only contain letters, digits and underscores!" $name $key) -}}
{{- end -}}
{{- end -}}
{{- $_ := required (printf "mysql.databases.%s.password is required!" $name) $db.password -}}
{{ toYaml $db }}
{{- end }}

{{/*
Config-group of the mysql-database of a component. The value 'database_type = "mysql"' has to be
set separately in the root of the config.
Usage: {{ include "ainari.mysqlConfig" (list $ "hanami") | indent 4 | trim }}
*/}}
{{- define "ainari.mysqlConfig" -}}
{{- $root := index . 0 -}}
{{- $db := include "ainari.mysqlDatabase" . | fromYaml -}}
# the password is read from the env-variable AINARI_MYSQL_PASSWORD
[mysql]
host = "{{ include "ainari.mysqlHost" $root }}"
port = {{ $root.Values.mysql.port }}
user = "{{ $db.user }}"
database = "{{ $db.database }}"
{{- end }}

{{/*
Env-variable with the password of the mysql-database of a component.
Usage: {{ include "ainari.mysqlPasswordEnv" "hanami" | nindent 8 }}
*/}}
{{- define "ainari.mysqlPasswordEnv" -}}
- name: AINARI_MYSQL_PASSWORD
  valueFrom:
    secretKeyRef:
      name: mysql-credentials
      key: {{ . }}_password
{{- end }}

{{/*
Init-container, which waits until the mysql-server accepts connections. The components exit at
their start, if they can't reach their database, so without it, they would be restarted with an
increasing delay, while the server is still initializing. mysqladmin succeeds, as soon as the
server answers, even if it rejects the login.
Usage: {{ include "ainari.waitForMysql" . | nindent 6 }}
*/}}
{{- define "ainari.waitForMysql" -}}
initContainers:
- name: wait-for-mysql
  image: {{ include "ainari.image" .Values.mysql }}
  imagePullPolicy: {{ include "ainari.pullPolicy" (list . .Values.mysql) }}
  command:
  - sh
  - -c
  - |
    until mysqladmin ping --host={{ include "ainari.mysqlHost" . }} --port={{ .Values.mysql.port }} --connect-timeout=2 > /dev/null 2>&1; do
      echo "waiting for the mysql-server ..."
      sleep 2
    done
{{- end }}

{{/*
Init-container, which waits until miko answers. Onsen and sakura request the endpoints of the
other components from miko at their start and exit, if it is not reachable yet, so without it
they would be restarted with an increasing delay, while miko is still starting. The certificate is
not verified, because the check sends no data and only waits for an answer.
Usage: {{ include "ainari.waitForMiko" . | nindent 6 }}
*/}}
{{- define "ainari.waitForMiko" -}}
initContainers:
- name: wait-for-miko
  image: nginx:latest
  command:
  - sh
  - -c
  - |
    until curl --silent --fail --insecure --max-time 3 {{ include "ainari.mikoAddress" . }}/v1alpha/is_ready > /dev/null; do
      echo "waiting for miko ..."
      sleep 2
    done
{{- end }}

{{/*
Config of a nginx-sidecar, which terminates tls and forwards the requests to a port of the
component within the pod.
Usage: {{ include "ainari.nginxConfig" (list 8443 10417) | nindent 4 }}
The first argument is the port, where nginx listens, the second one the port of the component.
*/}}
{{- define "ainari.nginxConfig" -}}
{{- $listen := index . 0 -}}
{{- $upstream := index . 1 -}}
user nginx;
worker_processes  2;
error_log  /dev/stdout;
events {
    worker_connections  1024;
}
http {
    server
    {
        server_tokens off;

        listen {{ $listen }} ssl;
        listen [::]:{{ $listen }} ssl;
        http2  on;

        ssl_certificate /etc/nginx/certs/tls.crt;
        ssl_certificate_key /etc/nginx/certs/tls.key;
        ssl_session_timeout 1d;
        ssl_session_cache shared:SSL:50m;
        ssl_session_tickets off;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384;
        ssl_prefer_server_ciphers on;

        # HSTS (15768000 seconds = 6 months)
        add_header Strict-Transport-Security max-age=15768000;

        client_max_body_size 1G;

        access_log /dev/stdout;
        error_log  /dev/stdout;

        location / {
            proxy_cache         off;

            proxy_set_header    Upgrade $http_upgrade;
            proxy_set_header    Connection "Upgrade";
            proxy_http_version  1.1;
            # large uploads, like the images of ryokan, are streamed to the component instead of
            # being buffered in the sidecar, whose memory is limited
            proxy_request_buffering off;
            proxy_set_header    Host                $http_host;
            proxy_set_header    X-Real-IP           $remote_addr;
            proxy_set_header    X-Forwarded-For     $proxy_add_x_forwarded_for;

            proxy_pass http://127.0.0.1:{{ $upstream }}/;
            proxy_read_timeout 180;
        }
    }
}
{{- end }}

{{/*
ConfigMap with the configs of the two nginx-sidecars of a component, which has an internal and an
external api. The internal one listens on 8443 for '<component>-tls-service' and forwards to the
internal port of the component, the external one listens on 9443 for '<component>-external' and
the ingress and forwards to the public port of the component.
Usage: {{ include "ainari.nginxConfigMap" (list "miko" 10417 11417) }}
*/}}
{{- define "ainari.nginxConfigMap" -}}
{{- $name := index . 0 -}}
apiVersion: v1
kind: ConfigMap
metadata:
  name: {{ $name }}-nginx-config
data:
  # tls-termination of the internal api, which the other components reach over {{ $name }}-tls-service
  internal.conf: |+
    {{- include "ainari.nginxConfig" (list 8443 (index . 1)) | nindent 4 }}
  # tls-termination of the external api, which the clients reach over {{ $name }}-external and the
  # ingress
  external.conf: |+
    {{- include "ainari.nginxConfig" (list 9443 (index . 2)) | nindent 4 }}
{{- end }}

{{/*
The two nginx-sidecars of a component with an internal and an external api, see
ainari.nginxConfigMap.
Usage: {{ include "ainari.tlsSidecars" "miko" | nindent 6 }}
*/}}
{{- define "ainari.tlsSidecars" -}}
{{- range $sidecar := list (list "tls-termination" "internal.conf") (list "external-tls-termination" "external.conf") }}
- name: {{ index $sidecar 0 }}
  image: nginx:latest
  volumeMounts:
  - name: tls-certs
    mountPath: /etc/nginx/certs
  - name: {{ $ }}-nginx-config
    mountPath: /etc/nginx/nginx.conf
    subPath: {{ index $sidecar 1 }}
  resources:
    # Without requests, kubernetes reserves the limits for the sidecar, which would block
    # half a cpu per sidecar, although the tls-termination needs only a fraction of it.
    requests:
      memory: "32Mi"
      cpu: "50m"
    limits:
      memory: "128Mi"
      cpu: "500m"
{{- end }}
{{- end }}

{{/*
Public key of hanami, which signs the membership-grants of the MLS-groups. Izakaya and the
gateways in front of the sakura-hosts check the grants with it.
Usage: {{ include "ainari.mlsGrantPublicKey" . }}
*/}}
{{- define "ainari.mlsGrantPublicKey" -}}
{{ required "secrets.mls_grant_public_key is required, if hanami.network.mls_encryption is enabled!" .Values.secrets.mls_grant_public_key }}
{{- end }}
