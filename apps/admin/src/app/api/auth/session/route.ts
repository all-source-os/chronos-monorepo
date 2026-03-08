import { type NextRequest, NextResponse } from "next/server";
import { validateAdminToken } from "@/lib/auth";

function getApiUrl(): string {
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

// GET /api/auth/session - Get current admin session
export async function GET(request: NextRequest) {
  const token = request.cookies.get("admin_token")?.value;

  if (!token) {
    return NextResponse.json(
      { error: { code: "not_authenticated", message: "No session found" } },
      { status: 401 }
    );
  }

  const result = await validateAdminToken(token);

  if (!result.valid) {
    const response = NextResponse.json(
      { error: { code: result.error, message: "Session invalid" } },
      { status: 401 }
    );
    response.cookies.delete("admin_token");
    return response;
  }

  return NextResponse.json({
    data: { user: result.user },
  });
}

// DELETE /api/auth/session - Logout
export async function DELETE(request: NextRequest) {
  const token = request.cookies.get("admin_token")?.value;

  if (token) {
    try {
      await fetch(`${getApiUrl()}/api/auth/logout`, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
      });
    } catch {
      // Ignore errors, clear the cookie anyway
    }
  }

  const response = NextResponse.json({ data: { success: true } });
  response.cookies.delete("admin_token");
  return response;
}
