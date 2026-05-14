{{/*
Common labels applied to every resource.
*/}}
{{- define "tyche.labels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: tyche
tyche.network/env: {{ .Values.global.environment }}
tyche.network/tenant: {{ .Values.global.tenant }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Per-service selector labels. `component` is the differentiator and must be
unique across api / web / batcher.
*/}}
{{- define "tyche.selectorLabels" -}}
app.kubernetes.io/name: {{ .root.Chart.Name }}
app.kubernetes.io/instance: {{ .root.Release.Name }}
tyche.network/component: {{ .component }}
{{- end }}

{{/*
Image reference helper. Honours per-service repo override and global registry.
Pinning by digest is preferred; we accept tags during the spike.
*/}}
{{- define "tyche.image" -}}
{{- $registry := .root.Values.global.imageRegistry -}}
{{- printf "%s/%s:%s" $registry .service.image.repository .service.image.tag -}}
{{- end }}

{{/*
Service-account name with optional ESO/IRSA annotations.
*/}}
{{- define "tyche.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{ .Release.Name }}-sa
{{- else -}}
default
{{- end -}}
{{- end }}
