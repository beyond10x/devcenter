{{- define "devcenter.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "devcenter.fullname" -}}
{{- $name := include "devcenter.name" . -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name (include "devcenter.name" .) | trunc 63 | trimSuffix "-" }}
{{- end -}}
{{- end }}

{{- define "devcenter.workloadName" -}}
{{- include "devcenter.fullname" . -}}
{{- end }}

{{- define "devcenter.secretsWorkloadGrants" -}}
{{- $serviceAccount := default (printf "%s-connectors" (include "devcenter.fullname" .)) (index .Values.components "connectors").serviceAccountName -}}
{{- $subject := printf "system:serviceaccount:%s:%s" .Release.Namespace $serviceAccount -}}
{{- list (dict "subject" $subject "tenant" .Values.global.tenantId "actions" (list "secret:abort" "secret:commit" "secret:delete" "secret:list" "secret:prepare" "secret:read_metadata" "secret:read_value" "secret:write")) | toJson -}}
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
