import { type NextRequest, NextResponse } from "next/server";
import { getApiUrl } from "@/lib/api/client";

export async function GET(request: NextRequest) {
  const searchParams = request.nextUrl.searchParams;
  const token = searchParams.get("token");
  const error = searchParams.get("error");
  const isNewUser = searchParams.get("new_user") === "true";

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
    const meResponse = await fetch(`${getApiUrl()}/api/auth/me`, {
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });

    if (!meResponse.ok) {
      const loginUrl = new URL("/login", request.url);
      loginUrl.searchParams.set("error", "invalid_token");
      return NextResponse.redirect(loginUrl);
    }

    // Token is valid, set cookie and redirect
    const redirectUrl = isNewUser ? "/onboarding" : "/dashboard";
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
