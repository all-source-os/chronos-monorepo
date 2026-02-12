{{/*
Expand the name of the chart.
*/}}
{{- define "allsource.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "allsource.fullname" -}}
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
{{- define "allsource.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "allsource.labels" -}}
helm.sh/chart: {{ include "allsource.chart" . }}
{{ include "allsource.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "allsource.selectorLabels" -}}
app.kubernetes.io/name: {{ include "allsource.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Core specific labels
*/}}
{{- define "allsource.core.labels" -}}
{{ include "allsource.labels" . }}
app.kubernetes.io/component: core
{{- end }}

{{- define "allsource.core.selectorLabels" -}}
{{ include "allsource.selectorLabels" . }}
app.kubernetes.io/component: core
{{- end }}

{{/*
Query Service specific labels
*/}}
{{- define "allsource.queryService.labels" -}}
{{ include "allsource.labels" . }}
app.kubernetes.io/component: query-service
{{- end }}

{{- define "allsource.queryService.selectorLabels" -}}
{{ include "allsource.selectorLabels" . }}
app.kubernetes.io/component: query-service
{{- end }}

{{/*
Core fullname
*/}}
{{- define "allsource.core.fullname" -}}
{{- printf "%s-core" (include "allsource.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Query Service fullname
*/}}
{{- define "allsource.queryService.fullname" -}}
{{- printf "%s-query-service" (include "allsource.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}
