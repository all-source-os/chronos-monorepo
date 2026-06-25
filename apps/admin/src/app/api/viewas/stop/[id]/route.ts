import { type NextRequest, NextResponse } from "next/server";
import { getControlPlaneUrl } from "@/lib/auth";

/**
 * Server-side view-as TEARDOWN (ADMIN_TENANT_POWER_TOOL §5.3 / §9 Phase 7).
 *
 * POST /api/viewas/stop/:id   body: { reason?: "exit" | "expired" }
 *
 * Called by BOTH the one-click Exit AND the auto-expiry path. It:
 *   1. calls the CP teardown — POST /api/v1/admin/tenants/:id/view-as/stop —
 *      with the admin Bearer, so the CP writes admin.viewas.stopped (every
 *      started has a paired stopped in the audit — §5.4);
 *   2. clears the SEPARATE `viewas_token` cookie unconditionally;
 *   3. leaves the `admin_token` session cookie untouched — the operator returns
 *      to the normal admin frame still logged in.
 *
 * The cookie is cleared even if the CP call fails (best-effort audit, but the
 * local view-as session MUST end). The CP stop endpoint is idempotent (it only
 * writes the audit event — the token is stateless + short-TTL, nothing to revoke),
 * so the auto-expiry call is safe to make after the token has already lapsed.
 */

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> }
): Promise<NextResponse> {
  const { id } = await params;

  let reason = "exit";
  try {
    const body = await request.json();
    if (body && typeof body.reason === "string" && body.reason) {
      reason = body.reason;
    }
  } catch {
    // No/invalid body — default to "exit".
  }

  const adminToken = request.cookies.get("admin_token")?.value;

  // Best-effort audit teardown via the CP (writes admin.viewas.stopped). We need
  // the admin Bearer for this — the stop endpoint is admin-gated. If the admin
  // session is gone we still clear the local cookie below.
  if (adminToken) {
    const url = new URL(
      `/api/v1/admin/tenants/${encodeURIComponent(id)}/view-as/stop`,
      getControlPlaneUrl()
    );
    try {
      await fetch(url.toString(), {
        method: "POST",
        headers: {
          authorization: `Bearer ${adminToken}`,
          "content-type": "application/json",
        },
        body: JSON.stringify({ reason }),
      });
    } catch {
      // Ignore — the local cookie is cleared regardless so the frame dies.
    }
  }

  const response = NextResponse.json({ data: { stopped: true, tenant_id: id, reason } });
  // Clear ONLY the view-as cookie — admin_token is untouched.
  response.cookies.delete("viewas_token");
  return response;
}
