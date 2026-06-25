import { type NextRequest, NextResponse } from "next/server";

/**
 * Runtime proxy for the tenant-facing notice dismiss (ADMIN_TENANT_POWER_TOOL
 * §4 Pillar C / §9 Phase 6 — the web half).
 *
 * Mirrors the sibling /api/notices read: the dismiss endpoint lives on the
 * Control Plane (Go, port 3901), so this dedicated route forwards there with
 * the httpOnly `auth_token` cookie attached as a Bearer header (the CP's
 * AuthMiddleware is Bearer-only — it ignores cookies). The CP scopes the
 * dismiss to the caller's own tenant from the JWT and records the dismissal
 * as a durable `admin.notice.dismissed` Core event.
 *
 * Maps to: POST /api/v1/notices/:id/dismiss on the Control Plane →
 *   { dismissed: true, notice_id }
 */

function getControlPlaneUrl(): string {
  return process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3901";
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> }
): Promise<NextResponse> {
  const { id } = await params;
  const token = request.cookies.get("auth_token")?.value;
  if (!token) {
    return NextResponse.json(
      { error: { code: "not_authenticated", message: "No session found" } },
      { status: 401 }
    );
  }

  const url = new URL(`/api/v1/notices/${encodeURIComponent(id)}/dismiss`, getControlPlaneUrl());

  try {
    const response = await fetch(url.toString(), {
      method: "POST",
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
    return NextResponse.json(
      { error: { code: "control_plane_unreachable", message: "Failed to reach Control Plane" } },
      { status: 502 }
    );
  }
}
