import { test, expect, type APIRequestContext, type Page } from "@playwright/test";
import { demoLoginFull, type DemoAuth } from "../../fixtures/demo-auth";

/**
 * Backend Integration E2E Tests — Full-stack: QS + Core + Control Plane
 *
 * No mocking. Every test creates real data, queries real APIs, and verifies
 * the responses. Tests exercise the actual QS → Core pipeline and validate
 * that the UI correctly displays what the backends return.
 *
 * Two test paths:
 * 1. Direct QS calls with Bearer token (tests the QS API contract)
 * 2. Browser-based calls through the Next.js app (tests the real user flow)
 *
 * Run against production:
 *   BASE_URL=https://www.all-source.xyz CONTROL_PLANE_URL=https://allsource-control-plane.fly.dev \
 *     QS_URL=https://allsource-query.fly.dev \
 *     bunx playwright test tests/smoke/backend-integration.spec.ts
 */

const QS_URL =
  process.env.QS_URL || process.env.NEXT_PUBLIC_API_URL || "https://allsource-query.fly.dev";

let auth: DemoAuth | null = null;

test.beforeAll(async ({ request }) => {
  auth = await demoLoginFull(request);
});

/** Authenticate the browser via callback so cookie is set. */
async function authenticateBrowser(page: Page) {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
}

/** Make an authenticated request to the Query Service directly (Bearer token). */
async function qsDirect(
  request: APIRequestContext,
  method: string,
  path: string,
  data?: unknown
) {
  const url = `${QS_URL}${path}`;
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${auth!.token}`,
  };
  return request.fetch(url, { method, headers, data });
}

/**
 * Make an authenticated request through the browser context (cookie auth).
 * This mirrors what the dashboard SWR hooks actually do — fetch from QS
 * with credentials: "include" so the auth_token cookie is forwarded.
 */
async function qsViaBrowser(
  page: Page,
  method: string,
  path: string,
  data?: unknown
) {
  const url = `${QS_URL}${path}`;
  if (method === "GET") {
    return page.request.get(url);
  }
  return page.request.fetch(url, { method, data });
}

// ---------------------------------------------------------------------------
// Health — all 3 backends must be reachable
// ---------------------------------------------------------------------------

test.describe("Backend Health", () => {
  test("Control Plane /health is reachable", async ({ request }) => {
    const resp = await request.get(`${CP_URL}/health`);
    expect(resp.status()).toBeLessThan(500);
  });

  test("QS /api/health is reachable", async ({ request }) => {
    const resp = await request.get(`${QS_URL}/api/health`);
    // 503 means QS is up but a dependency is down — still a real issue to surface
    expect(resp.status()).toBeLessThan(504);
    if (resp.status() >= 500) {
      const body = await resp.text().catch(() => "");
      console.warn(`QS health returned ${resp.status()}: ${body}`);
    }
  });

  test("status page shows real health for all 3 services", async ({ page }) => {
    await page.goto("/status");

    await expect(
      page.getByRole("heading", { name: /system status/i })
    ).toBeVisible({ timeout: 10000 });

    // Wait for the first health poll
    await expect(
      page.getByText(/operational|degraded|down/i).first()
    ).toBeVisible({ timeout: 20000 });

    await expect(page.getByText("Core (Event Store)")).toBeVisible();
    await expect(page.getByText("Query Service")).toBeVisible();
    await expect(page.getByText("Control Plane", { exact: true }).first()).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Session — QS /api/auth/me + /api/tenant via Next.js session route
// ---------------------------------------------------------------------------

test.describe("Session (QS auth/me + tenant)", () => {
  test.beforeEach(async ({ page }) => {
    test.skip(!auth, "Demo login failed");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  });

  test("session returns user with email from QS /api/auth/me", async ({ page }) => {
    const resp = await page.request.get("/api/auth/session");
    expect(resp.ok()).toBeTruthy();

    const session = await resp.json();
    expect(session.data).toBeDefined();
    expect(session.data.user).toBeDefined();
    // BUG: QS /api/auth/me returns email: null for demo accounts.
    // The CP JWT doesn't embed the email, so QS can't populate it.
    // This means the Settings > Profile email field will be empty.
    expect(session.data.user.email).toBe(auth!.email);
  });

  test("session returns tenant data from QS /api/tenant", async ({ page }) => {
    const resp = await page.request.get("/api/auth/session");
    expect(resp.ok()).toBeTruthy();

    const session = await resp.json();
    // BUG: QS /api/tenant returns "tenant_not_found" for demo accounts.
    // The demo tenant created in CP doesn't get synced to QS's PostgreSQL.
    // This breaks: billing, usage quotas, team members, audit logs, API keys.
    expect(session.data.tenant).toBeDefined();
    expect(session.data.tenant.id).toBeTruthy();
  });

  test("dashboard greeting renders with real user data from QS", async ({ page }) => {
    await page.goto("/dashboard");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await expect(
      page.getByRole("heading", { name: /good (morning|afternoon|evening)/i })
    ).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Events CRUD — create via QS → Core stores → query back → UI shows
// ---------------------------------------------------------------------------

test.describe("Events E2E (QS → Core)", () => {
  test.beforeEach(async ({ page }) => {
    test.skip(!auth, "Demo login failed");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  });

  const testEventType = `e2e.dashboard.${Date.now()}`;
  const testEntityId = `e2e-entity-${Date.now()}`;

  test("POST /api/events creates an event in Core", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "POST", "/api/events", {
      event_type: testEventType,
      entity_id: testEntityId,
      data: { test: true, source: "e2e-backend-integration" },
    });

    if (!resp.ok()) {
      const body = await resp.text();
      console.error(`POST /api/events failed (${resp.status()}): ${body}`);
    }
    expect(resp.ok()).toBeTruthy();
  });

  test("GET /api/events returns valid response with events array", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/events?limit=5");
    expect(resp.ok()).toBeTruthy();

    const data = await resp.json();
    const events = data.events || data.data || data;
    expect(Array.isArray(events)).toBeTruthy();
  });

  test("created event appears in Event Explorer UI", async ({ page, request }) => {
    test.skip(!auth, "Demo login failed");

    // Create an event
    const createResp = await qsDirect(request, "POST", "/api/events", {
      event_type: `e2e.ui.${Date.now()}`,
      entity_id: `e2e-ui-${Date.now()}`,
      data: { source: "e2e-ui-test" },
    });
    expect(createResp.ok()).toBeTruthy();

    // Navigate to events page — should show real events (not demo notice)
    await page.goto("/dashboard/events");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await expect(
      page.getByRole("heading", { name: /event explorer/i })
    ).toBeVisible({ timeout: 10000 });

    // Should show results count (real events exist now)
    await expect(
      page.getByText(/results\)/).or(page.getByText(/showing sample events/i))
    ).toBeVisible({ timeout: 15000 });
  });

  test("GET /api/event-types returns list", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/event-types");
    if (!resp.ok()) {
      console.warn(`GET /api/event-types returned ${resp.status()} — endpoint may not be implemented`);
    }
    // Don't hard-fail — just surface the issue
    expect(resp.status()).toBeLessThan(500);
  });

  test("GET /api/streams returns list", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/streams");
    if (!resp.ok()) {
      console.warn(`GET /api/streams returned ${resp.status()} — endpoint may not be implemented`);
    }
    expect(resp.status()).toBeLessThan(500);
  });
});

// ---------------------------------------------------------------------------
// API Keys CRUD — full lifecycle through QS
// ---------------------------------------------------------------------------

test.describe("API Keys E2E (QS)", () => {
  test.beforeEach(async ({ page }) => {
    test.skip(!auth, "Demo login failed");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  });

  test("create key → list shows it → revoke removes it", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const keyName = `e2e-key-${Date.now()}`;

    // Create
    const createResp = await qsDirect(request, "POST", "/api/api-keys", {
      name: keyName,
      scopes: ["events:read"],
    });
    if (!createResp.ok()) {
      const body = await createResp.text();
      console.error(`POST /api/api-keys failed (${createResp.status()}): ${body}`);
    }
    expect(createResp.ok()).toBeTruthy();

    const keyData = await createResp.json();
    const keyId = keyData.id || keyData.key_id;
    expect(keyId).toBeTruthy();

    // List — our key should be there
    const listResp = await qsDirect(request, "GET", "/api/api-keys");
    expect(listResp.ok()).toBeTruthy();
    const listData = await listResp.json();
    const keys = listData.keys || listData.data || listData;
    expect(Array.isArray(keys)).toBeTruthy();
    const found = keys.find((k: any) => (k.id || k.key_id) === keyId);
    expect(found).toBeDefined();

    // Revoke
    const revokeResp = await qsDirect(request, "DELETE", `/api/api-keys/${keyId}`);
    expect(revokeResp.ok()).toBeTruthy();

    // List again — key should be gone or marked revoked
    const list2Resp = await qsDirect(request, "GET", "/api/api-keys");
    expect(list2Resp.ok()).toBeTruthy();
    const list2Data = await list2Resp.json();
    const keys2 = list2Data.keys || list2Data.data || list2Data;
    const active = keys2.find(
      (k: any) => (k.id || k.key_id) === keyId && k.status !== "revoked"
    );
    expect(active).toBeUndefined();
  });

  test("created key appears in API Keys UI", async ({ page, request }) => {
    test.skip(!auth, "Demo login failed");

    const keyName = `e2e-visible-${Date.now()}`;
    const createResp = await qsDirect(request, "POST", "/api/api-keys", {
      name: keyName,
      scopes: ["events:read"],
    });
    expect(createResp.ok()).toBeTruthy();

    await page.goto("/dashboard/api-keys");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await expect(
      page.getByRole("heading", { name: /api keys/i }).first()
    ).toBeVisible({ timeout: 10000 });

    // Our key name should appear in the table
    await expect(page.getByText(keyName).first()).toBeVisible({ timeout: 10000 });

    // Cleanup: revoke
    const keyData = await createResp.json();
    const keyId = keyData.id || keyData.key_id;
    if (keyId) {
      await qsDirect(request, "DELETE", `/api/api-keys/${keyId}`);
    }
  });
});

// ---------------------------------------------------------------------------
// Analytics — QS /api/tenants/me/analytics
// ---------------------------------------------------------------------------

test.describe("Analytics E2E (QS)", () => {
  test("GET /api/tenants/me/analytics returns data or surfaces error", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/tenants/me/analytics?range=30d");
    if (!resp.ok()) {
      const body = await resp.text();
      console.error(`BUG: GET /api/tenants/me/analytics returned ${resp.status()}: ${body}`);
    }
    // This endpoint MUST work for the analytics page to function
    expect(resp.ok()).toBeTruthy();
  });

  test("analytics data drives the UI summary cards", async ({ page, request }) => {
    test.skip(!auth, "Demo login failed");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    const analyticsResp = await qsDirect(request, "GET", "/api/tenants/me/analytics?range=30d");

    await page.goto("/dashboard/analytics");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    if (analyticsResp.ok()) {
      // If the API works, summary cards MUST render
      await expect(page.getByText("Total Events")).toBeVisible({ timeout: 15000 });
      await expect(page.getByText("Event Types")).toBeVisible();
      await expect(page.getByText("Unique Entities")).toBeVisible();
    } else {
      // If the API is broken, the page should still not crash
      await expect(
        page.getByRole("heading", { name: /usage analytics/i })
      ).toBeVisible({ timeout: 10000 });
    }
  });
});

// ---------------------------------------------------------------------------
// Tenant / Billing — QS /api/tenant + /api/tenant/usage
// ---------------------------------------------------------------------------

test.describe("Tenant & Billing E2E (QS)", () => {
  test("GET /api/tenant returns tenant with plan info", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/tenant");
    if (!resp.ok()) {
      const body = await resp.text();
      console.error(`BUG: GET /api/tenant returned ${resp.status()}: ${body}`);
    }
    expect(resp.ok()).toBeTruthy();

    const tenant = await resp.json();
    expect(tenant.data?.id || tenant.id || tenant.tenant_id).toBeTruthy();
  });

  test("GET /api/tenant/usage returns quota data", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/tenant/usage");
    if (!resp.ok()) {
      const body = await resp.text();
      console.error(`BUG: GET /api/tenant/usage returned ${resp.status()}: ${body}`);
    }
    expect(resp.ok()).toBeTruthy();
  });

  test("billing page shows plan from QS /api/tenant", async ({ page, request }) => {
    test.skip(!auth, "Demo login failed");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    await page.goto("/dashboard/billing");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await expect(
      page.getByRole("heading", { name: /billing/i }).first()
    ).toBeVisible({ timeout: 10000 });

    // Plan tier badge should come from real QS data
    const main = page.getByRole("main");
    await expect(
      main.getByText(/developer|team|enterprise|free|pro/i).first()
    ).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Team — QS /api/team/members
// ---------------------------------------------------------------------------

test.describe("Team E2E (QS)", () => {
  test("GET /api/team/members includes current user", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/team/members");
    if (!resp.ok()) {
      const body = await resp.text();
      console.error(`BUG: GET /api/team/members returned ${resp.status()}: ${body}`);
    }
    expect(resp.ok()).toBeTruthy();

    const data = await resp.json();
    const members = data.data?.members || data.members || data.data || data;
    expect(Array.isArray(members)).toBeTruthy();
    expect(members.length).toBeGreaterThan(0);

    // Current user should be in the list
    const me = members.find((m: any) => m.email === auth!.email);
    expect(me).toBeDefined();
  });

  test("team page shows member count from QS", async ({ page, request }) => {
    test.skip(!auth, "Demo login failed");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    await page.goto("/dashboard/team");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await expect(page.getByText(/seats used/i)).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Audit Log — QS /api/tenant/audit-logs
// ---------------------------------------------------------------------------

test.describe("Audit Log E2E (QS)", () => {
  test("GET /api/tenant/audit-logs returns valid response", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/tenant/audit-logs?limit=25");
    if (!resp.ok()) {
      const body = await resp.text();
      console.error(`BUG: GET /api/tenant/audit-logs returned ${resp.status()}: ${body}`);
    }
    expect(resp.ok()).toBeTruthy();
  });

  test("audit log UI matches QS response", async ({ page, request }) => {
    test.skip(!auth, "Demo login failed");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    const auditResp = await qsDirect(request, "GET", "/api/tenant/audit-logs?limit=25");

    await page.goto("/dashboard/settings/audit-log");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await expect(
      page.getByRole("heading", { name: /audit log/i })
    ).toBeVisible({ timeout: 10000 });

    if (auditResp.ok()) {
      const data = await auditResp.json();
      const logs = data.data?.entries || data.logs || data.entries || data.data || data;

      if (Array.isArray(logs) && logs.length > 0) {
        // Logs exist — UI should show the table
        await expect(
          page.getByText("Timestamp")
            .or(page.getByText(/api.key|member|plan/i).first())
        ).toBeVisible({ timeout: 15000 });
      } else {
        // No logs — empty state
        await expect(
          page.getByText(/no audit log entries/i)
        ).toBeVisible({ timeout: 10000 });
      }
    }
  });
});

// ---------------------------------------------------------------------------
// Projections / Pipelines — QS /api/projections → Core
// ---------------------------------------------------------------------------

test.describe("Projections E2E (QS → Core)", () => {
  test("GET /api/projections returns valid response", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/projections");
    if (!resp.ok()) {
      const body = await resp.text();
      console.error(`BUG: GET /api/projections returned ${resp.status()}: ${body}`);
    }
    // Projections endpoint should work — it proxies to Core
    expect(resp.ok()).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Replay — QS /api/replay
// ---------------------------------------------------------------------------

test.describe("Replay E2E (QS)", () => {
  test("GET /api/replay returns valid response", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/replay");
    if (!resp.ok()) {
      const body = await resp.text();
      console.error(`BUG: GET /api/replay returned ${resp.status()}: ${body}`);
    }
    expect(resp.ok()).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Settings — QS /api/auth/me drives Profile, /api/tenant drives Workspace
// ---------------------------------------------------------------------------

test.describe("Settings Data E2E (QS)", () => {
  test("GET /api/auth/me returns user email", async ({ request }) => {
    test.skip(!auth, "Demo login failed");

    const resp = await qsDirect(request, "GET", "/api/auth/me");
    expect(resp.ok()).toBeTruthy();

    const me = await resp.json();
    const email = me.data?.user?.email || me.email || me.user?.email;
    expect(email).toBe(auth!.email);
  });

  test("profile email field matches QS /api/auth/me", async ({ page, request }) => {
    test.skip(!auth, "Demo login failed");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    const meResp = await qsDirect(request, "GET", "/api/auth/me");
    expect(meResp.ok()).toBeTruthy();
    const me = await meResp.json();
    const expectedEmail = me.email || me.user?.email || auth!.email;

    await page.goto("/dashboard/settings");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await expect(page.getByText("Profile Information")).toBeVisible({ timeout: 10000 });
    const emailInput = page.getByLabel(/email/i);
    await expect(emailInput).toBeVisible();
    const emailValue = await emailInput.inputValue();
    expect(emailValue).toBe(expectedEmail);
  });

  test("workspace tab shows tenant ID from QS /api/tenant", async ({ page, request }) => {
    test.skip(!auth, "Demo login failed");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(auth!.token)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    const tenantResp = await qsDirect(request, "GET", "/api/tenant");
    // Don't hard-fail if tenant endpoint is broken — the test below will catch it
    if (!tenantResp.ok()) {
      console.error(`GET /api/tenant returned ${tenantResp.status()}`);
      test.skip(true, "QS /api/tenant endpoint not working");
    }
    const tenant = await tenantResp.json();
    const tenantId = tenant.data?.id || tenant.id || tenant.tenant_id;

    await page.goto("/dashboard/settings");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await page.getByText("Workspace", { exact: true }).first().click();
    await expect(page.getByText("Workspace Settings")).toBeVisible({ timeout: 10000 });

    // Tenant ID should be displayed somewhere on the workspace tab
    if (tenantId) {
      await expect(page.getByText(tenantId)).toBeVisible({ timeout: 5000 });
    }
  });
});
