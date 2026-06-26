/**
 * Tenants API client for the admin dashboard.
 *
 * Fetches data from the Control Plane admin endpoint:
 *   GET /api/v1/admin/tenants?search=&plan=&status=&page=&per_page=
 */

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    // Client-side: hit the same-origin BFF proxy (src/app/api/v1/[...path]/route.ts),
    // which attaches the admin_token Bearer and forwards to the Control Plane.
    return "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

/**
 * Coerce a possibly-wrapped API response into an array (the shared §6 pattern —
 * see security-api.ts:26 / billing-api.ts:84). The Control Plane wraps list
 * responses inconsistently ({members:[…]}, {items}, {data}, or a bare array, and
 * sometimes a null); a non-array reaching a page's `.map` crashes the whole route
 * ("x.map is not a function"). Always resolve to an array.
 */
export function asList<T>(data: unknown, ...keys: string[]): T[] {
  if (Array.isArray(data)) return data as T[];
  if (data && typeof data === "object") {
    for (const k of [...keys, "items", "data"]) {
      const v = (data as Record<string, unknown>)[k];
      if (Array.isArray(v)) return v as T[];
    }
  }
  return [];
}

// Canonical subscription tiers (Control Plane authority — subscription.go).
// The CP filters the tenant list by exact case-insensitive match against these
// stored tier strings, so the admin filter MUST use the same names. The retired
// 010 aliases (starter/pro/growth/team) match nothing and are gone.
export type TenantPlan = "free" | "indie" | "studio" | "scale" | "enterprise";
export type TenantStatus = "active" | "suspended" | "archived";

// The CP list DTO emits `plan` as a bare tier string ("studio"); the DETAIL DTO
// emits it as an object {name, tier} (and `subscription.plan` may be either, or
// null). Rendering the object directly as a React child throws Minified React
// error #31 ("objects are not valid as a React child, found object with keys
// {name, tier}") — which crashed the whole tenant 360 page. Every plan render
// site MUST go through planLabel(); never render a plan value raw (resilience §6).
export type PlanLike = TenantPlan | { name?: string; tier?: string } | string | null | undefined;

export function planLabel(plan: PlanLike): string {
  if (plan == null) return "—";
  if (typeof plan === "string") return plan;
  // tier is the canonical id (matches TenantPlan + the `capitalize` styling); name is the fallback.
  return plan.tier ?? plan.name ?? "—";
}

export interface Tenant {
  id: string;
  name: string;
  plan: PlanLike;
  status: TenantStatus;
  // Per-tenant counts emitted by the CP list/detail DTOs under the SINGULAR
  // canonical field names `event_count` / `member_count` (prompt 033). The
  // earlier plural `events_count`/`members_count` never deserialized and always
  // rendered 0. Optional + guarded with `?? 0` at every render site (§6).
  event_count?: number;
  member_count?: number;
  created_at: string;
}

export interface TenantsResponse {
  tenants: Tenant[];
  total: number;
  page: number;
  per_page: number;
  _links: {
    self: string;
    next?: string;
    prev?: string;
  };
}

export interface FetchTenantsParams {
  search?: string;
  plan?: TenantPlan | "";
  status?: TenantStatus | "";
  page?: number;
  per_page?: number;
}

// ── Detail types ──────────────────────────────────────────────────────

export interface TenantMember {
  id: string;
  email: string;
  role: string;
  joined_at: string;
}

export interface TenantSubscription {
  plan: PlanLike;
  started_at: string;
  current_period_end: string;
  // Optional billing-state fields the 360 surfaces when the CP detail carries
  // them (admin_tenant_dto.go SubscriptionInfo). All optional + guarded.
  status?: string;
  provider?: string;
  renews_at?: string;
  dunning_state?: string;
  grandfathered?: boolean;
  grandfather_until?: string;
}

export interface TenantQuotas {
  event_limit: number;
  query_limit: number;
  storage_limit_mb: number;
}

/**
 * An API key surfaced on the tenant 360 (Pillar A — API keys section). The CP
 * detail may include the tenant's keys; the canonical role string MUST be
 * `serviceaccount` (no underscore) — a drifted `service_account` silently 403s
 * every key (MEMORY: API-key role string contract). All fields optional/guarded.
 */
export interface ApiKeyInfo {
  id: string;
  name?: string;
  role?: string;
  created_at?: string;
  last_used_at?: string;
}

export interface TenantDetail extends Tenant {
  description?: string;
  home_region?: string;
  quotas: TenantQuotas;
  members: TenantMember[];
  api_keys?: ApiKeyInfo[];
  subscription: TenantSubscription;
  audit_log: AuditEntry[];
}

export interface AuditEntry {
  id: string;
  action: string;
  actor: string;
  timestamp: string;
  details?: string;
}

// ── Usage types ───────────────────────────────────────────────────────

export interface DailyUsagePoint {
  date: string;
  events: number;
}

export interface TenantUsage {
  events_ingested: number;
  queries_run: number;
  storage_used_mb: number;
  event_limit: number;
  query_limit: number;
  storage_limit_mb: number;
  daily: DailyUsagePoint[];
}

// ── Detail fetchers ───────────────────────────────────────────────────

export async function fetchTenantDetail(id: string): Promise<TenantDetail> {
  const url = `${getApiUrl()}/api/v1/admin/tenants/${id}`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to fetch tenant detail: ${res.status}`);
  }
  const data = (await res.json()) ?? {};
  // Guard every list field so a wrapped/null payload can't crash the 360's .map (§6).
  return {
    ...data,
    members: asList<TenantMember>(data, "members"),
    api_keys: asList<ApiKeyInfo>(data, "api_keys", "keys"),
    audit_log: asList<AuditEntry>(data, "audit_log", "audit", "events"),
  };
}

// ── Per-tenant invoices (Pillar A — Subscription/billing section) ─────────
// Same-origin BFF read of the existing admin billing endpoint, filtered to this
// tenant. Mirrors billing-api.ts fetchInvoices but scoped + array-guarded so the
// 360 can show the tenant's recent invoices without importing the billing page.

export interface TenantInvoice {
  id: string;
  amount: number;
  currency: string;
  status: string;
  created_at: string;
}

export async function fetchTenantInvoices(id: string): Promise<TenantInvoice[]> {
  const url = `${getApiUrl()}/api/v1/admin/billing/invoices?tenant_id=${encodeURIComponent(id)}`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to fetch tenant invoices: ${res.status}`);
  }
  const data = (await res.json()) ?? {};
  return asList<TenantInvoice>(data, "invoices");
}

export async function fetchTenantUsage(id: string): Promise<TenantUsage> {
  const url = `${getApiUrl()}/api/v1/admin/tenants/${id}/usage`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to fetch tenant usage: ${res.status}`);
  }
  return res.json();
}

export async function updateTenantQuotas(id: string, quotas: TenantQuotas): Promise<void> {
  const url = `${getApiUrl()}/api/v1/admin/tenants/${id}/quotas`;
  const res = await fetch(url, {
    method: "PUT",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(quotas),
  });
  if (!res.ok) {
    throw new Error(`Failed to update quotas: ${res.status}`);
  }
}

export async function suspendTenant(id: string): Promise<void> {
  const url = `${getApiUrl()}/api/v1/admin/tenants/${id}/suspend`;
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to suspend tenant: ${res.status}`);
  }
}

export async function unsuspendTenant(id: string): Promise<void> {
  const url = `${getApiUrl()}/api/v1/admin/tenants/${id}/unsuspend`;
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to unsuspend tenant: ${res.status}`);
  }
}

// ── List fetcher ──────────────────────────────────────────────────────

export async function fetchTenants(params: FetchTenantsParams = {}): Promise<TenantsResponse> {
  const searchParams = new URLSearchParams();

  if (params.search) searchParams.set("search", params.search);
  if (params.plan) searchParams.set("plan", params.plan);
  if (params.status) searchParams.set("status", params.status);
  if (params.page) searchParams.set("page", String(params.page));
  if (params.per_page) searchParams.set("per_page", String(params.per_page));

  const qs = searchParams.toString();
  const url = `${getApiUrl()}/api/v1/admin/tenants${qs ? `?${qs}` : ""}`;

  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to fetch tenants: ${res.status}`);
  }
  return res.json();
}
