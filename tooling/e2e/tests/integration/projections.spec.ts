import { test, expect } from "../../fixtures/api-auth";
import { expectOk } from "../../fixtures/api-helpers";

/**
 * Projections — List projections, verify structure
 */

test.describe("Projections", () => {
  test("GET /api/projections returns array with total count", async ({
    apiContext,
  }) => {
    const resp = await apiContext.get("/api/projections");
    await expectOk(resp, "GET /api/projections");

    const body = await resp.json();
    const projections = body.projections || body.data || body;
    expect(Array.isArray(projections)).toBeTruthy();

    if (body.total !== undefined) {
      expect(typeof body.total).toBe("number");
    }
  });

  test("each projection has name and status fields", async ({
    apiContext,
  }) => {
    const resp = await apiContext.get("/api/projections");
    await expectOk(resp, "GET /api/projections");

    const body = await resp.json();
    const projections = body.projections || body.data || body;

    for (const proj of projections) {
      expect(proj.name || proj.projection_name).toBeTruthy();
      expect(proj.status !== undefined).toBeTruthy();
    }
  });
});
