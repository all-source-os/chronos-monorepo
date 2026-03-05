import { type NextRequest, NextResponse } from "next/server";

/**
 * Runtime proxy for Query Service API requests.
 *
 * Client components cannot read env vars at runtime — NEXT_PUBLIC_* vars are
 * inlined at build time. This route handler reads the Query Service URL at
 * request time, so the frontend can use relative URLs (e.g. /api/v1/demo/seed)
 * and the proxy forwards them to the correct backend.
 *
 * The more specific /api/v1/auth/oauth/[...path] route takes precedence for
 * OAuth requests — this catch-all handles everything else.
 */

function getQueryServiceUrl(): string {
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

async function proxyToQueryService(
  request: NextRequest,
  path: string
): Promise<NextResponse> {
  const url = new URL(`/api/v1/${path}`, getQueryServiceUrl());

  // Forward query params
  request.nextUrl.searchParams.forEach((value, key) => {
    url.searchParams.set(key, value);
  });

  const headers: Record<string, string> = {
    "content-type": request.headers.get("content-type") || "application/json",
  };

  // Forward auth headers if present
  const authorization = request.headers.get("authorization");
  if (authorization) {
    headers["authorization"] = authorization;
  }
  const cookie = request.headers.get("cookie");
  if (cookie) {
    headers["cookie"] = cookie;
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
  } catch (error) {
    return NextResponse.json(
      { error: "Failed to reach Query Service" },
      { status: 502 }
    );
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
