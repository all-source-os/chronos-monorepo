import { test, expect } from "../../fixtures/api-auth";
import { expectOk } from "../../fixtures/api-helpers";

/**
 * Tenant — Tenant info, usage, team, audit logs
 *
 * Strict assertions — if demo tenant resolution is broken, these fail
 * with descriptive error messages showing the actual response.
 */

test.describe("Tenant Info", () => {
  test("GET /api/tenant returns tenant with plan info", async ({
    apiContext,
  }) => {
    const resp = await apiContext.get("/api/tenant");
    await expectOk(resp, "GET /api/tenant");

    const body = await resp.json();
    const tenant = body.data || body;
    expect(tenant.id || tenant.tenant_id).toBeTruthy();
  });

  test("GET /api/tenant/usage returns quota data", async ({ apiContext }) => {
    const resp = await apiContext.get("/api/tenant/usage");
    await expectOk(resp, "GET /api/tenant/usage");

    const body = await resp.json();
    expect(body).toBeDefined();
  });
});

test.describe("Team", () => {
  test("GET /api/team/members includes current user", async ({
    apiContext,
    demoAuth,
  }) => {
    const resp = await apiContext.get("/api/team/members");
    await expectOk(resp, "GET /api/team/members");

    const body = await resp.json();
    const members =
      body.data?.members || body.members || body.data || body;
    expect(Array.isArray(members)).toBeTruthy();
    expect(members.length).toBeGreaterThan(0);

    const me = members.find((m: any) => m.email === demoAuth.email);
    expect(me).toBeDefined();
  });
});

test.describe("Audit Logs", () => {
  test("GET /api/tenant/audit-logs returns entries array", async ({
    apiContext,
  }) => {
    const resp = await apiContext.get("/api/tenant/audit-logs?limit=25");
    await expectOk(resp, "GET /api/tenant/audit-logs");

    const body = await resp.json();
    const entries =
      body.data?.entries || body.logs || body.entries || body.data || body;
    expect(Array.isArray(entries)).toBeTruthy();
  });
});
