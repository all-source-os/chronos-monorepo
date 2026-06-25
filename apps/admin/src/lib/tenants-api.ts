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

// Canonical subscription tiers (Control Plane authority — subscription.go).
// The CP filters the tenant list by exact case-insensitive match against these
// stored tier strings, so the admin filter MUST use the same names. The retired
// 010 aliases (starter/pro/growth/team) match nothing and are gone.
export type TenantPlan =
  | "free"
  | "indie"
  | "studio"
  | "scale"
  | "enterprise";
export type TenantStatus = "active" | "suspended" | "archived";

export interface Tenant {
  id: string;
  name: string;
  plan: TenantPlan;
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
  plan: TenantPlan;
  started_at: string;
  current_period_end: string;
}

export interface TenantQuotas {
  event_limit: number;
  query_limit: number;
  storage_limit_mb: number;
}

export interface TenantDetail extends Tenant {
  description?: string;
  quotas: TenantQuotas;
  members: TenantMember[];
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
  return res.json();
}

export async function fetchTenantUsage(id: string): Promise<TenantUsage> {
  const url = `${getApiUrl()}/api/v1/admin/tenants/${id}/usage`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to fetch tenant usage: ${res.status}`);
  }
  return res.json();
}

export async function updateTenantQuotas(
  id: string,
  quotas: TenantQuotas
): Promise<void> {
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

export async function fetchTenants(
  params: FetchTenantsParams = {}
): Promise<TenantsResponse> {
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
