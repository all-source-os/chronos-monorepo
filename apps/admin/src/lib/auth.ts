/**
 * Admin auth utilities.
 *
 * The admin app uses the same OAuth providers (Google, GitHub) as the main web
 * app, proxying through the Control Plane. After login, the Control Plane
 * returns a JWT. The admin app stores it in an httpOnly cookie (`admin_token`)
 * and validates that the JWT contains `role: "admin"` on every request.
 */

export interface AdminUser {
  id: string;
  email: string;
  name: string;
  role: string;
  avatar_url?: string;
}

export interface JwtPayload {
  sub: string;
  email: string;
  name: string;
  role: string;
  exp: number;
  iat: number;
}

/**
 * Decode and validate a JWT token without verifying the signature.
 * Signature verification is handled by the Control Plane /api/auth/me endpoint.
 * This only extracts claims for client-side role checks.
 */
// Edge-safe base64url decode. decodeJwt runs inside the Next.js middleware
// (proxy.ts, Edge runtime) where Node's `Buffer` is NOT available — using Buffer
// there threw and crashed the middleware for every authenticated request once a
// token cookie was present (the post-login "This page couldn't load"). atob +
// TextDecoder work in Edge, Node, and the browser.
function base64UrlDecode(input: string): string {
  const b64 = input.replace(/-/g, "+").replace(/_/g, "/");
  const padded = b64.padEnd(b64.length + ((4 - (b64.length % 4)) % 4), "=");
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

export function decodeJwt(token: string): JwtPayload | null {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;

    const payloadPart = parts[1];
    if (!payloadPart) return null;
    const payload = JSON.parse(base64UrlDecode(payloadPart));

    // Check expiration
    if (payload.exp && payload.exp * 1000 < Date.now()) {
      return null;
    }

    return payload as JwtPayload;
  } catch {
    return null;
  }
}

/**
 * Check if a decoded JWT has the admin role.
 */
export function isAdminRole(payload: JwtPayload): boolean {
  return payload.role === "admin";
}

export function getControlPlaneUrl(): string {
  return process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3901";
}

/**
 * Validate a token server-side by calling the Control Plane and checking the
 * admin role claim.
 */
export async function validateAdminToken(
  token: string
): Promise<{ valid: true; user: AdminUser } | { valid: false; error: string }> {
  // Server-side call → use the runtime Control Plane URL (CONTROL_PLANE_INTERNAL_URL),
  // NOT the build-inlined NEXT_PUBLIC_API_URL which can resolve to the localhost
  // fallback inside a Vercel server function and make this fetch throw.
  //
  // The Control Plane validates a Bearer token and returns the caller via
  // GET /api/v1/auth/session (SessionHandler). NOTE: /api/auth/me is served by the
  // Query Service, NOT the CP — calling it on the CP 404s and yields invalid_token.
  const sessionUrl = `${getControlPlaneUrl()}/api/v1/auth/session`;
  try {
    const meResponse = await fetch(sessionUrl, {
      headers: { Authorization: `Bearer ${token}` },
    });

    if (!meResponse.ok) {
      console.error(
        `[admin-auth] /api/v1/auth/session -> ${meResponse.status} (url: ${sessionUrl})`
      );
      return { valid: false, error: "invalid_token" };
    }

    const data = await meResponse.json();
    // SessionHandler returns { user: {...}, tenant_id, core_api_key }.
    const user = data.user || data.data?.user || data.data;

    if (!user) {
      console.error("[admin-auth] /api/auth/me ok but no user in payload");
      return { valid: false, error: "no_user_data" };
    }

    // Check admin role from JWT claims
    const payload = decodeJwt(token);
    if (!payload || !isAdminRole(payload)) {
      console.error(
        `[admin-auth] role check failed (role: ${payload?.role ?? "none"})`
      );
      return { valid: false, error: "not_admin" };
    }

    return {
      valid: true,
      user: {
        id: user.id,
        email: user.email,
        name: user.name || user.email,
        role: payload.role,
        avatar_url: user.avatar_url,
      },
    };
  } catch (err) {
    console.error(
      `[admin-auth] validateAdminToken threw (url: ${sessionUrl}):`,
      err instanceof Error ? err.message : err
    );
    return { valid: false, error: "auth_failed" };
  }
}
