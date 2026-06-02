import { type NextRequest, NextResponse } from "next/server";

/**
 * GET /api/auth/ws-token — surface the session JWT for the Phoenix WebSocket.
 *
 * Why this exists: the `auth_token` cookie is httpOnly, so browser JS cannot
 * read it. The Phoenix Live Feed socket (Query Service `UserSocket.connect/3`)
 * authenticates with that exact JWT via a `token` connect param — it verifies
 * the HS256 signature with the shared `JWT_SECRET` and reads the `sub` / `exp`
 * / `tenant_id` claims. The REST `/api/auth/session` route intentionally does
 * NOT echo the raw token (it returns user/tenant/core_api_key only), so this
 * dedicated endpoint exposes the JWT solely for the socket handshake.
 *
 * Security: the token is the same short-lived (7-day) session JWT already held
 * in the cookie; this only hands the logged-in user back their own token over
 * a same-origin authenticated request. We do not weaken the socket's auth — the
 * Query Service still verifies the signature and expiry on every connect.
 */
export async function GET(request: NextRequest) {
  const token = request.cookies.get("auth_token")?.value;

  if (!token) {
    return NextResponse.json(
      { error: { code: "not_authenticated", message: "No session found" } },
      { status: 401 }
    );
  }

  return NextResponse.json({ token });
}
