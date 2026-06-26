/**
 * analysis-api.ts — typed client for the read-only tenant-data analysis endpoint
 * GET /api/v1/admin/tenants/analyze (prompt 046, admin_analysis_dto.go).
 *
 * Mirrors the exact client pattern in tenants-api.ts / fleet-api.ts / comms-api.ts:
 *   - SAME-ORIGIN only: fetch("/api/v1/...", { credentials: "include" }) so the
 *     BFF (src/app/api/v1/[...path]/route.ts) attaches the admin_token Bearer and
 *     forwards to the Control Plane (Bearer-only via the BFF). Never a cross-origin
 *     credentials call to the CP (§6 rule 4).
 *   - list-returning fields resolve through asList() so a wrapped/null payload can
 *     never crash a `.map` (§6 rule 1).
 *   - the endpoint NEVER mutates; this client only READS. Every Finding carries a
 *     SuggestedAction that deep-links to an ALREADY-built guarded remediation — the
 *     UI renders those as links/instructions and never executes a mutation here.
 *
 * Graceful degradation: when the route 404s (e.g. 046 not yet deployed) the
 * fetchers throw a typed AnalysisUnavailableError the page catches to show an
 * "analysis unavailable" notice WITHOUT losing the instant client heuristics.
 */

import { asList } from "./tenants-api";

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    // Client-side: hit the same-origin BFF proxy (src/app/api/v1/[...path]/route.ts),
    // which attaches the admin_token Bearer and forwards to the Control Plane.
    return "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

// ── Contract (prompt 046 — admin_analysis_dto.go) ──────────────────────

/** The four analysis buckets — also the values of the ?category= query param. */
export type AnalysisCategory = "data_integrity" | "plan_billing" | "litter" | "usage_health";

/** Finding severities (worst→best). "ok" is only ever a tenant roll-up sentinel. */
export type AnalysisSeverity = "critical" | "warn" | "info" | "ok";

/** SuggestedAction kind: a deep-link route ("link") or an operator task ("task"). */
export type AnalysisActionKind = "link" | "task";

/**
 * The stable analysis codes (an API contract — analyze_tenants.go). The UI
 * switches on these to pick an icon/colour; unknown codes degrade to a default.
 */
export type AnalysisCode =
  // data_integrity
  | "count_cap_artifact"
  | "unmetered_or_empty"
  | "created_at_not_real"
  | "missing_required_fields"
  // plan_billing
  | "plan_shape_drift"
  | "plan_not_in_catalog"
  | "paid_plan_no_subscription"
  | "plan_distribution_suspicious"
  | "billing_config_broken"
  // litter
  | "demo_litter"
  | "trial_or_test_tenant"
  | "duplicate_name"
  | "empty_tenant"
  // usage_health
  | "quota_pressure"
  | "health_at_risk"
  | "stale_tenant"
  // meta
  | "analysis_degraded";

/**
 * Points the operator at an EXISTING guarded remediation. The analysis adds no
 * new mutation surface: `target` is always a route (link) or a task command that
 * already exists. The UI renders link→navigate / task→show-the-command; it never
 * auto-executes (verification #5).
 */
export interface SuggestedAction {
  label: string;
  kind: AnalysisActionKind;
  target: string;
}

/** One anomaly. Used for both per-tenant findings and (with affected_count) fleet findings. */
export interface AnalysisFinding {
  category: AnalysisCategory;
  severity: AnalysisSeverity;
  code: AnalysisCode | string;
  title: string;
  detail: string;
  affected_count?: number;
  suggested_action?: SuggestedAction;
}

/** Per-tenant slice of the report: identity columns + worst severity + findings. */
export interface AnalysisTenant {
  id: string;
  name: string;
  plan: string;
  status: string;
  event_count: number;
  member_count: number;
  created_at: string;
  worst_severity: AnalysisSeverity;
  findings: AnalysisFinding[];
}

/** Headline rollup rendered above the table. */
export interface AnalysisSummary {
  total_tenants: number;
  flagged_tenants: number;
  by_category: Partial<Record<AnalysisCategory, number>>;
  by_severity: Partial<Record<Exclude<AnalysisSeverity, "ok">, number>>;
}

/** The full GET /api/v1/admin/tenants/analyze response. */
export interface AnalysisReport {
  generated_at: string;
  summary: AnalysisSummary;
  fleet_findings: AnalysisFinding[];
  tenants: AnalysisTenant[];
}

// ── Errors ─────────────────────────────────────────────────────────────

/**
 * Thrown when the analyze endpoint is unreachable / not deployed (404/501/502/503)
 * so the page can distinguish "deep analysis unavailable, fall back to instant
 * heuristics" from a genuine error. Carries the HTTP status for messaging.
 */
export class AnalysisUnavailableError extends Error {
  readonly status: number;
  constructor(status: number) {
    super(
      status === 404
        ? "Deep analysis endpoint not available (the Control Plane /analyze route is not deployed)."
        : `Deep analysis endpoint unavailable (HTTP ${status}).`
    );
    this.name = "AnalysisUnavailableError";
    this.status = status;
  }
}

/** Statuses that mean "the feature isn't there", vs a real server error. */
function isUnavailable(status: number): boolean {
  return status === 404 || status === 405 || status === 501 || status === 502 || status === 503;
}

// ── Fetcher ────────────────────────────────────────────────────────────

/**
 * Fetch the analysis report, optionally scoped to one category. Each per-button
 * call passes a single category; "Analyze all" calls with no category.
 *
 * Resilience: list fields (`tenants`, `fleet_findings`) are array-guarded; a 404/
 * 5xx surfaces as AnalysisUnavailableError so the caller degrades gracefully.
 */
export async function fetchTenantAnalysis(category?: AnalysisCategory): Promise<AnalysisReport> {
  const qs = category ? `?category=${encodeURIComponent(category)}` : "";
  const url = `${getApiUrl()}/api/v1/admin/tenants/analyze${qs}`;

  let res: Response;
  try {
    res = await fetch(url, { credentials: "include" });
  } catch (_networkErr) {
    // A network failure (offline / DNS) is treated as unavailable, not a crash.
    throw new AnalysisUnavailableError(0);
  }

  if (!res.ok) {
    if (isUnavailable(res.status)) {
      throw new AnalysisUnavailableError(res.status);
    }
    throw new Error(`Failed to fetch tenant analysis: ${res.status}`);
  }

  const data = (await res.json().catch(() => ({}))) ?? {};
  const obj = (data && typeof data === "object" ? data : {}) as Partial<AnalysisReport>;

  // Always return well-formed, array-guarded data so the page never crashes on a
  // missing/odd field (§6 rules 1 & 3). Summary maps default to {}.
  return {
    generated_at: typeof obj.generated_at === "string" ? obj.generated_at : "",
    summary: {
      total_tenants: obj.summary?.total_tenants ?? 0,
      flagged_tenants: obj.summary?.flagged_tenants ?? 0,
      by_category: obj.summary?.by_category ?? {},
      by_severity: obj.summary?.by_severity ?? {},
    },
    fleet_findings: asList<AnalysisFinding>(obj, "fleet_findings"),
    tenants: asList<AnalysisTenant>(obj, "tenants").map((t) => ({
      ...t,
      findings: asList<AnalysisFinding>(t, "findings"),
    })),
  };
}

// ── Presentation helpers (display only) ────────────────────────────────

/** Human label for an analysis category. */
export function categoryLabel(category: AnalysisCategory): string {
  switch (category) {
    case "data_integrity":
      return "Data integrity";
    case "plan_billing":
      return "Plan & billing";
    case "litter":
      return "Litter";
    case "usage_health":
      return "Usage health";
    default:
      return category;
  }
}

/** Map a server severity to the shared dot/badge colour vocabulary. */
export function severityColour(severity: AnalysisSeverity): {
  dot: string;
  badge: "default" | "secondary" | "destructive" | "outline";
} {
  switch (severity) {
    case "critical":
      return { dot: "bg-red-500", badge: "destructive" };
    case "warn":
      return { dot: "bg-yellow-500", badge: "secondary" };
    case "info":
      return { dot: "bg-blue-500", badge: "outline" };
    default:
      return { dot: "bg-green-500", badge: "outline" };
  }
}

/** Human label for a severity. */
export function severityLabel(severity: AnalysisSeverity): string {
  switch (severity) {
    case "critical":
      return "Critical";
    case "warn":
      return "Warning";
    case "info":
      return "Info";
    default:
      return "OK";
  }
}

/** Worst→best rank for sorting/merging. */
export function severityRank(severity: AnalysisSeverity): number {
  switch (severity) {
    case "critical":
      return 3;
    case "warn":
      return 2;
    case "info":
      return 1;
    default:
      return 0;
  }
}
