/**
 * Metrics API client for the monitoring dashboard.
 *
 * Fetches data from the Query Service admin metrics endpoints:
 *   GET /api/admin/metrics/summary
 *   GET /api/admin/metrics/timeseries?metric=<metric>&range=<range>
 *   GET /api/cluster/members
 */

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    return process.env.NEXT_PUBLIC_API_URL || "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

export interface MetricsSummary {
  uptime_seconds: number;
  events_total: number;
  events_per_second: number;
  query_latency_p99_ms: number;
  error_rate_percent: number;
  active_tenants: number;
}

export interface TimeseriesPoint {
  timestamp: string;
  value: number;
}

export type TimeRange = "1h" | "24h" | "7d";

export type MetricName = "events_per_second" | "error_rate_percent";

export interface ClusterMember {
  id: string;
  role: "leader" | "follower";
  address: string;
  status: "healthy" | "degraded" | "unreachable";
  lag_ms?: number;
  uptime_seconds?: number;
}

export async function fetchMetricsSummary(): Promise<MetricsSummary> {
  const res = await fetch(`${getApiUrl()}/api/admin/metrics/summary`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch metrics summary: ${res.status}`);
  }
  return res.json();
}

export async function fetchTimeseries(
  metric: MetricName,
  range: TimeRange
): Promise<TimeseriesPoint[]> {
  const res = await fetch(
    `${getApiUrl()}/api/admin/metrics/timeseries?metric=${metric}&range=${range}`,
    { credentials: "include" }
  );
  if (!res.ok) {
    throw new Error(`Failed to fetch timeseries: ${res.status}`);
  }
  const data = await res.json();
  return data.points || data;
}

export async function fetchClusterMembers(): Promise<ClusterMember[]> {
  const res = await fetch(`${getApiUrl()}/api/cluster/members`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch cluster members: ${res.status}`);
  }
  const data = await res.json();
  return data.members || data;
}

/**
 * Format uptime seconds into a human-readable string.
 */
export function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}
