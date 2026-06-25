import { type NextRequest, NextResponse } from "next/server";
import { getControlPlaneUrl } from "@/lib/auth";

/**
 * Server-side view-as MINT (ADMIN_TENANT_POWER_TOOL §5.3 / §9 Phase 7 admin half).
 *
 * POST /api/viewas/start/:id
 *
 * Reads the admin's session (the httpOnly `admin_token` cookie), calls the Control
 * Plane mint — POST /api/v1/admin/tenants/:id/view-as — with the admin Bearer, and
 * sets the returned scoped token in a SEPARATE short-lived `viewas_token` cookie.
 *
 * Why this is a DEDICATED route and not the generic BFF catch-all
 * (src/app/api/v1/[...path]/route.ts):
 *   - the generic BFF forwards the response body verbatim and never sets cookies;
 *   - the view-as token must land in a SEPARATE `viewas_token` cookie, NEVER the
 *     `admin_token` session cookie (overwriting/deriving the admin session from a
 *     view-as token is a security regression — §5.5);
 *   - the raw token must NOT be exposed to client JS. It lives only in the
 *     httpOnly cookie; this route returns just the banner metadata
 *     (tenant_id, tenant_name, expires_at, ttl_seconds, role, view_as).
 *
 * The cookie max-age is the token's own ttl_seconds (~15m) so the cookie can never
 * outlive the token — auto-expiry by construction (§5.1 "short TTL is the backstop
 * for a forgotten Exit"). The CP refuses any write carrying a view_as token, so the
 * frame is read-only by construction regardless of what the cookie is used for.
 *
 * The admin session cookie (`admin_token`) is read here but NEVER written — the
 * only Set-Cookie this route emits is the separate `viewas_token`.
 */

interface ViewAsMintResponse {
  token: string;
  tenant_id: string;
  tenant_name?: string;
  role: string;
  view_as: boolean;
  expires_at: number;
  ttl_seconds: number;
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> }
): Promise<NextResponse> {
  const { id } = await params;

  // The admin must be authenticated as an admin to mint a view-as token. The
  // route group's proxy (proxy.ts) already gated the page that called this, but
  // re-assert the admin session here so a direct POST without it can't mint.
  const adminToken = request.cookies.get("admin_token")?.value;
  if (!adminToken) {
    return NextResponse.json(
      { error: { code: "not_authenticated", message: "No admin session." } },
      { status: 401 }
    );
  }

  const url = new URL(
    `/api/v1/admin/tenants/${encodeURIComponent(id)}/view-as`,
    getControlPlaneUrl()
  );

  let cpResponse: Response;
  try {
    // Mint server-side with the admin Bearer. The CP is Bearer-only and ignores
    // cookies; the admin_token NEVER leaves this server boundary toward the client.
    cpResponse = await fetch(url.toString(), {
      method: "POST",
      headers: {
        authorization: `Bearer ${adminToken}`,
        "content-type": "application/json",
      },
      body: JSON.stringify({}),
    });
  } catch {
    return NextResponse.json(
      { error: { code: "upstream_unreachable", message: "Failed to reach Control Plane." } },
      { status: 502 }
    );
  }

  const rawBody = await cpResponse.text();
  if (!cpResponse.ok) {
    // Surface the CP's status + body so the entry point can show a real error.
    return new NextResponse(rawBody || JSON.stringify({ error: "view-as mint failed" }), {
      status: cpResponse.status,
      headers: { "content-type": "application/json" },
    });
  }

  let mint: ViewAsMintResponse;
  try {
    mint = JSON.parse(rawBody) as ViewAsMintResponse;
  } catch {
    return NextResponse.json(
      { error: { code: "bad_upstream", message: "Malformed mint response." } },
      { status: 502 }
    );
  }

  if (!mint.token || !mint.expires_at) {
    return NextResponse.json(
      { error: { code: "bad_upstream", message: "Mint response missing token/expiry." } },
      { status: 502 }
    );
  }

  // Clamp the cookie max-age to the token's own lifetime. Prefer ttl_seconds; fall
  // back to (expires_at - now) so the cookie can NEVER outlive the token. Floor at
  // 1s to keep Set-Cookie valid even on a near-expired clock skew.
  const now = Math.floor(Date.now() / 1000);
  const ttlFromExp = mint.expires_at - now;
  const maxAge = Math.max(1, mint.ttl_seconds || ttlFromExp);

  // Return ONLY the banner metadata — never the raw token — to the client.
  const response = NextResponse.json({
    data: {
      tenant_id: mint.tenant_id,
      tenant_name: mint.tenant_name ?? "",
      role: mint.role,
      view_as: mint.view_as,
      expires_at: mint.expires_at,
      ttl_seconds: mint.ttl_seconds,
    },
  });

  // SEPARATE cookie — NEVER admin_token. httpOnly so client JS cannot read the
  // token; max-age clamped to the token TTL so it auto-expires with the token.
  response.cookies.set("viewas_token", mint.token, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    maxAge,
    path: "/",
  });

  return response;
}
