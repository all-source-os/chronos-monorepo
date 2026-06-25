import { type NextRequest, NextResponse } from "next/server";
import { decodeJwt } from "@/lib/auth";

/**
 * View-as SESSION STATUS (ADMIN_TENANT_POWER_TOOL §5.3).
 *
 * GET /api/viewas/status
 *
 * Reads the httpOnly `viewas_token` cookie (which client JS cannot read) and
 * returns the active view-as session metadata — WITHOUT ever returning the raw
 * token. The banner/frame use this to hydrate after a reload and to recover the
 * tenant id / expiry / `view_as` marker.
 *
 * Validation is structural only: `decodeJwt` (the Edge-safe atob+TextDecoder
 * decoder, reused from lib/auth.ts) returns null for an expired or malformed
 * token, in which case there is no active view-as session. The CP is the real
 * authority on the token; this is just enough to drive the banner.
 *
 * `active: false` means: no view-as session — render the normal admin frame.
 */

interface ViewAsClaims {
  sub?: string;
  tenant_id?: string;
  role?: string;
  view_as?: boolean;
  act_as?: string;
  exp?: number;
}

export async function GET(request: NextRequest): Promise<NextResponse> {
  const token = request.cookies.get("viewas_token")?.value;

  if (!token) {
    return NextResponse.json({ data: { active: false } });
  }

  // decodeJwt returns null for expired/malformed tokens (it checks exp), so an
  // auto-expired token reads as "no active session" here too.
  const payload = decodeJwt(token) as (ViewAsClaims & Record<string, unknown>) | null;

  // Defense in depth: a token in the viewas_token cookie MUST carry view_as:true.
  // Anything else is not a view-as session (and must never be treated as one).
  if (!payload || payload.view_as !== true || !payload.tenant_id) {
    return NextResponse.json({ data: { active: false } });
  }

  return NextResponse.json({
    data: {
      active: true,
      tenant_id: payload.tenant_id,
      role: payload.role ?? "readonly",
      view_as: true,
      // act_as is the admin behind the view (audit trail); sub is the same admin.
      act_as: payload.act_as ?? payload.sub ?? "",
      expires_at: payload.exp ?? 0,
    },
  });
}
