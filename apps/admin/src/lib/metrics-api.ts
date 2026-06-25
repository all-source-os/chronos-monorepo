/**
 * Metrics API client for the monitoring dashboard.
 *
 * Fetches from the Control Plane admin metrics passthrough (prompt 034),
 * SAME-ORIGIN via the BFF proxy (src/app/api/v1/[...path]/route.ts), which
 * attaches the admin_token Bearer and forwards to the CP:
 *   GET /api/v1/admin/metrics/summary
 *   GET /api/v1/admin/metrics/timeseries?metric=<metric>&range=<range>
 *   GET /api/v1/admin/cluster/members
 *
 * NOTE: the old client called the Query Service cross-origin
 * (`/api/admin/metrics/*`, `/api/cluster/members`) with `credentials:"include"`.
 * The CP is Bearer-only and ignores cookies, so that direct cross-origin call
 * never authenticated through the admin chain — the live `/monitoring` bug
 * (gap #2 / CONTROL_PLANE_CORS.md §4). Do NOT restore a cross-origin call here.
 */

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    // Client-side: hit the same-origin BFF proxy, which attaches the admin_token
    // Bearer and forwards to the Control Plane. Never a cross-origin CP/QS call.
    return "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

/**
 * Coerce a possibly-wrapped API response into an array (see security-api). A
 * non-array reaching a chart's `.map` / the cluster list crashes the whole
 * route ("x.map is not a function"). Always resolve to an array.
 */
function asList<T>(data: unknown, ...keys: string[]): T[] {
  if (Array.isArray(data)) return data as T[];
  if (data && typeof data === "object") {
    for (const k of [...keys, "items", "data"]) {
      const v = (data as Record<string, unknown>)[k];
      if (Array.isArray(v)) return v as T[];
    }
  }
  return [];
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
  const res = await fetch(`${getApiUrl()}/api/v1/admin/metrics/summary`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch metrics summary: ${res.status}`);
  }
  const data = (await res.json()) ?? {};
  // Guard every numeric field with `?? 0` so a missing field never reaches a
  // `.toLocaleString()`/`.toFixed()` on undefined in the stat cards (§6).
  return {
    uptime_seconds: data.uptime_seconds ?? 0,
    events_total: data.events_total ?? 0,
    events_per_second: data.events_per_second ?? 0,
    query_latency_p99_ms: data.query_latency_p99_ms ?? 0,
    error_rate_percent: data.error_rate_percent ?? 0,
    active_tenants: data.active_tenants ?? 0,
  };
}

export async function fetchTimeseries(
  metric: MetricName,
  range: TimeRange
): Promise<TimeseriesPoint[]> {
  const res = await fetch(
    `${getApiUrl()}/api/v1/admin/metrics/timeseries?metric=${metric}&range=${range}`,
    { credentials: "include" }
  );
  if (!res.ok) {
    throw new Error(`Failed to fetch timeseries: ${res.status}`);
  }
  const data = await res.json();
  // CP shape: { metric, range, points: [{ timestamp, value }] }
  return asList<TimeseriesPoint>(data, "points").map((p) => ({
    timestamp: p?.timestamp ?? "",
    value: p?.value ?? 0,
  }));
}

export async function fetchClusterMembers(): Promise<ClusterMember[]> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/cluster/members`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch cluster members: ${res.status}`);
  }
  const data = await res.json();
  // CP passthrough shape: { members: [{ id, role, address, status, lag_ms?, uptime_seconds? }] }
  return asList<ClusterMember>(data, "members");
}

/**
 * Format uptime seconds into a human-readable string. Guards against an
 * undefined/null input (`?? 0`) so a missing summary field can't crash render.
 */
export function formatUptime(seconds: number): string {
  const s = seconds ?? 0;
  const days = Math.floor(s / 86400);
  const hours = Math.floor((s % 86400) / 3600);
  const minutes = Math.floor((s % 3600) / 60);

  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}
