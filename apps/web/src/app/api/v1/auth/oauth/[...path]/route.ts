import { type NextRequest, NextResponse } from "next/server";

/**
 * Runtime proxy for OAuth requests to the control plane.
 *
 * Previously this was a Next.js rewrite in next.config.mjs, but rewrites are
 * evaluated at build time — so CONTROL_PLANE_INTERNAL_URL was baked in as
 * localhost on Vercel. This route handler reads the env var at request time.
 */

function getControlPlaneUrl(): string {
  return process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3901";
}

async function proxyToControlPlane(
  request: NextRequest,
  path: string
): Promise<NextResponse> {
  const url = new URL(`/api/v1/auth/oauth/${path}`, getControlPlaneUrl());
  // Forward query params (e.g. ?code=...&state=... on OAuth callback)
  request.nextUrl.searchParams.forEach((value, key) => {
    url.searchParams.set(key, value);
  });

  // Only forward headers the control plane needs. Passing the original Host
  // header (www.all-source.xyz) causes Fly.io to misroute the request, and
  // Vercel internal headers can confuse downstream services.
  const headers: Record<string, string> = {};
  const cookie = request.headers.get("cookie");
  if (cookie) {
    headers["cookie"] = cookie; // CSRF state cookie for OAuth callback
  }

  const response = await fetch(url.toString(), {
    method: request.method,
    headers,
    redirect: "manual",
  });

  // The control plane responds with redirects (to Google/GitHub, then back).
  // Forward them directly to the browser.
  if (response.status >= 300 && response.status < 400) {
    const location = response.headers.get("location");
    if (location) {
      const res = NextResponse.redirect(location, response.status);
      // Forward set-cookie headers (CSRF state cookie)
      const setCookies = response.headers.getSetCookie();
      for (const cookie of setCookies) {
        res.headers.append("set-cookie", cookie);
      }
      return res;
    }
  }

  // For non-redirect responses, pass through body and status
  const body = await response.text();
  return new NextResponse(body, {
    status: response.status,
    headers: {
      "content-type": response.headers.get("content-type") || "text/plain",
    },
  });
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToControlPlane(request, path.join("/"));
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  return proxyToControlPlane(request, path.join("/"));
}
