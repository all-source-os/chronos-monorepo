/**
 * Suspicious Activity API client.
 *
 * Endpoint on the Control Plane:
 *   GET /api/v1/admin/security/suspicious-activity
 */

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    // Client-side: hit the same-origin BFF proxy (src/app/api/v1/[...path]/route.ts),
    // which attaches the admin_token Bearer and forwards to the Control Plane.
    return "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type AlertSeverity = "high" | "medium" | "low";

export type SuspiciousActivityType =
  | "excessive_failed_auth"
  | "unusual_query_pattern"
  | "rate_limit_exceeded"
  | "token_abuse"
  | "geo_anomaly";

export interface SuspiciousActivityAlert {
  id: string;
  type: SuspiciousActivityType;
  tenant_id: string;
  tenant_name: string;
  description: string;
  severity: AlertSeverity;
  timestamp: string;
  details: Record<string, unknown>;
}

export interface SuspiciousActivityResponse {
  alerts: SuspiciousActivityAlert[];
  total: number;
}

// ---------------------------------------------------------------------------
// API calls
// ---------------------------------------------------------------------------

export async function fetchSuspiciousActivity(params?: {
  severity?: AlertSeverity;
  type?: SuspiciousActivityType;
}): Promise<SuspiciousActivityResponse> {
  const searchParams = new URLSearchParams();
  if (params?.severity) searchParams.set("severity", params.severity);
  if (params?.type) searchParams.set("type", params.type);

  const qs = searchParams.toString();
  const url = `${getApiUrl()}/api/v1/admin/security/suspicious-activity${qs ? `?${qs}` : ""}`;

  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to fetch suspicious activity: ${res.status}`);
  }
  return res.json();
}

export async function fetchHighSeverityCount(): Promise<number> {
  const data = await fetchSuspiciousActivity({ severity: "high" });
  return data.total;
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

export const ACTIVITY_TYPE_LABELS: Record<SuspiciousActivityType, string> = {
  excessive_failed_auth: "Excessive Failed Auth",
  unusual_query_pattern: "Unusual Query Pattern",
  rate_limit_exceeded: "Rate Limit Exceeded",
  token_abuse: "Token Abuse",
  geo_anomaly: "Geographic Anomaly",
};

export const SEVERITY_CONFIG: Record<
  AlertSeverity,
  { label: string; className: string; dotClassName: string }
> = {
  high: {
    label: "High",
    className: "border-red-500/30 bg-red-500/5 text-red-700 dark:text-red-400",
    dotClassName: "bg-red-500",
  },
  medium: {
    label: "Medium",
    className:
      "border-yellow-500/30 bg-yellow-500/5 text-yellow-700 dark:text-yellow-400",
    dotClassName: "bg-yellow-500",
  },
  low: {
    label: "Low",
    className:
      "border-blue-500/30 bg-blue-500/5 text-blue-700 dark:text-blue-400",
    dotClassName: "bg-blue-500",
  },
};
