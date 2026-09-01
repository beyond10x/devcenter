{{- define "devcenter.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "devcenter.fullname" -}}
{{- printf "%s-%s" .Release.Name (include "devcenter.name" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "devcenter.workloadName" -}}
{{- $name := include "devcenter.name" . -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- include "devcenter.fullname" . -}}
{{- end -}}
{{- end }}

{{- define "devcenter.labels" -}}
app.kubernetes.io/name: {{ include "devcenter.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | quote }}
{{- end }}

{{- define "devcenter.image" -}}
{{- if not .repository }}{{ fail "an enabled workload requires image.repository" }}{{ end -}}
{{- if not .digest }}{{ fail "an enabled workload requires image.digest" }}{{ end -}}
{{- printf "%s@%s" .repository .digest -}}
{{- end }}
