/**
 * tenant-anomalies.ts — pure, client-side anomaly heuristics for the admin
 * /tenants page (ADMIN_TENANT_POWER_TOOL §3 gaps + §6 resilience).
 *
 * WHY this exists: whole columns in the tenant list turned out NOT to be backed
 * by real data — `created_at` was `time.Now()` on load (ADMIN_HEALTH §4), and
 * `event_count` showed a backfill-capped `1,000,000` (ADMIN_HEALTH "canonical
 * event count"). A human cannot eyeball real-vs-defaulted across 40+ rows. These
 * cheap heuristics run over the ALREADY-fetched tenant list (zero backend calls)
 * so the operator gets immediate signal, and they keep working even when the deep
 * /analyze endpoint (prompt 046) is down.
 *
 * These functions are PURE (no fetch, no React, no Date.now side effects beyond
 * the explicit `now` arg) so they are trivially unit-testable. They MIRROR the
 * runbook definitions:
 *   - cap artifact         == event_count exactly 1_000_000   (ADMIN_HEALTH §"What number")
 *   - fake created_at      == ≥8 tenants share ONE calendar date (ADMIN_HEALTH §4 / gate)
 *   - plan shape/validity  == planLabel(plan) not a canonical tier, or a non-string plan repr
 *   - litter               == trial/demo/smoke/long-suffix names, or a fully-empty tenant
 *
 * The `code` strings returned here are a SUBSET of the prompt-046 contract codes,
 * so a client-side finding and a server-side finding for the same anomaly use the
 * SAME code (count_cap_artifact, created_at_not_real, plan_shape_drift,
 * plan_not_in_catalog, demo_litter, trial_or_test_tenant, empty_tenant). That lets
 * the page merge the two sources without translation.
 */

import { type PlanLike, planLabel, type Tenant } from "./tenants-api";

// ── Severity (mirrors the 046 worst→best ordering) ─────────────────────

export type AnomalySeverity = "critical" | "warn" | "info";

const SEVERITY_RANK: Record<AnomalySeverity, number> = {
  critical: 3,
  warn: 2,
  info: 1,
};

/** Return the worse of two severities (higher rank wins). */
export function worseSeverity(a: AnomalySeverity, b: AnomalySeverity): AnomalySeverity {
  return SEVERITY_RANK[a] >= SEVERITY_RANK[b] ? a : b;
}

/** The worst severity across a set of findings, or null when there are none. */
export function worstSeverityOf(findings: { severity: AnomalySeverity }[]): AnomalySeverity | null {
  let worst: AnomalySeverity | null = null;
  for (const f of findings) {
    worst = worst == null ? f.severity : worseSeverity(worst, f.severity);
  }
  return worst;
}

// ── Finding shape ──────────────────────────────────────────────────────

/**
 * Stable client-side codes. Each is also a prompt-046 server code, so the page
 * can merge client + server findings under one code namespace.
 */
export type AnomalyCode =
  | "count_cap_artifact"
  | "created_at_not_real"
  | "plan_shape_drift"
  | "plan_not_in_catalog"
  | "demo_litter"
  | "trial_or_test_tenant"
  | "empty_tenant";

export type AnomalyCategory = "data_integrity" | "plan_billing" | "litter";

/** One client-side anomaly for a single tenant. */
export interface ClientFinding {
  code: AnomalyCode;
  category: AnomalyCategory;
  severity: AnomalySeverity;
  title: string;
  detail: string;
}

// ── Constants (mirror the runbook) ─────────────────────────────────────

/** The backfill cap (`defaultBackfillMaxPages × queryPageLimit`, ADMIN_HEALTH). */
export const EVENT_COUNT_CAP = 1_000_000;

/** ≥ this many tenants sharing ONE created_at date trips the fake-date banner. */
export const SAME_DATE_THRESHOLD = 8;

/**
 * Canonical billing tiers (Control Plane authority — subscription.go; the same
 * list `TenantPlan` enumerates). A `planLabel(plan)` outside this set is drift.
 * Retired aliases (starter/pro/growth/team) and the web "self-host" label are
 * intentionally NOT canonical, so a tenant stamped with one is flagged.
 */
export const CANONICAL_TIERS = new Set(["free", "indie", "studio", "scale", "enterprise"]);

/**
 * Litter name patterns (ADMIN_TENANT_POWER_TOOL §3 gap #1 demo litter +
 * the prompt's litter heuristics):
 *   ^anonymous-trial… | ^demo… | …smoke… | a trailing 10+ digit epoch suffix.
 * Matched case-insensitively against the tenant name.
 */
const LITTER_NAME_RE = /^anonymous-trial|^demo|smoke|-\d{10,}$/i;

// ── Heuristics (pure) ──────────────────────────────────────────────────

/**
 * The local calendar date (YYYY-MM-DD) of an ISO timestamp, or null when the
 * value is missing/unparseable. Pure given a fixed timezone; we use UTC slicing
 * of the ISO string so the bucketing is timezone-stable across environments.
 */
export function calendarDate(iso: string | undefined | null): string | null {
  if (!iso || typeof iso !== "string") return null;
  // Fast path: an RFC3339 string starts with YYYY-MM-DD; slice avoids Date TZ skew.
  const m = iso.match(/^(\d{4}-\d{2}-\d{2})/);
  if (m?.[1]) return m[1];
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return null;
  return d.toISOString().slice(0, 10);
}

/** True when this tenant's event_count is exactly the backfill cap artifact. */
export function isCapArtifact(tenant: Pick<Tenant, "event_count">): boolean {
  return tenant.event_count === EVENT_COUNT_CAP;
}

/**
 * True when the plan is a non-string representation (object/array) — rendering it
 * raw crashed the 360 (React #31). planLabel() is what makes it safe; this just
 * flags that the shape itself is unexpected.
 */
export function isPlanShapeDrift(plan: PlanLike): boolean {
  return plan != null && typeof plan !== "string";
}

/**
 * True when the (labelled) plan is not one of the canonical tiers — a retired
 * alias, a typo, or an empty string. `null`/`undefined` plans are NOT flagged
 * here (they render as "—" and are a legitimate "no plan" state).
 */
export function isPlanNotInCatalog(plan: PlanLike): boolean {
  if (plan == null) return false;
  const label = planLabel(plan).trim().toLowerCase();
  if (label === "" || label === "—") return false;
  return !CANONICAL_TIERS.has(label);
}

/** True when the tenant name matches a litter pattern (trial/demo/smoke/epoch). */
export function isLitterName(name: string | undefined | null): boolean {
  if (!name || typeof name !== "string") return false;
  return LITTER_NAME_RE.test(name.trim());
}

/** True when the tenant has zero events AND zero members (a fully-empty shell). */
export function isEmptyTenant(tenant: Pick<Tenant, "event_count" | "member_count">): boolean {
  return (tenant.event_count ?? 0) === 0 && (tenant.member_count ?? 0) === 0;
}

/**
 * All client-side findings for ONE tenant. Order is worst→least so the first
 * finding's severity is the row's worst severity.
 */
export function analyzeTenant(tenant: Tenant): ClientFinding[] {
  const findings: ClientFinding[] = [];

  if (isCapArtifact(tenant)) {
    findings.push({
      code: "count_cap_artifact",
      category: "data_integrity",
      severity: "warn",
      title: "Event count is the backfill cap (1,000,000)",
      detail:
        "event_count is exactly 1,000,000 — the backfill page cap, an honest “≥1M”, not a real count. The 5-min reconciler supersedes it; reconcile to get the true store count.",
    });
  }

  if (isPlanShapeDrift(tenant.plan)) {
    findings.push({
      code: "plan_shape_drift",
      category: "plan_billing",
      severity: "warn",
      title: "Plan is not a plain string",
      detail:
        "The plan is an object/array, not a tier string. It renders safely via planLabel() but the shape is unexpected — the raw object crashed the 360 page once (React #31).",
    });
  } else if (isPlanNotInCatalog(tenant.plan)) {
    findings.push({
      code: "plan_not_in_catalog",
      category: "plan_billing",
      severity: "warn",
      title: `Plan “${planLabel(tenant.plan)}” is not a canonical tier`,
      detail:
        "The plan is not one of free/indie/studio/scale/enterprise — likely a retired alias (pro/starter/growth/team) or a typo. Reconcile the subscription to a canonical tier.",
    });
  }

  // Litter: a litter NAME is a stronger signal than a merely-empty tenant, so a
  // tenant matching both gets the trial/test finding (not the generic empty one).
  if (isLitterName(tenant.name)) {
    findings.push({
      code: "trial_or_test_tenant",
      category: "litter",
      severity: "info",
      title: "Looks like a trial / demo / test tenant",
      detail:
        "The name matches a trial/demo/smoke/epoch-suffix pattern — likely litter from a probe or smoke test. Consider reaping if it is a demo tenant.",
    });
  } else if (isEmptyTenant(tenant)) {
    findings.push({
      code: "empty_tenant",
      category: "litter",
      severity: "info",
      title: "Empty tenant (0 events, 0 members)",
      detail:
        "No events and no members — a fully-empty shell. Often abandoned onboarding or probe litter; safe to review for reaping.",
    });
  }

  return findings;
}

// ── List-level analysis ────────────────────────────────────────────────

/** A tenant's findings keyed for quick row lookup. */
export type FindingsByTenant = Map<string, ClientFinding[]>;

export interface FakeCreatedAtResult {
  /** True when ≥ SAME_DATE_THRESHOLD tenants share one calendar date. */
  suspicious: boolean;
  /** The over-represented date (YYYY-MM-DD), when suspicious. */
  date: string | null;
  /** How many tenants share that date. */
  count: number;
}

/**
 * Detect the `time.Now()`-on-load created_at bug: if too many tenants share ONE
 * calendar date, the column is almost certainly defaulted, not real (ADMIN_HEALTH
 * §4 / the data-integrity gate). Returns the worst (most over-represented) date.
 *
 * NOTE: operates over whatever slice of tenants is passed; a single page may not
 * hit the threshold even when the fleet does. Pass the largest list you have.
 */
export function detectFakeCreatedAt(
  tenants: readonly Tenant[],
  threshold: number = SAME_DATE_THRESHOLD
): FakeCreatedAtResult {
  const counts = new Map<string, number>();
  for (const t of tenants) {
    const day = calendarDate(t.created_at);
    if (day == null) continue;
    counts.set(day, (counts.get(day) ?? 0) + 1);
  }
  let worstDate: string | null = null;
  let worstCount = 0;
  for (const [day, n] of counts) {
    if (n > worstCount) {
      worstCount = n;
      worstDate = day;
    }
  }
  return {
    suspicious: worstCount >= threshold,
    date: worstCount >= threshold ? worstDate : null,
    count: worstCount,
  };
}

export interface ClientAnalysis {
  /** Per-tenant findings (only tenants WITH findings appear). */
  byTenant: FindingsByTenant;
  /** Worst severity per flagged tenant, for the row badge. */
  worstByTenant: Map<string, AnomalySeverity>;
  /** Number of tenants with ≥1 finding. */
  flaggedCount: number;
  /** Finding counts by severity across all tenants. */
  bySeverity: Record<AnomalySeverity, number>;
  /** The fleet-wide fake-created_at signal (page-level banner). */
  fakeCreatedAt: FakeCreatedAtResult;
}

/**
 * Run every heuristic over a tenant list and return the merged, render-ready
 * client analysis. Pure: zero backend calls, safe to recompute on each render.
 */
export function analyzeTenants(tenants: readonly Tenant[]): ClientAnalysis {
  const byTenant: FindingsByTenant = new Map();
  const worstByTenant = new Map<string, AnomalySeverity>();
  const bySeverity: Record<AnomalySeverity, number> = { critical: 0, warn: 0, info: 0 };

  for (const t of tenants) {
    const findings = analyzeTenant(t);
    if (findings.length === 0) continue;
    byTenant.set(t.id, findings);
    const worst = worstSeverityOf(findings);
    if (worst) worstByTenant.set(t.id, worst);
    for (const f of findings) bySeverity[f.severity] += 1;
  }

  return {
    byTenant,
    worstByTenant,
    flaggedCount: byTenant.size,
    bySeverity,
    fakeCreatedAt: detectFakeCreatedAt(tenants),
  };
}
