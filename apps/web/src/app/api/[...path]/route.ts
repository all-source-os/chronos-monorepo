import { type NextRequest, NextResponse } from "next/server";

/**
 * Catch-all proxy for Query Service API requests.
 *
 * Routes all /api/* requests (that aren't handled by more specific routes like
 * /api/v1/*, /api/auth/*, /api/status/*) to the Query Service backend.
 *
 * This eliminates CORS issues by keeping all browser requests same-origin.
 * The NEXT_PUBLIC_API_URL env var is read at request time (not build time),
 * so it works correctly on Vercel and other platforms.
 */

function getQueryServiceUrl(): string {
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

async function proxyToQueryService(request: NextRequest, path: string): Promise<NextResponse> {
  const url = new URL(`/api/${path}`, getQueryServiceUrl());

  // Forward query params
  request.nextUrl.searchParams.forEach((value, key) => {
    url.searchParams.set(key, value);
  });

  const headers: Record<string, string> = {
    "content-type": request.headers.get("content-type") || "application/json",
  };

  // Forward auth headers
  const authorization = request.headers.get("authorization");
  if (authorization) {
    headers.authorization = authorization;
  }
  const cookie = request.headers.get("cookie");
  if (cookie) {
    headers.cookie = cookie;
  }

  const fetchOptions: RequestInit = {
    method: request.method,
    headers,
  };

  // Forward body for POST/PUT/PATCH/DELETE
  if (["POST", "PUT", "PATCH", "DELETE"].includes(request.method) && request.body) {
    fetchOptions.body = await request.text();
  }

  try {
    const response = await fetch(url.toString(), fetchOptions);
    const body = await response.text();

    // Forward set-cookie headers from backend
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
    return NextResponse.json({ error: "Failed to reach Query Service" }, { status: 502 });
  }
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToQueryService(request, path.join("/"));
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToQueryService(request, path.join("/"));
}

export async function PUT(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToQueryService(request, path.join("/"));
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToQueryService(request, path.join("/"));
}

export async function PATCH(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToQueryService(request, path.join("/"));
}
