/**
 * AI Inbox API client for the admin dashboard.
 *
 * Thin consumer of the Control Plane admin inbox endpoints added in prompt 043.
 * Mirrors the exact pattern in tenants-api.ts / fleet-api.ts:
 *   fetch(url, { credentials: "include" }), typed interfaces, getApiUrl() that
 *   resolves to the same-origin BFF proxy (src/app/api/v1/[...path]/route.ts),
 *   which attaches the admin_token Bearer and forwards to the Control Plane.
 *
 * Endpoints (Control Plane admin group, gated by ManageTenants permission —
 * except the OAuth callback, which is public and authenticated by sealed state):
 *   GET    /api/v1/admin/inbox/connect?tenant_id=&email=&return_to=  -> { auth_url, tenant_id }
 *   GET    /api/v1/webhooks/inbox/connect/callback                   (OAuth return; 302 -> return_to?status=…)
 *   GET    /api/v1/admin/inbox/connections[?tenant_id=]              -> { connections, count }
 *   DELETE /api/v1/admin/inbox/connections/:grant_id                 -> { status, grant_id }
 *   GET    /api/v1/admin/inbox/messages?tenant_id=&limit=            -> { messages: email.* events, count }
 *   POST   /api/v1/admin/inbox/triage                                -> { status, message_id, label }
 *   POST   /api/v1/admin/inbox/draft                                 -> { status, draft_id }
 *   POST   /api/v1/admin/inbox/send  { …, confirm:true }             -> { status, message_id, thread_id? }
 *
 * The proxy authorizes every call with the admin session, so the client never
 * holds a token. A non-2xx surfaces as a thrown ApiError whose `.status` lets the
 * page distinguish auth failures (401/403) from "not configured" (503) instead of
 * silently rendering an empty page (MEMORY: admin cross-origin auth is finicky —
 * surface it).
 */

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    // Client-side: hit the same-origin BFF proxy, which attaches the admin_token
    // Bearer and forwards to the Control Plane.
    return "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

/** A typed fetch error that preserves the HTTP status and the CP's error body. */
export class ApiError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

/**
 * Coerce a possibly-wrapped API response into an array (the shared §6 pattern —
 * see tenants-api.ts:asList). A non-array reaching a page's `.map` crashes the
 * whole route; always resolve to an array.
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

async function readError(res: Response): Promise<string> {
  try {
    const body = (await res.json()) as { error?: string; reason?: string };
    return body.error || body.reason || `Request failed (${res.status})`;
  } catch {
    return `Request failed (${res.status})`;
  }
}

// ── Types ─────────────────────────────────────────────────────────────

/** A connected mailbox (a sealed grant record, decrypted by the CP). */
export interface InboxConnection {
  grant_id: string;
  tenant_id: string;
  email: string;
  provider: string;
  connected_at: string;
}

export interface ConnectionsResponse {
  connections: InboxConnection[];
  count: number;
}

/** An email address as carried in event payloads ({name?, email}). */
export interface EmailAddress {
  name?: string;
  email: string;
}

/**
 * A single email.* event from Core, returned verbatim by the messages endpoint.
 * `event_type` is one of email.received | email.sent | email.triaged |
 * email.drafted; `payload` shape varies by type (see the typed accessors below).
 */
export interface EmailEvent {
  id: string;
  event_type: string;
  /** message_id for received/sent/triaged; draft_id for drafted. */
  entity_id: string;
  timestamp: string;
  payload?: Record<string, unknown>;
}

export interface MessagesResponse {
  messages: EmailEvent[];
  count: number;
}

export type TriageLabel = "needs-reply" | "fyi" | "spam" | "archive";

export interface TriageRequest {
  tenant_id: string;
  grant_id?: string;
  message_id: string;
  thread_id?: string;
  label: TriageLabel;
  reason?: string;
  by?: string;
}

export interface DraftRequest {
  tenant_id: string;
  grant_id?: string;
  thread_id: string;
  body: string;
  intent: string;
  in_reply_to?: string;
  subject?: string;
  to?: EmailAddress[];
  by?: string;
}

export interface DraftResponse {
  status: string;
  draft_id: string;
}

export interface GenerateDraftRequest {
  tenant_id: string;
  grant_id?: string;
  thread_id: string;
  /** Operator's instruction for the reply, e.g. "accept and propose Thursday". */
  intent: string;
  tone?: string;
  /** The mailbox address, used to frame the persona in the prompt. */
  mailbox_email?: string;
}

export interface GenerateDraftResponse {
  body: string;
  thread_id: string;
  grant_id?: string;
  /** What the model was grounded on, surfaced so the operator can trust it. */
  grounded_on?: { thread_messages: number; prior_threads: number };
}

export interface SendRequest {
  tenant_id: string;
  grant_id?: string;
  to: EmailAddress[];
  body: string;
  subject?: string;
  thread_id?: string;
  in_reply_to?: string;
  draft_id?: string;
  confirm: true;
}

export interface SendResponse {
  status: string;
  message_id: string;
  thread_id?: string;
  warning?: string;
}

export interface ConnectResponse {
  auth_url: string;
  tenant_id: string;
}

// ── Connect (hosted OAuth — leaves and re-enters the app) ─────────────

/**
 * Start the hosted-OAuth round trip. Returns the provider auth_url the browser
 * is sent to; the CP's callback bounces back to `return_to?status=connected|error`.
 * `return_to` MUST be an absolute URL whose origin is on the CP's
 * ADMIN_DASHBOARD_ORIGIN allowlist or the CP drops it (no open redirect).
 */
export async function startConnect(params: {
  tenant_id: string;
  email?: string;
  return_to: string;
}): Promise<ConnectResponse> {
  const qs = new URLSearchParams();
  qs.set("tenant_id", params.tenant_id);
  if (params.email) qs.set("email", params.email);
  qs.set("return_to", params.return_to);
  const url = `${getApiUrl()}/api/v1/admin/inbox/connect?${qs.toString()}`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
  return res.json();
}

// ── Connections ───────────────────────────────────────────────────────

export async function fetchConnections(tenantId?: string): Promise<InboxConnection[]> {
  const qs = tenantId ? `?tenant_id=${encodeURIComponent(tenantId)}` : "";
  const url = `${getApiUrl()}/api/v1/admin/inbox/connections${qs}`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
  const data = (await res.json()) ?? {};
  return asList<InboxConnection>(data, "connections");
}

export async function disconnect(grantId: string): Promise<void> {
  const url = `${getApiUrl()}/api/v1/admin/inbox/connections/${encodeURIComponent(grantId)}`;
  const res = await fetch(url, { method: "DELETE", credentials: "include" });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
}

// ── Adopt a hosted/existing provider grant (no OAuth login) ────────────

/** A provider mailbox (Nylas grant) available to adopt. */
export interface AvailableGrant {
  grant_id: string;
  email: string;
  provider: string;
  /** Already mapped to a tenant (has a Core config record). */
  registered: boolean;
}

/** List the provider's grants (hosted + connected), with a registered flag. */
export async function availableGrants(): Promise<AvailableGrant[]> {
  const url = `${getApiUrl()}/api/v1/admin/inbox/available-grants`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
  return asList<AvailableGrant>((await res.json()) ?? {}, "grants");
}

/** Register an existing grant to a tenant (seals + writes the Core config). */
export async function adoptGrant(params: {
  tenant_id: string;
  grant_id?: string;
  email?: string;
}): Promise<{ status: string; grant_id: string; email: string; tenant_id: string }> {
  const url = `${getApiUrl()}/api/v1/admin/inbox/connections`;
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(params),
  });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
  return res.json();
}

// ── Messages ──────────────────────────────────────────────────────────

export async function fetchMessages(params: {
  tenant_id: string;
  /** Scope to a single mailbox (grant) within the tenant. */
  grant_id?: string;
  limit?: number;
}): Promise<EmailEvent[]> {
  const qs = new URLSearchParams();
  qs.set("tenant_id", params.tenant_id);
  if (params.grant_id) qs.set("grant_id", params.grant_id);
  if (params.limit) qs.set("limit", String(params.limit));
  const url = `${getApiUrl()}/api/v1/admin/inbox/messages?${qs.toString()}`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
  const data = (await res.json()) ?? {};
  return asList<EmailEvent>(data, "messages");
}

// ── Actions ───────────────────────────────────────────────────────────

export async function triage(req: TriageRequest): Promise<void> {
  const url = `${getApiUrl()}/api/v1/admin/inbox/triage`;
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
}

export async function draft(req: DraftRequest): Promise<DraftResponse> {
  const url = `${getApiUrl()}/api/v1/admin/inbox/draft`;
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
  return res.json();
}

/**
 * Generate an AI reply draft grounded in the thread + the contact's prior
 * threads (read server-side from Core events). Returns an editable body — it does
 * NOT write an event; the operator edits, then draft()/send() persists. 503 when
 * ANTHROPIC_API_KEY is unset on the Control Plane.
 */
export async function generateDraft(req: GenerateDraftRequest): Promise<GenerateDraftResponse> {
  const url = `${getApiUrl()}/api/v1/admin/inbox/draft/generate`;
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
  return res.json();
}

export async function send(req: SendRequest): Promise<SendResponse> {
  const url = `${getApiUrl()}/api/v1/admin/inbox/send`;
  const res = await fetch(url, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ ...req, confirm: true }),
  });
  if (!res.ok) {
    throw new ApiError(await readError(res), res.status);
  }
  return res.json();
}

// ── Payload accessors + thread grouping (presentation only) ───────────

const str = (v: unknown): string | undefined => (typeof v === "string" ? v : undefined);

function asAddress(v: unknown): EmailAddress | undefined {
  if (v && typeof v === "object" && typeof (v as EmailAddress).email === "string") {
    return v as EmailAddress;
  }
  return undefined;
}

function asAddresses(v: unknown): EmailAddress[] {
  if (!Array.isArray(v)) return [];
  return v.map(asAddress).filter((a): a is EmailAddress => a !== undefined);
}

/** Render an address as "Name <email>" or just the email. */
export function formatAddress(a?: EmailAddress): string {
  if (!a) return "—";
  return a.name ? `${a.name} <${a.email}>` : a.email;
}

/** The thread id an event belongs to (payload.thread_id, falling back to entity). */
export function threadIdOf(e: EmailEvent): string {
  return str(e.payload?.thread_id) || e.entity_id;
}

/** A flattened, render-ready view of one email.* event. */
export interface EmailMessageView {
  id: string;
  event: EmailEvent;
  type: "received" | "sent" | "triaged" | "drafted" | "other";
  /** Inbound (received) vs outbound (sent/drafted). */
  direction: "inbound" | "outbound" | "none";
  threadId: string;
  /** message_id for received/sent/triaged; draft_id for drafted. */
  messageId: string;
  subject?: string;
  snippet?: string;
  body?: string;
  from?: EmailAddress;
  to: EmailAddress[];
  timestamp: string;
  /** For triaged events. */
  label?: TriageLabel;
  /** For drafted events. */
  draftId?: string;
  intent?: string;
  by?: string;
}

const KNOWN_LABELS: TriageLabel[] = ["needs-reply", "fyi", "spam", "archive"];

function asLabel(v: unknown): TriageLabel | undefined {
  return typeof v === "string" && (KNOWN_LABELS as string[]).includes(v)
    ? (v as TriageLabel)
    : undefined;
}

/** Map a raw email.* event onto a render-ready view. */
export function toMessageView(e: EmailEvent): EmailMessageView {
  const p = e.payload ?? {};
  const base = {
    id: e.id,
    event: e,
    threadId: threadIdOf(e),
    messageId: e.entity_id,
    timestamp: e.timestamp,
    to: asAddresses(p.to),
    by: str(p.by),
  };
  switch (e.event_type) {
    case "email.received":
      return {
        ...base,
        type: "received",
        direction: "inbound",
        subject: str(p.subject),
        snippet: str(p.snippet),
        body: str(p.body),
        from: asAddress(p.from),
      };
    case "email.sent":
      return {
        ...base,
        type: "sent",
        direction: "outbound",
        subject: str(p.subject),
      };
    case "email.triaged":
      return {
        ...base,
        type: "triaged",
        direction: "none",
        label: asLabel(p.label),
      };
    case "email.drafted":
      return {
        ...base,
        type: "drafted",
        direction: "outbound",
        subject: str(p.subject),
        body: str(p.body),
        intent: str(p.intent),
        draftId: e.entity_id,
      };
    default:
      return { ...base, type: "other", direction: "none" };
  }
}

/** A thread: its message/draft/sent events plus the latest triage label. */
export interface EmailThread {
  threadId: string;
  /** Best-effort subject (first non-empty across the thread's events). */
  subject: string;
  /** The thread's events, oldest → newest for display. */
  messages: EmailMessageView[];
  /** The most recent triage label applied to any message in the thread. */
  currentLabel?: TriageLabel;
  /** Newest event timestamp in the thread (drives thread ordering). */
  lastActivity: string;
  /** Total non-triage messages (received/sent/drafted). */
  messageCount: number;
}

/**
 * Group a flat (newest-first) list of email.* events into threads, newest thread
 * first. Triage events are folded into their thread's `currentLabel` (latest
 * wins) rather than shown as standalone rows.
 */
export function groupIntoThreads(events: EmailEvent[]): EmailThread[] {
  const byThread = new Map<string, EmailMessageView[]>();
  for (const e of events) {
    const v = toMessageView(e);
    const arr = byThread.get(v.threadId) ?? [];
    arr.push(v);
    byThread.set(v.threadId, arr);
  }

  const threads: EmailThread[] = [];
  for (const [threadId, views] of byThread) {
    // Oldest → newest within the thread for natural reading order.
    const sorted = [...views].sort((a, b) => a.timestamp.localeCompare(b.timestamp));
    const messages = sorted.filter((v) => v.type !== "triaged");
    const triages = sorted.filter((v) => v.type === "triaged");
    const currentLabel = triages.at(-1)?.label;
    const subject =
      messages.find((v) => v.subject && v.subject.trim() !== "")?.subject || "(no subject)";
    const lastActivity = sorted.at(-1)?.timestamp ?? "";
    threads.push({
      threadId,
      subject,
      messages,
      currentLabel,
      lastActivity,
      messageCount: messages.length,
    });
  }

  // Newest thread first.
  threads.sort((a, b) => b.lastActivity.localeCompare(a.lastActivity));
  return threads;
}

/** Human label for a triage value. */
export function triageLabelText(label: TriageLabel): string {
  switch (label) {
    case "needs-reply":
      return "Needs reply";
    case "fyi":
      return "FYI";
    case "spam":
      return "Spam";
    case "archive":
      return "Archive";
    default:
      return label;
  }
}
