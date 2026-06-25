/**
 * View-as (read-only impersonation) client (ADMIN_TENANT_POWER_TOOL §5 / §9
 * Phase 7 admin half).
 *
 * The view-as TOKEN is minted + held server-side in a SEPARATE httpOnly
 * `viewas_token` cookie (src/app/api/viewas/start/[id]/route.ts). Client JS never
 * sees the token — these functions only talk to the admin app's own same-origin
 * view-as routes and receive banner metadata:
 *
 *   start  → POST /api/viewas/start/:id   (server mints + sets viewas_token)
 *   stop   → POST /api/viewas/stop/:id    (server tears down + clears viewas_token)
 *   status → GET  /api/viewas/status      (server decodes the cookie → metadata)
 *
 * Reads inside the read-only frame go through GET /api/viewas/data/* (the
 * read-only data proxy that attaches the viewas_token Bearer) — see the frame page.
 *
 * Every field returned here is guarded (§6): the banner coalesces a missing
 * expiry/name so a render can never crash on undefined.
 */

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    // Client-side: same-origin admin routes (they set/clear the viewas_token cookie).
    return "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

/**
 * Banner/session metadata for an active view-as session. Intentionally does NOT
 * include the raw token — that stays in the httpOnly cookie, server-side only.
 */
export interface ViewAsSession {
  tenant_id: string;
  tenant_name: string;
  /** Always "readonly". */
  role: string;
  /** Always true for a real view-as session. */
  view_as: boolean;
  /** Unix seconds — the banner shows the countdown from this. */
  expires_at: number;
  /** The token time-box length in seconds (~900). Optional on status reads. */
  ttl_seconds?: number;
}

/** Teardown reasons, paired with the CP audit (`exit` | `expired`). */
export type ViewAsStopReason = "exit" | "expired";

/**
 * Start a read-only view-as session for a tenant. The server reads the admin
 * session cookie, mints the scoped token via the CP, sets the SEPARATE
 * viewas_token cookie, and returns just the banner metadata (no token).
 */
export async function startViewAs(tenantId: string): Promise<ViewAsSession> {
  const res = await fetch(`${getApiUrl()}/api/viewas/start/${encodeURIComponent(tenantId)}`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: "{}",
  });
  if (!res.ok) {
    throw new Error(await viewAsError(res, "start view-as"));
  }
  const json = await res.json();
  const d = (json?.data ?? {}) as Partial<ViewAsSession>;
  return {
    tenant_id: d.tenant_id ?? tenantId,
    tenant_name: d.tenant_name ?? "",
    role: d.role ?? "readonly",
    view_as: d.view_as ?? true,
    expires_at: d.expires_at ?? 0,
    ttl_seconds: d.ttl_seconds,
  };
}

/**
 * Tear down the current view-as session. Called by the one-click Exit (reason
 * "exit") AND the auto-expiry path (reason "expired"). The server writes the CP
 * audit (admin.viewas.stopped) and clears the viewas_token cookie; the admin
 * session is untouched.
 */
export async function stopViewAs(
  tenantId: string,
  reason: ViewAsStopReason = "exit"
): Promise<void> {
  const res = await fetch(`${getApiUrl()}/api/viewas/stop/${encodeURIComponent(tenantId)}`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ reason }),
  });
  if (!res.ok) {
    // Stop is best-effort teardown — surface the error but the cookie is cleared
    // server-side regardless, so the caller can still navigate away.
    throw new Error(await viewAsError(res, "stop view-as"));
  }
}

/**
 * Read the current view-as session (or null if none/expired). Drives the banner
 * on load and on reload. Never throws on a missing session — returns null.
 */
export async function getViewAsSession(): Promise<ViewAsSession | null> {
  try {
    const res = await fetch(`${getApiUrl()}/api/viewas/status`, {
      credentials: "include",
      cache: "no-store",
    });
    if (!res.ok) return null;
    const json = await res.json();
    const d = json?.data;
    if (!d || d.active !== true || !d.tenant_id) return null;
    return {
      tenant_id: d.tenant_id,
      tenant_name: d.tenant_name ?? "",
      role: d.role ?? "readonly",
      view_as: true,
      expires_at: d.expires_at ?? 0,
    };
  } catch {
    return null;
  }
}

/**
 * Build an error message from a non-OK view-as response, preferring the CP/route
 * JSON `{error}` body so the entry point surfaces the real cause (e.g. a 404 for
 * an unknown tenant, or a 503 "view-as not configured") instead of a bare status.
 */
async function viewAsError(res: Response, action: string): Promise<string> {
  try {
    const data = await res.json();
    const err = data?.error;
    if (typeof err === "string") return `Failed to ${action}: ${err}`;
    if (err && typeof err === "object" && typeof err.message === "string") {
      return `Failed to ${action}: ${err.message}`;
    }
  } catch {
    // fall through
  }
  return `Failed to ${action}: ${res.status}`;
}
