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
    return process.env.NEXT_PUBLIC_API_URL || "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
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
  return data.rules || data;
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
  return res.json();
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
  return data.summary || data;
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
  return data.policies || data;
}
