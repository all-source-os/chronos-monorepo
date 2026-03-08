/**
 * Minimal HS256 JWT generator for e2e tests.
 *
 * No external dependencies — uses Node's built-in crypto.
 * Generates tokens compatible with the Control Plane's AdminAuthMiddleware.
 */

import { createHmac } from "node:crypto";

export const E2E_JWT_SECRET =
  "e2e-test-jwt-secret-that-is-at-least-32-characters-long";

function base64url(input: string | Buffer): string {
  const buf = typeof input === "string" ? Buffer.from(input) : input;
  return buf.toString("base64url");
}

interface AdminClaims {
  sub: string;
  email: string;
  name: string;
  username: string;
  tenant_id: string;
  role: "admin";
  provider: string;
  iat: number;
  exp: number;
  iss: string;
}

export function generateAdminJwt(
  overrides: Partial<AdminClaims> = {}
): string {
  const now = Math.floor(Date.now() / 1000);

  const header = base64url(JSON.stringify({ alg: "HS256", typ: "JWT" }));

  const payload: AdminClaims = {
    sub: "e2e-admin-user",
    email: "admin@e2e-test.local",
    name: "E2E Admin",
    username: "e2e-admin",
    tenant_id: "e2e-admin-tenant",
    role: "admin",
    provider: "e2e",
    iat: now,
    exp: now + 7 * 24 * 60 * 60, // 7 days
    iss: "allsource",
    ...overrides,
  };

  const payloadEncoded = base64url(JSON.stringify(payload));
  const sigInput = `${header}.${payloadEncoded}`;
  const signature = createHmac("sha256", E2E_JWT_SECRET)
    .update(sigInput)
    .digest();

  return `${sigInput}.${base64url(signature)}`;
}
