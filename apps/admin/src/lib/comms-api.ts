/**
 * Proactive-comms API client for the admin dashboard (ADMIN_TENANT_POWER_TOOL
 * §4 Pillar C / §9 Phase 6 admin half).
 *
 * Thin consumer of the Control Plane comms endpoints (prompt 037,
 * comms_handler.go). Mirrors the exact client pattern in tenants-api.ts /
 * billing-api.ts / fleet-api.ts:
 *   - every call is SAME-ORIGIN: `fetch("/api/v1/...", { credentials: "include" })`
 *     so the BFF (src/app/api/v1/[...path]/route.ts) attaches the admin_token
 *     Bearer and forwards to the Control Plane (the CP is Bearer-only via the BFF);
 *   - list-returning functions resolve through `asList()` so a wrapped/null
 *     response can never crash a page's `.map` (§6 resilience rule 1);
 *   - this client renders the server's guard contract; it NEVER bypasses it.
 *     The cohort dry-run → echoed-confirm_token guard is server-enforced; the UI
 *     just forwards `dry_run` + `confirm_token` straight through.
 *
 * Endpoints (Control Plane admin group, gated by AdminAuthMiddleware):
 *   POST /api/v1/admin/notices            create an in-app notice (tenant | cohort)
 *   GET  /api/v1/admin/notices?tenant_id= list notices (admin view)
 *   POST /api/v1/admin/messages           send an operator→tenant email (SMTP), audited
 *   POST /api/v1/admin/tenants/:id/notes  add a support note
 *   GET  /api/v1/admin/tenants/:id/notes  list support notes
 *
 * Read-only fleet-health (for the at-risk cohort) comes from fleet-api.ts'
 * fetchFleetHealth — this client does NOT re-derive tiers.
 */

import { type FetchFleetHealthParams, type FleetWorstTenant, fetchFleetHealth } from "./fleet-api";

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
 * see security-api.ts:26 / billing-api.ts:84 / tenants-api.ts:24). The Control
 * Plane wraps list responses inconsistently ({notices:[…]}, {notes:[…]}, {items},
 * {data}, a bare array, or a null); a non-array reaching a page's `.map` crashes
 * the whole route ("x.map is not a function"). Always resolve to an array.
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

// ── Notices ───────────────────────────────────────────────────────────

/** Notice severity — matches the CP `normalizeSeverity` accepted values. */
export type NoticeSeverity = "info" | "warning" | "critical";

/**
 * Notice audience. A single tenant ({tenant_id}) posts immediately; a cohort
 * ({tier} | {plan} | {health_tier}) goes through the recovery blast-radius guard
 * (dry-run preview → echoed confirm_token). Exactly one targeting key is set.
 */
export interface NoticeAudience {
  tenant_id?: string;
  /** Billing tier cohort (indie/studio/scale/…). */
  tier?: string;
  /** Alias for tier in the admin filter vocabulary. */
  plan?: string;
  /** Health-signal tier cohort (at_risk/critical/…) — drives at-risk outreach. */
  health_tier?: string;
}

/** Request body for POST /api/v1/admin/notices (prompt 037 shape). */
export interface CreateNoticeRequest {
  audience: NoticeAudience;
  title: string;
  body: string;
  severity: NoticeSeverity;
  expires_at?: string;
  dry_run?: boolean;
  confirm_token?: string;
}

/** One posted notice in a create response. */
export interface NoticeCreated {
  id: string;
  tenant_id: string;
}

/** Cohort dry-run preview: the recipients + count, mutating nothing. */
export interface NoticeWould {
  recipient_tenant_ids: string[];
  count: number;
}

/**
 * POST /api/v1/admin/notices result. Three shapes, all on one struct (prompt 037):
 *   single                → { dry_run:false, cohort:false, created:[{id,tenant_id}], count:1 }
 *   cohort dry_run:true    → { dry_run:true,  cohort:true,  would:{recipient_tenant_ids,count}, confirm_token, count }
 *   cohort apply           → { cohort:true, created:[…], count }
 */
export interface CreateNoticeResult {
  dry_run?: boolean;
  cohort?: boolean;
  would?: NoticeWould;
  confirm_token?: string;
  created?: NoticeCreated[];
  count?: number;
  message?: string;
}

/** A notice in the admin list view (and the tenant banner). */
export interface NoticeView {
  id: string;
  tenant_id: string;
  title: string;
  body: string;
  severity: NoticeSeverity;
  created_at: string;
  expires_at?: string;
  dismissed?: boolean;
}

/**
 * Create an in-app notice. The `dry_run` + `confirm_token` fields are forwarded
 * straight through to the Control Plane, which enforces the cohort blast-radius
 * guard (dry-run preview → echoed confirm_token). The client renders the guard;
 * the CP enforces it.
 */
export async function createNotice(body: CreateNoticeRequest): Promise<CreateNoticeResult> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/notices`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(await commsError(res, "create notice"));
  }
  return res.json();
}

/**
 * List notices (admin view). Filter to a single tenant by passing `tenant_id`;
 * omit it for all tenants. Always returns an array via asList.
 */
export async function listNotices(tenantId?: string): Promise<NoticeView[]> {
  const qs = tenantId ? `?tenant_id=${encodeURIComponent(tenantId)}` : "";
  const res = await fetch(`${getApiUrl()}/api/v1/admin/notices${qs}`, {
    credentials: "include",
  });
  if (!res.ok) {
    throw new Error(`Failed to list notices: ${res.status}`);
  }
  const data = await res.json();
  return asList<NoticeView>(data, "notices");
}

// ── Messages (operator → tenant email) ────────────────────────────────

/**
 * The operator templates the CP renders (comms_templates.go). A bespoke message
 * passes `subject`+`body` instead of a template. `custom` is the no-cooldown,
 * operator-driven bespoke key.
 */
export type MessageTemplate =
  | "at_risk_outreach"
  | "quota_warning"
  | "onboarding_nudge"
  | "dunning_reminder"
  | "custom";

/** Request body for POST /api/v1/admin/messages — template OR subject+body. */
export interface SendMessageRequest {
  tenant_id: string;
  template?: MessageTemplate;
  subject?: string;
  body?: string;
  dry_run?: boolean;
}

/** Skip reasons surfaced by the CP when a send is suppressed (NOT a failure). */
export type MessageSkipReason = "skipped_opt_out" | "skipped_rate_limit";

/**
 * POST /api/v1/admin/messages result (prompt 037). `skipped:true` with a
 * `skip_reason` is a SUCCESS response, not an error — the UI surfaces it as
 * opt-out / rate-limit feedback rather than failing silently.
 */
export interface SendMessageResult {
  tenant_id: string;
  template: string;
  dry_run: boolean;
  sent: boolean;
  skipped: boolean;
  skip_reason?: MessageSkipReason | string;
  recipient?: string;
  subject?: string;
  message?: string;
}

/**
 * Send an operator→tenant email over the existing SMTP client (audited). Opt-out
 * and rate-limit suppression come back as `{ skipped:true, skip_reason }` (HTTP
 * 200), NOT as an error — the caller must surface that feedback.
 */
export async function sendMessage(body: SendMessageRequest): Promise<SendMessageResult> {
  const res = await fetch(`${getApiUrl()}/api/v1/admin/messages`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(await commsError(res, "send message"));
  }
  return res.json();
}

// ── Support notes (internal-only, per tenant) ─────────────────────────

/** A projected support note (admin-internal; never sent to the tenant). */
export interface NoteView {
  id: string;
  tenant_id: string;
  body: string;
  actor: string;
  created_at: string;
}

/** Add a support note to a tenant. Returns the created note. */
export async function addNote(tenantId: string, body: string): Promise<NoteView> {
  const res = await fetch(
    `${getApiUrl()}/api/v1/admin/tenants/${encodeURIComponent(tenantId)}/notes`,
    {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ body }),
    }
  );
  if (!res.ok) {
    throw new Error(await commsError(res, "add note"));
  }
  return res.json();
}

/** List a tenant's support notes (admin view). Always returns an array via asList. */
export async function listNotes(tenantId: string): Promise<NoteView[]> {
  const res = await fetch(
    `${getApiUrl()}/api/v1/admin/tenants/${encodeURIComponent(tenantId)}/notes`,
    { credentials: "include" }
  );
  if (!res.ok) {
    throw new Error(`Failed to list notes: ${res.status}`);
  }
  const data = await res.json();
  return asList<NoteView>(data, "notes");
}

// ── At-risk cohort (read-only — composes fleet health) ────────────────

/**
 * Fetch the at-risk/critical cohort for outreach. Composes fleet-api.ts'
 * fetchFleetHealth (the shipped health rollup) and returns the worst-N tenants —
 * this client does NOT re-derive tiers. The at-risk page filters the returned
 * `worst` list to the tiers it targets. Always returns an array.
 */
export async function fetchAtRiskCohort(
  params: FetchFleetHealthParams = { limit: 100 }
): Promise<FleetWorstTenant[]> {
  const summary = await fetchFleetHealth(params);
  return Array.isArray(summary?.worst) ? summary.worst : [];
}

// ── Error helper ──────────────────────────────────────────────────────

/**
 * Build an error message from a non-OK comms response, preferring the CP's JSON
 * `{error}` body (the recovery/comms handlers always return one) so the error
 * boundary surfaces the real cause — e.g. "comms: cohort exceeds max_recipients
 * ceiling" (422) or a confirm-token mismatch (400) — not just a bare status.
 */
async function commsError(res: Response, action: string): Promise<string> {
  try {
    const data = await res.json();
    if (data && typeof data === "object" && typeof data.error === "string") {
      return `Failed to ${action}: ${data.error}`;
    }
  } catch {
    // fall through to the status-only message
  }
  return `Failed to ${action}: ${res.status}`;
}
