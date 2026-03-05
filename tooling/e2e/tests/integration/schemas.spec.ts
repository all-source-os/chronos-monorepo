import { test, expect } from "../../fixtures/api-auth";
import { expectOk } from "../../fixtures/api-helpers";

/**
 * Schemas — Register and list event schemas
 */

test.describe("Schemas", () => {
  test("GET /api/schemas returns valid response", async ({ apiContext }) => {
    const resp = await apiContext.get("/api/schemas");
    await expectOk(resp, "GET /api/schemas");

    const body = await resp.json();
    const schemas = body.schemas || body.data || body;
    expect(Array.isArray(schemas)).toBeTruthy();
  });

  test("POST /api/schemas registers a schema", async ({ apiContext }) => {
    const schemaName = `e2e.schema.${Date.now()}`;
    const resp = await apiContext.post("/api/schemas", {
      data: {
        name: schemaName,
        schema: {
          type: "object",
          properties: {
            test: { type: "boolean" },
            value: { type: "number" },
          },
        },
      },
    });

    await expectOk(resp, "POST /api/schemas");
  });
});
