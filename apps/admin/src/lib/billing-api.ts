/**
 * Billing API client for the admin billing dashboard.
 *
 * Endpoints on the Control Plane:
 *   GET  /api/v1/admin/billing/revenue?range=30d|90d|1y
 *   GET  /api/v1/admin/billing/invoices?tenant_id=&status=&page=
 *   POST /api/v1/admin/billing/refund
 *   GET  /api/v1/admin/billing/dunning
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
// Revenue types
// ---------------------------------------------------------------------------

export type RevenueRange = "30d" | "90d" | "1y";

export interface RevenueSummary {
  mrr: number;
  arr: number;
  growth_rate: number;
  churn_rate: number;
  trend: RevenueTrendPoint[];
}

export interface RevenueTrendPoint {
  date: string;
  mrr: number;
}

// ---------------------------------------------------------------------------
// Invoice types
// ---------------------------------------------------------------------------

export type InvoiceStatus = "paid" | "pending" | "failed";

export interface Invoice {
  id: string;
  tenant_id: string;
  tenant_name: string;
  amount: number;
  currency: string;
  status: InvoiceStatus;
  created_at: string;
}

export interface InvoicesResponse {
  invoices: Invoice[];
  total: number;
  page: number;
  per_page: number;
}

// ---------------------------------------------------------------------------
// Dunning types
// ---------------------------------------------------------------------------

export interface DunningEntry {
  tenant_id: string;
  tenant_name: string;
  retry_count: number;
  last_attempt: string;
  amount_due: number;
  currency: string;
}

// ---------------------------------------------------------------------------
// Revenue API
// ---------------------------------------------------------------------------

export async function fetchRevenue(range: RevenueRange): Promise<RevenueSummary> {
  const res = await fetch(
    `${getApiUrl()}/api/v1/admin/billing/revenue?range=${range}`,
    { credentials: "include" }
  );
  if (!res.ok) {
    throw new Error(`Failed to fetch revenue: ${res.status}`);
  }
  return res.json();
}

// ---------------------------------------------------------------------------
// Invoices API
// ---------------------------------------------------------------------------

export async function fetchInvoices(params: {
  tenant_id?: string;
  status?: InvoiceStatus;
  page?: number;
}): Promise<InvoicesResponse> {
  const searchParams = new URLSearchParams();
  if (params.tenant_id) searchParams.set("tenant_id", params.tenant_id);
  if (params.status) searchParams.set("status", params.status);
  if (params.page) searchParams.set("page", String(params.page));

  const res = await fetch(
    `${getApiUrl()}/api/v1/admin/billing/invoices?${searchParams.toString()}`,
    { credentials: "include" }
  );
  if (!res.ok) {
    throw new Error(`Failed to fetch invoices: ${res.status}`);
  }
  return res.json();
}

// ---------------------------------------------------------------------------
// Refund API
// ---------------------------------------------------------------------------

export async function processRefund(invoiceId: string): Promise<void> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/billing/refund`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ invoice_id: invoiceId }),
  });
  if (!res.ok) {
    throw new Error(`Failed to process refund: ${res.status}`);
  }
}

// ---------------------------------------------------------------------------
// Dunning API
// ---------------------------------------------------------------------------

export async function fetchDunning(): Promise<DunningEntry[]> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/billing/dunning`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch dunning entries: ${res.status}`);
  }
  const data = await res.json();
  return data.entries || data;
}

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

export function formatCurrency(amount: number, currency = "USD"): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency,
    minimumFractionDigits: 2,
  }).format(amount);
}

export function formatPercent(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(1)}%`;
}
