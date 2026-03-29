import { type NextRequest, NextResponse } from "next/server";

/**
 * Proxy for team management requests to the Control Plane.
 *
 * Path mapping (Next.js → Control Plane):
 *   GET  /api/team/members           → GET  /api/v1/teams/members
 *   POST /api/team/invite            → POST /api/v1/teams/invite
 *   DEL  /api/team/members/:id       → DEL  /api/v1/teams/members/:id
 *   PUT  /api/team/members/:id/role  → PUT  /api/v1/teams/members/:id
 *
 * The "/role" suffix is stripped before forwarding so the CP route matches.
 */

function getControlPlaneUrl(): string {
  return process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3903";
}

async function proxyToControlPlane(request: NextRequest, pathSegments: string[]): Promise<NextResponse> {
  // Strip trailing "role" segment — client uses /members/:id/role but CP uses /members/:id
  const segments =
    pathSegments.at(-1) === "role" ? pathSegments.slice(0, -1) : pathSegments;

  const cpPath = `/api/v1/teams/${segments.join("/")}`;
  const url = new URL(cpPath, getControlPlaneUrl());

  request.nextUrl.searchParams.forEach((value, key) => {
    url.searchParams.set(key, value);
  });

  const headers: Record<string, string> = {
    "content-type": request.headers.get("content-type") || "application/json",
  };

  // Forward cookies so CP can read other cookie-based state.
  const cookie = request.headers.get("cookie");
  if (cookie) {
    headers.cookie = cookie;
  }

  // CP's AuthMiddleware reads only the Authorization header, not cookies.
  // Extract auth_token from the cookie string and promote it to a Bearer token.
  // Fall back to a forwarded Authorization header if the client sent one directly.
  const authorization = request.headers.get("authorization");
  if (authorization) {
    headers.authorization = authorization;
  } else if (cookie) {
    const authToken = cookie
      .split(";")
      .map((s) => s.trim())
      .find((s) => s.startsWith("auth_token="))
      ?.split("=")
      .slice(1)
      .join("="); // handle = inside base64 values
    if (authToken) {
      headers.authorization = `Bearer ${authToken}`;
    }
  }

  const fetchOptions: RequestInit = {
    method: request.method,
    headers,
  };

  if (["POST", "PUT", "PATCH"].includes(request.method)) {
    fetchOptions.body = await request.text();
  }

  try {
    const response = await fetch(url.toString(), fetchOptions);
    const body = await response.text();

    const responseHeaders: Record<string, string> = {
      "content-type": response.headers.get("content-type") || "application/json",
    };

    return new NextResponse(body, {
      status: response.status,
      headers: responseHeaders,
    });
  } catch (_error) {
    return NextResponse.json({ error: "Failed to reach Control Plane" }, { status: 502 });
  }
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToControlPlane(request, path);
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToControlPlane(request, path);
}

export async function PUT(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToControlPlane(request, path);
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToControlPlane(request, path);
}
