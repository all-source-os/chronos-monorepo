/**
 * Alerts & SLO API client.
 *
 * Endpoints on the Control Plane:
 *   POST/GET/PUT/DELETE /api/v1/admin/alerts
 *   POST/GET/DELETE     /api/v1/admin/slos
 */

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    return process.env.NEXT_PUBLIC_API_URL || "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

// ---------------------------------------------------------------------------
// Alert types
// ---------------------------------------------------------------------------

export type AlertOperator = "gt" | "gte" | "lt" | "lte" | "eq";
export type AlertChannel = "email" | "slack";
export type AlertMetric =
  | "events_per_second"
  | "error_rate_percent"
  | "query_latency_p99_ms"
  | "cpu_usage_percent"
  | "memory_usage_percent";

export interface AlertRule {
  id: string;
  name: string;
  metric: AlertMetric;
  operator: AlertOperator;
  threshold: number;
  duration_seconds: number;
  channel: AlertChannel;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateAlertPayload {
  name: string;
  metric: AlertMetric;
  operator: AlertOperator;
  threshold: number;
  duration_seconds: number;
  channel: AlertChannel;
}

export type UpdateAlertPayload = Partial<CreateAlertPayload> & {
  enabled?: boolean;
};

// ---------------------------------------------------------------------------
// SLO types
// ---------------------------------------------------------------------------

export interface SLO {
  id: string;
  name: string;
  metric: string;
  target_percent: number;
  window_days: number;
  current_compliance: number;
  error_budget_remaining: number;
  trend: number[]; // last N data points for sparkline
  created_at: string;
}

export interface CreateSLOPayload {
  name: string;
  metric: string;
  target_percent: number;
  window_days: number;
}

// ---------------------------------------------------------------------------
// Alert CRUD
// ---------------------------------------------------------------------------

export async function fetchAlerts(): Promise<AlertRule[]> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/alerts`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch alerts: ${res.status}`);
  }
  const data = await res.json();
  return data.alerts || data;
}

export async function createAlert(
  payload: CreateAlertPayload
): Promise<AlertRule> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/alerts`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    throw new Error(`Failed to create alert: ${res.status}`);
  }
  return res.json();
}

export async function updateAlert(
  id: string,
  payload: UpdateAlertPayload
): Promise<AlertRule> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/alerts/${id}`, {
    method: "PUT",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    throw new Error(`Failed to update alert: ${res.status}`);
  }
  return res.json();
}

export async function deleteAlert(id: string): Promise<void> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/alerts/${id}`, {
    method: "DELETE",
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to delete alert: ${res.status}`);
  }
}

// ---------------------------------------------------------------------------
// SLO CRUD
// ---------------------------------------------------------------------------

export async function fetchSLOs(): Promise<SLO[]> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/slos`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch SLOs: ${res.status}`);
  }
  const data = await res.json();
  return data.slos || data;
}

export async function createSLO(payload: CreateSLOPayload): Promise<SLO> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/slos`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    throw new Error(`Failed to create SLO: ${res.status}`);
  }
  return res.json();
}

export async function deleteSLO(id: string): Promise<void> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/slos/${id}`, {
    method: "DELETE",
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to delete SLO: ${res.status}`);
  }
}

// ---------------------------------------------------------------------------
// Constants for form dropdowns
// ---------------------------------------------------------------------------

export const ALERT_METRICS = [
  { value: "events_per_second", label: "Events/sec" },
  { value: "error_rate_percent", label: "Error Rate (%)" },
  { value: "query_latency_p99_ms", label: "Latency p99 (ms)" },
  { value: "cpu_usage_percent", label: "CPU Usage (%)" },
  { value: "memory_usage_percent", label: "Memory Usage (%)" },
] as const;

export const ALERT_OPERATORS: { value: AlertOperator; label: string }[] = [
  { value: "gt", label: "> (greater than)" },
  { value: "gte", label: ">= (greater or equal)" },
  { value: "lt", label: "< (less than)" },
  { value: "lte", label: "<= (less or equal)" },
  { value: "eq", label: "= (equal)" },
];

export const OPERATOR_SYMBOLS: Record<AlertOperator, string> = {
  gt: ">",
  gte: ">=",
  lt: "<",
  lte: "<=",
  eq: "=",
};
