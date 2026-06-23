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
export function decodeJwt(token: string): JwtPayload | null {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;

    const payloadPart = parts[1];
    if (!payloadPart) return null;
    const payload = JSON.parse(
      Buffer.from(payloadPart, "base64url").toString("utf-8")
    );

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
  // fallback inside a Vercel server function and make this fetch throw (→ auth_failed).
  const meUrl = `${getControlPlaneUrl()}/api/auth/me`;
  try {
    const meResponse = await fetch(meUrl, {
      headers: { Authorization: `Bearer ${token}` },
    });

    if (!meResponse.ok) {
      console.error(
        `[admin-auth] /api/auth/me -> ${meResponse.status} (url: ${meUrl})`
      );
      return { valid: false, error: "invalid_token" };
    }

    const data = await meResponse.json();
    const user = data.data?.user || data.data;

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
      `[admin-auth] validateAdminToken threw (url: ${meUrl}):`,
      err instanceof Error ? err.message : err
    );
    return { valid: false, error: "auth_failed" };
  }
}
