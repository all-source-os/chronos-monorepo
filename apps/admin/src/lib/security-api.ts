/**
 * Security API client for the admin security dashboard.
 *
 * Fetches data from the Control Plane admin security endpoints:
 *   GET/POST/DELETE /api/v1/admin/security/ip-rules
 *   GET /api/v1/admin/security/token-audit
 *   GET /api/v1/admin/security/token-audit/summary
 *   GET /api/v1/policies
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
 * Coerce a possibly-wrapped API response into an array. The Control Plane wraps
 * list responses inconsistently ({rules:[...]}, {items:[...]}, or a bare array);
 * returning a non-array makes a page's `.map` throw and crash the whole route
 * ("x.map is not a function"). Always resolve to an array.
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

// --- IP Rules ---

export interface IpRule {
  id: string;
  cidr: string;
  rule_type: "allow" | "deny";
  description: string;
  created_at: string;
}

export async function fetchIpRules(): Promise<IpRule[]> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/security/ip-rules`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch IP rules: ${res.status}`);
  }
  const data = await res.json();
  return asList<IpRule>(data, "rules", "ip_rules");
}

export async function createIpRule(rule: {
  cidr: string;
  rule_type: "allow" | "deny";
  description: string;
}): Promise<IpRule> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/security/ip-rules`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(rule),
  });
  if (!res.ok) {
    throw new Error(`Failed to create IP rule: ${res.status}`);
  }
  return res.json();
}

export async function deleteIpRule(id: string): Promise<void> {
  const res = await fetch(
    `${getApiUrl()}/api/v1/admin/security/ip-rules/${id}`,
    {
      method: "DELETE",
      credentials: "include",
    }
  );
  if (!res.ok) {
    throw new Error(`Failed to delete IP rule: ${res.status}`);
  }
}

// --- Token Audit ---

export interface TokenAuditEntry {
  id: string;
  tenant_id: string;
  tenant_name: string;
  token_prefix: string;
  action: string;
  ip_address: string;
  timestamp: string;
}

export interface TokenAuditResponse {
  entries: TokenAuditEntry[];
  total: number;
  page: number;
  per_page: number;
}

export async function fetchTokenAudit(params: {
  tenant_id?: string;
  from?: string;
  to?: string;
  page?: number;
}): Promise<TokenAuditResponse> {
  const searchParams = new URLSearchParams();
  if (params.tenant_id) searchParams.set("tenant_id", params.tenant_id);
  if (params.from) searchParams.set("from", params.from);
  if (params.to) searchParams.set("to", params.to);
  if (params.page) searchParams.set("page", String(params.page));

  const res = await fetch(
    `${getApiUrl()}/api/v1/admin/security/token-audit?${searchParams.toString()}`,
    { credentials: "include" }
  );
  if (!res.ok) {
    throw new Error(`Failed to fetch token audit: ${res.status}`);
  }
  const data = (await res.json()) ?? {};
  return { ...data, entries: asList<TokenAuditEntry>(data, "entries") };
}

export interface TokenAuditSummaryEntry {
  tenant_id: string;
  tenant_name: string;
  api_calls: number;
}

export async function fetchTokenAuditSummary(): Promise<
  TokenAuditSummaryEntry[]
> {
  const res = await fetch(
    `${getApiUrl()}/api/v1/admin/security/token-audit/summary`,
    { credentials: "include" }
  );
  if (!res.ok) {
    throw new Error(`Failed to fetch token audit summary: ${res.status}`);
  }
  const data = await res.json();
  return asList<TokenAuditSummaryEntry>(data, "summary");
}

// --- RBAC Policies ---

export interface RbacPolicy {
  id: string;
  name: string;
  description: string;
  permissions: string[];
  created_at: string;
}

export async function fetchPolicies(): Promise<RbacPolicy[]> {
  const res = await fetch(`${getApiUrl()}/api/v1/policies`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch policies: ${res.status}`);
  }
  const data = await res.json();
  return asList<RbacPolicy>(data, "policies").map((p) => ({
    ...p,
    permissions: Array.isArray(p.permissions) ? p.permissions : [],
  }));
}
