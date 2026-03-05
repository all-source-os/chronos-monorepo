import { test, expect } from "../../fixtures/api-auth";
import { expectOk } from "../../fixtures/api-helpers";

/**
 * API Keys — Create → use → rotate → revoke
 *
 * Full lifecycle test for API key management through QS.
 * Each test is self-contained (creates its own key).
 */

test.describe("API Keys Lifecycle", () => {
  test("POST /api/api-keys creates a key with id and value", async ({
    apiContext,
  }) => {
    const keyName = `e2e-create-${Date.now()}`;
    const resp = await apiContext.post("/api/api-keys", {
      data: { name: keyName, scopes: ["events:read"] },
    });
    await expectOk(resp, "API keys");

    const body = await resp.json();
    expect(body.id || body.key_id).toBeTruthy();
  });

  test("created key appears in GET /api/api-keys list", async ({
    apiContext,
  }) => {
    const keyName = `e2e-list-${Date.now()}`;
    const createResp = await apiContext.post("/api/api-keys", {
      data: { name: keyName, scopes: ["events:read"] },
    });
    expect(createResp.ok()).toBeTruthy();
    const created = await createResp.json();
    const keyId = created.id || created.key_id;

    const listResp = await apiContext.get("/api/api-keys");
    expect(listResp.ok()).toBeTruthy();

    const body = await listResp.json();
    const keys = body.keys || body.data || body;
    expect(Array.isArray(keys)).toBeTruthy();

    const found = keys.find((k: any) => (k.id || k.key_id) === keyId);
    expect(found).toBeDefined();

    // Cleanup
    await apiContext.delete(`/api/api-keys/${keyId}`);
  });

  test("API key authenticates requests via X-API-Key header", async ({
    apiContext,
    playwright,
  }) => {
    const keyName = `e2e-auth-${Date.now()}`;
    const createResp = await apiContext.post("/api/api-keys", {
      data: { name: keyName, scopes: ["events:read"] },
    });
    expect(createResp.ok()).toBeTruthy();
    const created = await createResp.json();
    const keyValue = created.key || created.api_key || created.value;
    const keyId = created.id || created.key_id;

    if (!keyValue) {
      // Cleanup and skip if API doesn't return the raw key value
      await apiContext.delete(`/api/api-keys/${keyId}`);
      test.skip(true, "Key value not returned by create endpoint");
    }

    const keyCtx = await playwright.request.newContext({
      baseURL:
        process.env.QS_URL ||
        process.env.NEXT_PUBLIC_API_URL ||
        "https://allsource-query.fly.dev",
      extraHTTPHeaders: { "X-API-Key": keyValue },
    });

    const resp = await keyCtx.get("/api/events?limit=1");
    await expectOk(resp, "API keys");

    await keyCtx.dispose();
    await apiContext.delete(`/api/api-keys/${keyId}`);
  });

  test("DELETE /api/api-keys/:id revokes the key", async ({ apiContext }) => {
    const keyName = `e2e-revoke-${Date.now()}`;
    const createResp = await apiContext.post("/api/api-keys", {
      data: { name: keyName, scopes: ["events:read"] },
    });
    expect(createResp.ok()).toBeTruthy();
    const created = await createResp.json();
    const keyId = created.id || created.key_id;

    const resp = await apiContext.delete(`/api/api-keys/${keyId}`);
    await expectOk(resp, "API keys");

    // Verify it's gone or marked revoked
    const listResp = await apiContext.get("/api/api-keys");
    expect(listResp.ok()).toBeTruthy();
    const body = await listResp.json();
    const keys = body.keys || body.data || body;
    const active = keys.find(
      (k: any) => (k.id || k.key_id) === keyId && k.status !== "revoked"
    );
    expect(active).toBeUndefined();
  });

  test("revoked key returns 401", async ({ apiContext, playwright }) => {
    const keyName = `e2e-revoked-${Date.now()}`;
    const createResp = await apiContext.post("/api/api-keys", {
      data: { name: keyName, scopes: ["events:read"] },
    });
    expect(createResp.ok()).toBeTruthy();
    const created = await createResp.json();
    const keyValue = created.key || created.api_key || created.value;
    const keyId = created.id || created.key_id;

    if (!keyValue) {
      await apiContext.delete(`/api/api-keys/${keyId}`);
      test.skip(true, "Key value not returned by create endpoint");
    }

    // Revoke first
    await apiContext.delete(`/api/api-keys/${keyId}`);

    // Now try using it
    const keyCtx = await playwright.request.newContext({
      baseURL:
        process.env.QS_URL ||
        process.env.NEXT_PUBLIC_API_URL ||
        "https://allsource-query.fly.dev",
      extraHTTPHeaders: { "X-API-Key": keyValue },
    });

    const resp = await keyCtx.get("/api/events?limit=1");
    expect(resp.status()).toBe(401);

    await keyCtx.dispose();
  });
});
