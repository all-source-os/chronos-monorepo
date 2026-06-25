import { type NextRequest, NextResponse } from "next/server";
import { getControlPlaneUrl } from "@/lib/auth";

/**
 * Read-only view-as DATA PROXY (ADMIN_TENANT_POWER_TOOL §5.2 layer 3).
 *
 * GET /api/viewas/data/*  →  GET {ControlPlane}/api/v1/*  with the view-as Bearer
 *
 * This is the read path for the read-only product frame. It mirrors the admin BFF
 * (src/app/api/v1/[...path]/route.ts) but:
 *   - attaches the SEPARATE `viewas_token` cookie (the readonly+view_as token),
 *     NOT the admin_token — so reads are scoped to the viewed tenant by the token
 *     itself (the CP authorizes the readonly role against the token's tenant_id);
 *   - ONLY proxies GET. POST/PUT/PATCH/DELETE are hard-refused at this boundary
 *     (405) — read-only BY CONSTRUCTION on the surface, on top of the role
 *     (readonly can't reach write endpoints) and the CP's view_as write-refusal.
 *     Three independent layers; this is the cheapest one (the request never even
 *     leaves the admin origin).
 *
 * The frame is read-only, so there is intentionally NO mutating method handler.
 */

async function proxyRead(request: NextRequest, path: string): Promise<NextResponse> {
  const token = request.cookies.get("viewas_token")?.value;
  if (!token) {
    // No view-as session (expired/torn-down) → 401 so the frame knows it is dead
    // and returns to the admin app (auto-expiry backstop, §5.3).
    return NextResponse.json(
      { error: { code: "viewas_inactive", message: "No active view-as session." } },
      { status: 401 }
    );
  }

  const url = new URL(`/api/v1/${path}`, getControlPlaneUrl());
  request.nextUrl.searchParams.forEach((value, key) => {
    url.searchParams.set(key, value);
  });

  try {
    const response = await fetch(url.toString(), {
      method: "GET",
      headers: { authorization: `Bearer ${token}` },
    });
    const body = await response.text();
    return new NextResponse(body, {
      status: response.status,
      headers: {
        "content-type": response.headers.get("content-type") || "application/json",
      },
    });
  } catch {
    return NextResponse.json(
      { error: { code: "upstream_unreachable", message: "Failed to reach Control Plane." } },
      { status: 502 }
    );
  }
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyRead(request, path.join("/"));
}

// No POST/PUT/PATCH/DELETE — the view-as frame is read-only. A mutating request
// to this proxy is rejected before it ever reaches the network (405). The CP
// independently refuses any view_as token on a mutating method (defense in depth).
const methodNotAllowed = () =>
  NextResponse.json(
    {
      error: {
        code: "read_only",
        message: "View-as is read-only — mutating requests are refused.",
      },
    },
    { status: 405, headers: { allow: "GET" } }
  );

export const POST = methodNotAllowed;
export const PUT = methodNotAllowed;
export const PATCH = methodNotAllowed;
export const DELETE = methodNotAllowed;
