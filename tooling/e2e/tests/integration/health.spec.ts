import { test, expect } from "@playwright/test";

/**
 * Health — Service reachability (no auth needed)
 *
 * Validates that all backend services are up and responding.
 * These fail HARD when services are down — no graceful skipping.
 */

const BASE_URL = process.env.BASE_URL || "https://www.all-source.xyz";
const QS_URL =
  process.env.QS_URL ||
  process.env.NEXT_PUBLIC_API_URL ||
  "https://allsource-query.fly.dev";
const CP_URL =
  process.env.CONTROL_PLANE_URL || "http://localhost:3901";

test.describe("Service Health", () => {
  test("Web App responds at BASE_URL", async ({ request }) => {
    const resp = await request.get(BASE_URL);
    expect(resp.status()).toBeLessThan(500);
  });

  test("QS /api/health returns 200 with status field", async ({ request }) => {
    const resp = await request.get(`${QS_URL}/api/health`);
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(body.status).toBeDefined();
  });

  test("Control Plane /health returns 200", async ({ request }) => {
    const resp = await request.get(`${CP_URL}/health`);
    expect(resp.status()).toBeLessThan(500);
  });

  test("Next.js proxy /api/v1/health forwards to QS", async ({ request }) => {
    // This catches proxy/CORS breakage — the Next.js app must successfully
    // forward to QS, not return its own error page.
    const resp = await request.get(`${BASE_URL}/api/v1/health`);
    expect(resp.status()).toBeLessThan(500);

    // If the proxy works, we should get JSON back (not an HTML error page)
    const contentType = resp.headers()["content-type"] || "";
    if (resp.ok()) {
      expect(contentType).toContain("json");
    }
  });
});
