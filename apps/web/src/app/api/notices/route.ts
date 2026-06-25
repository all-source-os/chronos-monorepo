import { type NextRequest, NextResponse } from "next/server";

/**
 * Runtime proxy for the tenant-facing notices read (ADMIN_TENANT_POWER_TOOL
 * §4 Pillar C / §9 Phase 6 — the web half).
 *
 * The notices endpoints live on the **Control Plane** (Go, port 3901), NOT the
 * Query Service. The catch-all /api/v1/[...path] route forwards to the Query
 * Service (NEXT_PUBLIC_API_URL), which does not serve /api/v1/notices — so this
 * dedicated route mirrors the existing CP proxies (/api/v1/auth/[...path],
 * /api/v1/demo/start) and reads CONTROL_PLANE_INTERNAL_URL at request time.
 *
 * The CP's regular AuthMiddleware extracts identity from `Authorization: Bearer`
 * (auth.go ExtractToken) — it does NOT read cookies. So this route reads the
 * httpOnly `auth_token` cookie server-side and attaches it as a Bearer header,
 * exactly like /api/auth/session does. The CP then scopes the read to the
 * caller's own tenant from the JWT (`auth_tenant_id`); the client never sees or
 * sends a tenant id.
 *
 * Maps to: GET /api/v1/notices on the Control Plane →
 *   { notices: [{ id, tenant_id, title, body, severity, created_at,
 *                 expires_at?, dismissed? }], count }
 */

function getControlPlaneUrl(): string {
  return process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3901";
}

export async function GET(request: NextRequest): Promise<NextResponse> {
  const token = request.cookies.get("auth_token")?.value;
  // No session → no notices. Render-nothing path on the client; never a crash.
  if (!token) {
    return NextResponse.json({ notices: [], count: 0 }, { status: 200 });
  }

  const url = new URL("/api/v1/notices", getControlPlaneUrl());

  try {
    const response = await fetch(url.toString(), {
      method: "GET",
      headers: {
        "content-type": "application/json",
        authorization: `Bearer ${token}`,
      },
    });

    const body = await response.text();
    return new NextResponse(body, {
      status: response.status,
      headers: { "content-type": "application/json" },
    });
  } catch {
    // Endpoint unreachable → degrade gracefully (empty, never a 5xx the banner
    // would have to special-case). The dashboard must never be blocked by this.
    return NextResponse.json({ notices: [], count: 0 }, { status: 200 });
  }
}
