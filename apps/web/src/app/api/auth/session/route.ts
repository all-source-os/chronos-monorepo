import { type NextRequest, NextResponse } from "next/server";

// Session validation must hit the Query Service, which serves /api/auth/me.
// NEXT_PUBLIC_API_URL is the branded gateway (api.all-source.xyz) and does NOT
// route /api/auth/* — fetching /api/auth/me there 404s, which this route treats
// as an invalid token and DELETES the auth_token cookie, silently logging the
// user out on every dashboard load. Hit the QS directly.
function getApiUrl(): string {
  return (
    process.env.QUERY_SERVICE_URL ||
    (process.env.NODE_ENV === "production"
      ? "https://allsource-query.fly.dev"
      : "http://localhost:3902")
  );
}

/**
 * Decode a JWT payload without verifying the signature.
 * Safe to use server-side because the token was already validated by the backend.
 */
function decodeJwtPayload(token: string): Record<string, unknown> {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return {};
    const payload = Buffer.from(parts[1] ?? "", "base64url").toString("utf-8");
    return JSON.parse(payload) as Record<string, unknown>;
  } catch {
    return {};
  }
}

// GET /api/auth/session - Get current user session
export async function GET(request: NextRequest) {
  const token = request.cookies.get("auth_token")?.value;

  if (!token) {
    return NextResponse.json(
      { error: { code: "not_authenticated", message: "No session found" } },
      { status: 401 }
    );
  }

  try {
    const apiUrl = getApiUrl();

    const requestOptions = {
      headers: { Authorization: `Bearer ${token}` },
      cache: "no-store" as const,
    };

    // These reads are independent. Running them together removes one backend
    // round trip from every cold dashboard load.
    const [meResponse, tenantResponse] = await Promise.all([
      fetch(`${apiUrl}/api/auth/me`, requestOptions),
      fetch(`${apiUrl}/api/tenant`, requestOptions),
    ]);

    if (!meResponse.ok) {
      if (meResponse.status !== 401 && meResponse.status !== 403) {
        return NextResponse.json(
          {
            error: {
              code: "session_unavailable",
              message: "Session service is temporarily unavailable",
            },
          },
          { status: 503 }
        );
      }

      const response = NextResponse.json(
        { error: { code: "invalid_session", message: "Session expired" } },
        { status: 401 }
      );
      response.cookies.delete("auth_token");
      return response;
    }

    const userData = await meResponse.json();

    let tenantData = null;
    if (tenantResponse.ok) {
      tenantData = await tenantResponse.json();
    }

    // Extract core_api_key from JWT payload (set by CP during OAuth login).
    const jwtPayload = decodeJwtPayload(token);
    const coreApiKey = typeof jwtPayload.core_api_key === "string" ? jwtPayload.core_api_key : null;

    return NextResponse.json({
      data: {
        user: userData.data?.user || userData.data,
        tenant: tenantData?.data || null,
        core_api_key: coreApiKey,
      },
    });
  } catch {
    return NextResponse.json(
      { error: { code: "session_error", message: "Failed to fetch session" } },
      { status: 500 }
    );
  }
}

// DELETE /api/auth/session - Logout
export async function DELETE(request: NextRequest) {
  const token = request.cookies.get("auth_token")?.value;

  if (token) {
    // Call backend logout to revoke token
    try {
      await fetch(`${getApiUrl()}/api/auth/logout`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });
    } catch {
      // Ignore errors, we'll clear the cookie anyway
    }
  }

  // Clear the auth cookie. Write an explicit expired Set-Cookie with the SAME
  // attributes the callback used to set it (path:"/", sameSite:lax, secure in
  // prod). `cookies.delete(name)` alone can emit an attribute-mismatched
  // Set-Cookie that some browsers refuse to apply, leaving the user logged in
  // after "Log out".
  const response = NextResponse.json({ data: { success: true } });
  response.cookies.set("auth_token", "", {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    maxAge: 0,
    expires: new Date(0),
  });
  return response;
}
