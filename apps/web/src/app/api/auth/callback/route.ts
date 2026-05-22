import { type NextRequest, NextResponse } from "next/server";
import { getServerApiUrl } from "@/lib/api/client";

// Only allow same-origin paths to defend against open-redirect via ?next=.
// Reject absolute URLs (//evil.com, https://...) and API/auth routes.
function safeNextPath(raw: string | null): string | null {
  if (!raw) return null;
  if (!raw.startsWith("/") || raw.startsWith("//")) return null;
  if (raw.startsWith("/api/") || raw.startsWith("/api?")) return null;
  return raw;
}

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const token = searchParams.get("token");
  const error = searchParams.get("error");
  const isNewUser = searchParams.get("new_user") === "true";
  const nextPath = safeNextPath(searchParams.get("next"));

  // Handle OAuth errors
  if (error) {
    const loginUrl = new URL("/login", request.url);
    loginUrl.searchParams.set("error", error);
    return NextResponse.redirect(loginUrl);
  }

  // Token is required
  if (!token) {
    const loginUrl = new URL("/login", request.url);
    loginUrl.searchParams.set("error", "missing_token");
    return NextResponse.redirect(loginUrl);
  }

  // Verify token by fetching user info
  try {
    const meResponse = await fetch(`${getServerApiUrl()}/api/auth/me`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });

    if (!meResponse.ok) {
      const loginUrl = new URL("/login", request.url);
      loginUrl.searchParams.set("error", "invalid_token");
      return NextResponse.redirect(loginUrl);
    }

    // Token is valid, set cookie and redirect.
    // If the caller specified a safe ?next= target (e.g. /connect), honor it.
    // Otherwise fall back to onboarding (new user) or dashboard.
    const redirectUrl = nextPath ?? (isNewUser ? "/onboarding" : "/dashboard");
    const response = NextResponse.redirect(new URL(redirectUrl, request.url));

    // Set httpOnly cookie with the token
    response.cookies.set("auth_token", token, {
      httpOnly: true,
      secure: process.env.NODE_ENV === "production",
      sameSite: "lax",
      maxAge: 60 * 60 * 24 * 7, // 7 days
      path: "/",
    });

    return response;
  } catch {
    const loginUrl = new URL("/login", request.url);
    loginUrl.searchParams.set("error", "auth_failed");
    return NextResponse.redirect(loginUrl);
  }
}
