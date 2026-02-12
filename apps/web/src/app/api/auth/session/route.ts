import { type NextRequest, NextResponse } from "next/server";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";

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
    // Fetch user info from backend
    const meResponse = await fetch(`${API_URL}/api/auth/me`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });

    if (!meResponse.ok) {
      // Token is invalid, clear cookie
      const response = NextResponse.json(
        { error: { code: "invalid_session", message: "Session expired" } },
        { status: 401 }
      );
      response.cookies.delete("auth_token");
      return response;
    }

    const userData = await meResponse.json();

    // Also fetch tenant info
    const tenantResponse = await fetch(`${API_URL}/api/tenant`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });

    let tenantData = null;
    if (tenantResponse.ok) {
      tenantData = await tenantResponse.json();
    }

    return NextResponse.json({
      data: {
        user: userData.data,
        tenant: tenantData?.data || null,
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
      await fetch(`${API_URL}/api/auth/logout`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });
    } catch {
      // Ignore errors, we'll clear the cookie anyway
    }
  }

  // Clear the auth cookie
  const response = NextResponse.json({ data: { success: true } });
  response.cookies.delete("auth_token");
  return response;
}
