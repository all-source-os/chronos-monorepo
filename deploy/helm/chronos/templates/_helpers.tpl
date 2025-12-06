{{/*
Expand the name of the chart.
*/}}
{{- define "chronos.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "chronos.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "chronos.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "chronos.labels" -}}
helm.sh/chart: {{ include "chronos.chart" . }}
{{ include "chronos.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "chronos.selectorLabels" -}}
app.kubernetes.io/name: {{ include "chronos.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Core specific labels
*/}}
{{- define "chronos.core.labels" -}}
{{ include "chronos.labels" . }}
app.kubernetes.io/component: core
{{- end }}

{{- define "chronos.core.selectorLabels" -}}
{{ include "chronos.selectorLabels" . }}
app.kubernetes.io/component: core
{{- end }}

{{/*
Query Service specific labels
*/}}
{{- define "chronos.queryService.labels" -}}
{{ include "chronos.labels" . }}
app.kubernetes.io/component: query-service
{{- end }}

{{- define "chronos.queryService.selectorLabels" -}}
{{ include "chronos.selectorLabels" . }}
app.kubernetes.io/component: query-service
{{- end }}

{{/*
Core fullname
*/}}
{{- define "chronos.core.fullname" -}}
{{- printf "%s-core" (include "chronos.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Query Service fullname
*/}}
{{- define "chronos.queryService.fullname" -}}
{{- printf "%s-query-service" (include "chronos.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}
