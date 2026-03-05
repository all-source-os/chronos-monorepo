import { test, expect } from "../../fixtures/api-auth";
import { expectOk } from "../../fixtures/api-helpers";

/**
 * Analytics — Tenant analytics endpoints
 */

test.describe("Analytics", () => {
  test("GET /api/tenants/me/analytics?range=30d returns data", async ({
    apiContext,
  }) => {
    const resp = await apiContext.get("/api/tenants/me/analytics?range=30d");
    await expectOk(resp, "GET /api/tenants/me/analytics?range=30d");

    const body = await resp.json();
    expect(body).toBeDefined();
  });

  for (const range of ["7d", "30d", "90d"]) {
    test(`GET /api/tenants/me/analytics?range=${range} returns valid response`, async ({
      apiContext,
    }) => {
      const resp = await apiContext.get(
        `/api/tenants/me/analytics?range=${range}`
      );
      await expectOk(resp, `GET /api/tenants/me/analytics?range=${range}`);

      const body = await resp.json();
      expect(body).toBeDefined();
    });
  }
});
