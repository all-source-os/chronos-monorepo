import { type NextRequest, NextResponse } from "next/server";
import { decodeJwt, isAdminRole } from "@/lib/auth";

/**
 * Proxy that protects all authenticated routes.
 *
 * - Public routes (/login, /api/auth/*, /api/v1/auth/*) are allowed through.
 * - All other routes require a valid `admin_token` cookie with `role: "admin"`.
 * - Non-admin users receive a 403 response.
 * - Unauthenticated users are redirected to /login.
 */
export function proxy(request: NextRequest) {
  const { pathname } = request.nextUrl;

  // Public routes — no auth required
  if (
    pathname === "/login" ||
    pathname.startsWith("/api/auth/") ||
    pathname.startsWith("/api/v1/auth/") ||
    pathname.startsWith("/_next/") ||
    pathname.startsWith("/favicon")
  ) {
    return NextResponse.next();
  }

  const token = request.cookies.get("admin_token")?.value;

  // No token — redirect to login
  if (!token) {
    const loginUrl = new URL("/login", request.url);
    loginUrl.searchParams.set("redirect", pathname);
    return NextResponse.redirect(loginUrl);
  }

  // Decode JWT and check admin role
  const payload = decodeJwt(token);

  if (!payload) {
    // Token is expired or malformed — clear cookie and redirect to login
    const loginUrl = new URL("/login", request.url);
    loginUrl.searchParams.set("error", "invalid_token");
    const response = NextResponse.redirect(loginUrl);
    response.cookies.delete("admin_token");
    return response;
  }

  if (!isAdminRole(payload)) {
    // Valid token but not an admin — 403
    return new NextResponse(
      JSON.stringify({
        error: {
          code: "forbidden",
          message: "Admin access required. Your account does not have the admin role.",
        },
      }),
      {
        status: 403,
        headers: { "content-type": "application/json" },
      }
    );
  }

  // Admin token is valid — allow through
  return NextResponse.next();
}

export const config = {
  matcher: [
    /*
     * Match all request paths except static files and images.
     */
    "/((?!_next/static|_next/image|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|gif|webp)$).*)",
  ],
};
