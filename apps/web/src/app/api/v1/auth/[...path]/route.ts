import { type NextRequest, NextResponse } from "next/server";

/**
 * Runtime proxy for auth requests.
 *
 * When AUTH_SERVICE_URL is set, routes go to the better-auth-rs service
 * at /api/auth/{path}. Otherwise, falls back to the Control Plane (Go)
 * at /api/v1/auth/{path} — note the different path prefix.
 */

function getAuthBackend(): { url: string; pathPrefix: string } {
  const authService = process.env.AUTH_SERVICE_URL;
  if (authService) {
    return { url: authService, pathPrefix: "/api/auth" };
  }
  const controlPlane = process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3901";
  return { url: controlPlane, pathPrefix: "/api/v1/auth" };
}

async function proxyToAuthService(request: NextRequest, path: string): Promise<NextResponse> {
  const backend = getAuthBackend();
  const url = new URL(`${backend.pathPrefix}/${path}`, backend.url);

  request.nextUrl.searchParams.forEach((value, key) => {
    url.searchParams.set(key, value);
  });

  const headers: Record<string, string> = {
    "content-type": request.headers.get("content-type") || "application/json",
  };

  const cookie = request.headers.get("cookie");
  if (cookie) {
    headers.cookie = cookie;
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
    const setCookie = response.headers.get("set-cookie");
    if (setCookie) {
      responseHeaders["set-cookie"] = setCookie;
    }

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
  return proxyToAuthService(request, path.join("/"));
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToAuthService(request, path.join("/"));
}

export async function PUT(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToAuthService(request, path.join("/"));
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToAuthService(request, path.join("/"));
}
