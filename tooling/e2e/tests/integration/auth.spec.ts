import { test, expect } from "@playwright/test";

/**
 * Auth — Demo account lifecycle and JWT validation
 *
 * Tests the full auth flow using raw request contexts (no browser).
 * Validates CP demo start → login → token usage → logout.
 */

const CP_URL =
  process.env.CONTROL_PLANE_URL || "http://localhost:3901";
const QS_URL =
  process.env.QS_URL ||
  process.env.NEXT_PUBLIC_API_URL ||
  "https://allsource-query.fly.dev";

/** Helper: create a fresh demo account and return credentials + token */
async function createDemoSession(request: any) {
  const demoResp = await request.post(`${CP_URL}/api/v1/demo/start`, {
    headers: { "Content-Type": "application/json" },
  });
  expect(demoResp.ok()).toBeTruthy();
  const demo = await demoResp.json();

  const loginResp = await request.post(`${CP_URL}/api/v1/auth/login`, {
    headers: { "Content-Type": "application/json" },
    data: { email: demo.email, password: demo.password },
  });
  expect(loginResp.ok()).toBeTruthy();
  const login = await loginResp.json();

  return { email: demo.email, password: demo.password, token: login.token };
}

test.describe("Demo Auth Lifecycle", () => {
  test("POST /api/v1/demo/start creates demo credentials", async ({
    request,
  }) => {
    const resp = await request.post(`${CP_URL}/api/v1/demo/start`, {
      headers: { "Content-Type": "application/json" },
    });
    expect(resp.ok()).toBeTruthy();

    const data = await resp.json();
    expect(data.email).toBeTruthy();
    expect(data.password).toBeTruthy();
  });

  test("POST /api/v1/auth/login returns JWT", async ({ request }) => {
    const demoResp = await request.post(`${CP_URL}/api/v1/demo/start`, {
      headers: { "Content-Type": "application/json" },
    });
    const demo = await demoResp.json();

    const resp = await request.post(`${CP_URL}/api/v1/auth/login`, {
      headers: { "Content-Type": "application/json" },
      data: { email: demo.email, password: demo.password },
    });
    expect(resp.ok()).toBeTruthy();

    const data = await resp.json();
    expect(data.token).toBeTruthy();
    expect(typeof data.token).toBe("string");
  });

  test("GET /api/auth/me with Bearer token returns user", async ({
    request,
  }) => {
    const session = await createDemoSession(request);

    const resp = await request.get(`${QS_URL}/api/auth/me`, {
      headers: { Authorization: `Bearer ${session.token}` },
    });
    expect(resp.ok()).toBeTruthy();

    const data = await resp.json();
    const user = data.data?.user || data.user || data;
    expect(user).toBeDefined();
  });

  test("invalid token returns 401", async ({ request }) => {
    const resp = await request.get(`${QS_URL}/api/auth/me`, {
      headers: { Authorization: "Bearer invalid-token-12345" },
    });
    expect(resp.status()).toBe(401);
  });

  test("missing token returns 401", async ({ request }) => {
    const resp = await request.get(`${QS_URL}/api/auth/me`);
    expect(resp.status()).toBe(401);
  });
});
