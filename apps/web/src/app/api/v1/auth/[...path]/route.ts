import { type NextRequest, NextResponse } from "next/server";

/**
 * Runtime proxy for auth requests to the Auth Service (better-auth + AllSource).
 *
 * The Auth Service runs better-auth-rs with the AllSource adapter.
 * Routes: /api/auth/sign-in/email, /api/auth/sign-up/email, /api/auth/callback/:provider, etc.
 *
 * Falls back to Control Plane if AUTH_SERVICE_URL is not set (migration compat).
 */

function getAuthServiceUrl(): string {
  return process.env.AUTH_SERVICE_URL || process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3903";
}

async function proxyToAuthService(request: NextRequest, path: string): Promise<NextResponse> {
  const url = new URL(`/api/auth/${path}`, getAuthServiceUrl());

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
