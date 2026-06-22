/**
 * Fleet health & recovery API client for the admin dashboard.
 *
 * Thin consumer of the Control Plane fleet/recovery endpoints. This client
 * performs NO scoring — it renders the tiers/reasons the Control Plane returns.
 * Mirrors the exact pattern in tenants-api.ts / billing-api.ts:
 *   fetch(url, { credentials: "include" }), typed interfaces, getApiUrl() from
 *   NEXT_PUBLIC_API_URL.
 *
 * Endpoints (Control Plane admin group, gated by AdminAuthMiddleware):
 *   GET  /api/v1/admin/fleet/health?tier=&limit=
 *   GET  /api/v1/admin/fleet/health/:id
 *   GET  /api/v1/admin/recovery/diagnose/edition
 *   GET  /api/v1/admin/recovery/:id/diagnose-identity
 *   POST /api/v1/admin/recovery/:id/{resync,reconcile-subscription,resolve-dunning,
 *                                    rotate-keys,reprovision,restore}
 *   POST /api/v1/admin/recovery/batch
 *
 * Every mutating endpoint accepts `dry_run`; a dry-run returns `{ dry_run, would, confirm_token? }`.
 * Destructive actions additionally require `confirm_tenant_id` (typed-name) or
 * `confirm_token` (echoed from the dry-run).
 */

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    return process.env.NEXT_PUBLIC_API_URL || "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

// ── Health model (rendered, never computed here) ──────────────────────

/** The four tiers the Control Plane assigns. Worst-wins severity. */
export type HealthTier = "healthy" | "degraded" | "at_risk" | "critical";

/** A single contributing signal: name, observed value, and the tier it triggered. */
export interface HealthReason {
  signal: string;
  value: string;
  tier: HealthTier;
}

/** A signal on the per-tenant assessment — reason + the backend it was read from. */
export interface HealthSignal {
  signal: string;
  value: string;
  tier: HealthTier;
  source: string;
}

/** Subscription block surfaced on the per-tenant assessment (§3). */
export interface HealthSubscription {
  tier?: string;
  status?: string;
  renews_at?: string;
  trial_ends_at?: string;
  grandfathered?: boolean;
  grandfather_until?: string;
  events_used?: number;
  events_quota?: number;
}

/** One row in the fleet "worst-N" list. */
export interface FleetWorstTenant {
  tenant_id: string;
  name: string;
  tier: HealthTier;
  reasons: HealthReason[];
}

/** Fleet rollup: per-tier counts + the ordered worst-N list (§2.2). */
export interface FleetHealthSummary {
  summary: {
    total: number;
    healthy: number;
    degraded: number;
    at_risk: number;
    critical: number;
  };
  worst: FleetWorstTenant[];
  _links?: {
    self?: string;
    tenant_template?: string;
  };
}

/** Single-tenant assessment: all signals + values + subscription (§5.1). */
export interface TenantHealth {
  tenant_id: string;
  name?: string;
  tier: HealthTier;
  signals: HealthSignal[];
  subscription?: HealthSubscription;
}

export interface FetchFleetHealthParams {
  tier?: HealthTier;
  limit?: number;
}

// ── Diagnosis shapes (Safe / read-only) ───────────────────────────────

/** Edition-trap detection (runbook §5 #5). Returns a copy-paste remediation. */
export interface EditionDiagnosis {
  trap_detected: boolean;
  edition?: string;
  non_community_data_tenants?: number;
  remediation_command?: string;
  verification?: string;
  detail?: string;
}

/** Tenant/JWT/store divergence (runbook §7). No mutation — the fix is a re-login. */
export interface IdentityDiagnosis {
  tenant_id: string;
  divergence_detected: boolean;
  jwt_tenant_id?: string;
  store_tenant_id?: string;
  resolved_tenant_id?: string;
  remediation?: string;
  detail?: string;
}

// ── Recovery action shapes ────────────────────────────────────────────

/** Recovery actions exposed by the NEW fleet endpoints. Guarded/Destructive only. */
export type RecoveryAction =
  | "resync"
  | "reconcile-subscription"
  | "resolve-dunning"
  | "rotate-keys"
  | "reprovision"
  | "restore";

/** Batch-allowed actions — Destructive sub-actions are forbidden in batch (§4.3). */
export type BatchRecoveryAction = "resync" | "reconcile-subscription" | "resolve-dunning";

/**
 * Request body shared across mutating actions (§5.1).
 *   dry_run:true  → returns a `would` preview + (for token-gated) a confirm_token
 *   confirm_tenant_id → typed-name confirmation for Destructive single-tenant actions
 *   confirm_token     → echoed token from a prior dry-run (rotate-keys, batch)
 */
export interface RecoveryRequest {
  dry_run?: boolean;
  reason?: string;
  confirm_tenant_id?: string;
  confirm_token?: string;
  /** restore only: the snapshot to replay. */
  snapshot_id?: string;
  [key: string]: unknown;
}

export interface BatchRecoveryRequest {
  filter: Record<string, unknown>;
  action: BatchRecoveryAction;
  max_tenants?: number;
  dry_run?: boolean;
  confirm_token?: string;
}

/**
 * Recovery response. A dry-run echoes `dry_run:true`, the `would` preview, and
 * (for token-gated actions) a `confirm_token` the Apply call must echo back.
 * A batch dry-run additionally lists the affected tenants + the count to confirm.
 */
export interface RecoveryResponse {
  dry_run?: boolean;
  applied?: boolean;
  would?: Record<string, unknown>;
  confirm_token?: string;
  affected?: Array<{ tenant_id: string; name?: string; preview?: Record<string, unknown> }>;
  count?: number;
  detail?: string;
  audit_event_id?: string;
}

// ── Health fetchers ───────────────────────────────────────────────────

export async function fetchFleetHealth(
  params: FetchFleetHealthParams = {}
): Promise<FleetHealthSummary> {
  const searchParams = new URLSearchParams();
  if (params.tier) searchParams.set("tier", params.tier);
  if (params.limit) searchParams.set("limit", String(params.limit));

  const qs = searchParams.toString();
  const url = `${getApiUrl()}/api/v1/admin/fleet/health${qs ? `?${qs}` : ""}`;

  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to fetch fleet health: ${res.status}`);
  }
  return res.json();
}

export async function fetchTenantHealth(id: string): Promise<TenantHealth> {
  const url = `${getApiUrl()}/api/v1/admin/fleet/health/${id}`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to fetch tenant health: ${res.status}`);
  }
  return res.json();
}

// ── Diagnosis fetchers (Safe / read-only) ─────────────────────────────

export async function diagnoseEdition(): Promise<EditionDiagnosis> {
  const url = `${getApiUrl()}/api/v1/admin/recovery/diagnose/edition`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to diagnose edition: ${res.status}`);
  }
  return res.json();
}

export async function diagnoseIdentity(id: string): Promise<IdentityDiagnosis> {
  const url = `${getApiUrl()}/api/v1/admin/recovery/${id}/diagnose-identity`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to diagnose tenant identity: ${res.status}`);
  }
  return res.json();
}

// ── Recovery (mutating) ───────────────────────────────────────────────

/**
 * Run a single-tenant recovery action. The `dry_run` + confirmation params in
 * `body` are passed straight through to POST /api/v1/admin/recovery/:id/<action>.
 * The client renders the guard; the Control Plane enforces it.
 */
export async function runRecovery(
  id: string,
  action: RecoveryAction,
  body: RecoveryRequest
): Promise<RecoveryResponse> {
  const url = `${getApiUrl()}/api/v1/admin/recovery/${id}/${action}`;
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(`Recovery ${action} failed: ${res.status}`);
  }
  return res.json();
}

/**
 * Run a bounded batch recovery (Guarded sub-actions only). Passes the filter,
 * action, max_tenants cap, dry_run, and echoed confirm_token straight through to
 * POST /api/v1/admin/recovery/batch.
 */
export async function runBatchRecovery(body: BatchRecoveryRequest): Promise<RecoveryResponse> {
  const url = `${getApiUrl()}/api/v1/admin/recovery/batch`;
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(`Batch recovery failed: ${res.status}`);
  }
  return res.json();
}

// ── Presentation helpers (display only — not scoring) ─────────────────

/** Human-readable label for a tier. */
export function tierLabel(tier: HealthTier): string {
  switch (tier) {
    case "healthy":
      return "Healthy";
    case "degraded":
      return "Degraded";
    case "at_risk":
      return "At-Risk";
    case "critical":
      return "Critical";
    default:
      return tier;
  }
}

/**
 * Map a tier to the StatCard status vocabulary ("healthy" | "warning" | "critical").
 * Degraded + At-Risk both surface as the warning colour on stat cards; the chip
 * keeps the four-way distinction.
 */
export function tierToStatCardStatus(tier: HealthTier): "healthy" | "warning" | "critical" {
  if (tier === "critical" || tier === "at_risk") return "critical";
  if (tier === "degraded") return "warning";
  return "healthy";
}
