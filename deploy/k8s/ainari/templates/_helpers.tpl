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
Service, which publishes the ports of a component on every node. The node-port is the port
itself plus a fixed offset, so the kind-setup can map it back to the original port on the host.
Every port is given as pair of the published port and the port of the pod, which is the
tls-termination in front of the component.
Usage: {{ include "ainari.nodePortService" (list $ "hanami" (list (list 11418 8443))) }}
*/}}
{{- define "ainari.nodePortService" -}}
{{- $root := index . 0 -}}
{{- $app := index . 1 -}}
{{- $ports := index . 2 -}}
{{- if $root.Values.global.node_ports.enabled }}
apiVersion: v1
kind: Service
metadata:
  name: {{ $app }}-node-port
  labels:
    app: {{ $app }}
spec:
  type: NodePort
  selector:
    app: {{ $app }}
  ports:
  {{- range $pair := $ports }}
  {{- $port := index $pair 0 }}
    - name: p-{{ $port }}
      protocol: TCP
      port: {{ $port }}
      targetPort: {{ index $pair 1 }}
      nodePort: {{ add $port $root.Values.global.node_ports.offset }}
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
