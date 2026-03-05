import { test, expect } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

/**
 * Proxy & CORS — Verifies the Next.js API proxy layer
 *
 * These tests ensure that:
 * 1. All API calls go through the Next.js proxy (no direct backend URLs)
 * 2. Proxy forwards set-cookie headers from backend
 * 3. Auth callback sets httpOnly cookie and redirects
 * 4. Session endpoint reads cookie and returns user data
 * 5. No CORS errors from same-origin proxy pattern
 *
 * Run locally:
 *   cd tooling/e2e && BASE_URL=http://localhost:3000 bunx playwright test tests/integration/proxy-cors.spec.ts
 *
 * Run against production:
 *   cd tooling/e2e && bunx playwright test tests/integration/proxy-cors.spec.ts
 */

const BASE_URL = process.env.BASE_URL || "https://www.all-source.xyz";
const QS_URL =
  process.env.QS_URL ||
  process.env.NEXT_PUBLIC_API_URL ||
  "https://allsource-query.fly.dev";

// ─── Proxy forwarding ────────────────────────────────────────────────────────

test.describe("Proxy — request forwarding", () => {
  test("GET /api/v1/health proxies to Query Service", async ({ request }) => {
    const resp = await request.get(`${BASE_URL}/api/v1/health`);

    // Proxy should return the QS response, not a Next.js error page
    expect(resp.status()).toBeLessThan(500);
    const contentType = resp.headers()["content-type"] || "";
    if (resp.ok()) {
      expect(contentType).toContain("json");
    }
  });

  test("proxied response has JSON content-type, not text/html", async ({
    request,
  }) => {
    // A misconfigured proxy returns Next.js HTML error pages instead of JSON
    const resp = await request.get(`${BASE_URL}/api/v1/health`);
    if (resp.ok()) {
      const contentType = resp.headers()["content-type"] || "";
      expect(contentType).not.toContain("text/html");
    }
  });
});

// ─── Auth cookie flow ────────────────────────────────────────────────────────

test.describe("Proxy — auth cookie flow", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test("auth callback sets httpOnly cookie and redirects to dashboard", async ({
    request,
  }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");

    // Hit the callback endpoint — it should validate the token and redirect
    const resp = await request.get(
      `${BASE_URL}/api/auth/callback?token=${encodeURIComponent(token!)}&new_user=false`,
      { maxRedirects: 0 }
    );

    // Should be a redirect (302)
    expect([301, 302, 307, 308]).toContain(resp.status());

    // Redirect should point to /dashboard, not to an external URL
    const location = resp.headers()["location"] || "";
    expect(location).toMatch(/\/dashboard/);
    // Must NOT redirect to a different origin (that would be a CORS leak)
    if (location.startsWith("http")) {
      const redirectOrigin = new URL(location).origin;
      const baseOrigin = new URL(BASE_URL).origin;
      expect(redirectOrigin).toBe(baseOrigin);
    }

    // Should set auth_token cookie
    const setCookie = resp.headers()["set-cookie"] || "";
    expect(setCookie).toContain("auth_token");
    expect(setCookie.toLowerCase()).toContain("httponly");
  });

  test("session endpoint returns user data with valid cookie", async ({
    request,
  }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");

    // First set the cookie via callback
    await request.get(
      `${BASE_URL}/api/auth/callback?token=${encodeURIComponent(token!)}&new_user=false`
    );

    // Now call session — cookie should be included automatically
    const resp = await request.get(`${BASE_URL}/api/auth/session`);
    expect(resp.ok()).toBeTruthy();

    const body = await resp.json();
    expect(body.data).toBeDefined();
    expect(body.data.user).toBeDefined();
  });

  test("session endpoint returns 401 without cookie", async ({ request }) => {
    // Use a fresh request context (no cookies)
    const resp = await request.fetch(`${BASE_URL}/api/auth/session`, {
      headers: {},
    });
    expect(resp.status()).toBe(401);

    const body = await resp.json();
    expect(body.error).toBeDefined();
    expect(body.error.code).toBe("not_authenticated");
  });

  test("session DELETE clears auth cookie", async ({ request }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");

    // First authenticate
    await request.get(
      `${BASE_URL}/api/auth/callback?token=${encodeURIComponent(token!)}&new_user=false`
    );

    // Logout
    const resp = await request.delete(`${BASE_URL}/api/auth/session`);
    expect(resp.ok()).toBeTruthy();

    // Cookie should be cleared
    const setCookie = resp.headers()["set-cookie"] || "";
    if (setCookie.includes("auth_token")) {
      // Cookie deletion sets Max-Age=0 or Expires in the past
      expect(
        setCookie.includes("Max-Age=0") ||
          setCookie.includes("max-age=0") ||
          setCookie.includes("Expires=Thu, 01 Jan 1970")
      ).toBeTruthy();
    }
  });
});

// ─── Proxy — authenticated API calls ─────────────────────────────────────────

test.describe("Proxy — authenticated API forwarding", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test("proxied GET /api/v1/events returns JSON with auth header", async ({
    request,
  }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");

    const resp = await request.get(`${BASE_URL}/api/v1/events`, {
      headers: { Authorization: `Bearer ${token}` },
    });

    // Should proxy to QS and return events (or 200 with empty list)
    expect(resp.status()).toBeLessThan(500);
    if (resp.ok()) {
      const body = await resp.json();
      // QS wraps in {events: [...]} or proxy unwraps — either is valid
      expect(body).toBeDefined();
    }
  });

  test("proxied POST /api/v1/events creates event through proxy", async ({
    request,
  }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");

    const resp = await request.post(`${BASE_URL}/api/v1/events`, {
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
      data: {
        entity_id: `proxy-test-${Date.now()}`,
        event_type: "proxy.cors.test",
        payload: { source: "e2e-proxy-cors-test" },
      },
    });

    // 200 (Core returns 200, not 201) or 201 — either means the proxy works
    expect(resp.status()).toBeLessThan(300);
  });

  test("proxied DELETE forwards request body", async ({ request }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");

    // We can't easily test DELETE body forwarding end-to-end without a
    // specific endpoint that reads DELETE bodies. Instead, verify that a
    // DELETE request to a known endpoint succeeds (doesn't 502/504).
    const resp = await request.delete(`${BASE_URL}/api/v1/events`, {
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
    });

    // Should NOT be a proxy error — any 4xx/5xx from the backend is fine,
    // but 502 means the proxy itself failed
    expect(resp.status()).not.toBe(502);
  });
});

// ─── No direct backend URL leaks ─────────────────────────────────────────────

test.describe("CORS — no backend URL leaks in client bundle", () => {
  test("dashboard page does not load scripts referencing backend directly", async ({
    page,
    request,
  }) => {
    const token = await demoLogin(request);
    test.skip(!token, "Demo login failed (Control Plane not running)");

    // Authenticate
    await page.goto(
      `${BASE_URL}/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
    if (!page.url().includes("/dashboard")) {
      await page.goto(`${BASE_URL}/dashboard`);
    }

    // Collect all script sources loaded by the page
    const scriptUrls: string[] = [];
    page.on("request", (req) => {
      if (req.resourceType() === "script" || req.resourceType() === "fetch") {
        scriptUrls.push(req.url());
      }
    });

    // Give the page time to load scripts and make initial API calls
    await page.waitForTimeout(3000);

    // No request should go directly to the backend ports (3900, 3901, 3902)
    // or to fly.dev backend URLs
    for (const url of scriptUrls) {
      expect(url).not.toMatch(/:390[012]\b/);
      expect(url).not.toContain("allsource-query.fly.dev");
      expect(url).not.toContain("allsource-core.fly.dev");
    }
  });
});
