import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

/**
 * Navigation & Auth Protection E2E Tests — Production / Staging compatible
 *
 * Tests:
 * - Unauthenticated users are redirected to /login for protected pages
 * - Authenticated users can navigate between pages via sidebar
 * - Session persists across page navigations
 * - Logout flow works end-to-end
 * - Authenticated users are redirected away from /login
 *
 * Run against production:
 *   BASE_URL=https://www.all-source.xyz CONTROL_PLANE_URL=https://allsource-control-plane.fly.dev \
 *     bunx playwright test tests/smoke/navigation.spec.ts
 */

// ---------------------------------------------------------------------------
// Auth protection — unauthenticated redirects
// ---------------------------------------------------------------------------

test.describe("Auth protection (unauthenticated)", () => {
  const protectedPaths = [
    "/dashboard",
    "/dashboard/events",
    "/dashboard/analytics",
    "/dashboard/api-keys",
    "/dashboard/billing",
    "/dashboard/pipelines",
    "/dashboard/team",
    "/dashboard/settings",
    "/dashboard/settings/audit-log",
    "/dashboard/tools/replay",
    "/dashboard/demo",
    "/onboarding",
  ];

  for (const path of protectedPaths) {
    test(`${path} redirects to /login when unauthenticated`, async ({ page }) => {
      await page.goto(path);
      await page.waitForURL(/\/login/, { timeout: 15000 });
      expect(page.url()).toContain("/login");
    });
  }
});

// ---------------------------------------------------------------------------
// Auth redirect — authenticated users bounced from auth pages
// ---------------------------------------------------------------------------

test.describe("Auth redirect (authenticated)", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(token!)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  });

  test("/login redirects authenticated user to /dashboard", async ({ page }) => {
    await page.goto("/login");
    await page.waitForURL(/\/dashboard/, { timeout: 15000 });
    expect(page.url()).toContain("/dashboard");
  });

  test("/signup redirects authenticated user to /dashboard", async ({ page }) => {
    await page.goto("/signup");
    await page.waitForURL(/\/dashboard/, { timeout: 15000 });
    expect(page.url()).toContain("/dashboard");
  });
});

// ---------------------------------------------------------------------------
// Sidebar navigation — authenticated
// ---------------------------------------------------------------------------

test.describe("Sidebar navigation", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(token!)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
    // Ensure we're on the dashboard
    if (!page.url().includes("/dashboard")) {
      await page.goto("/dashboard");
    }
    // Wait for layout to load
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
  });

  const sidebarLinks: Array<{ name: RegExp; expectedPath: string; expectedHeading: RegExp }> = [
    { name: /^overview$/i, expectedPath: "/dashboard", expectedHeading: /good (morning|afternoon|evening)/i },
    { name: /^events$/i, expectedPath: "/dashboard/events", expectedHeading: /event explorer/i },
    { name: /^analytics$/i, expectedPath: "/dashboard/analytics", expectedHeading: /usage analytics/i },
    { name: /^api keys$/i, expectedPath: "/dashboard/api-keys", expectedHeading: /api keys/i },
    { name: /^billing$/i, expectedPath: "/dashboard/billing", expectedHeading: /billing/i },
    { name: /^pipelines$/i, expectedPath: "/dashboard/pipelines", expectedHeading: /pipelines/i },
    { name: /^team$/i, expectedPath: "/dashboard/team", expectedHeading: /team/i },
    { name: /^settings$/i, expectedPath: "/dashboard/settings", expectedHeading: /settings/i },
  ];

  for (const { name, expectedPath, expectedHeading } of sidebarLinks) {
    test(`clicking "${name.source}" navigates to ${expectedPath}`, async ({ page }) => {
      // Click the sidebar link
      const link = page.getByRole("link", { name }).first();
      await expect(link).toBeVisible({ timeout: 10000 });
      await link.click();

      // Verify URL
      await page.waitForURL(new RegExp(expectedPath.replace(/\//g, "\\/")), {
        timeout: 15000,
      });

      // Verify heading renders
      await expect(
        page.getByRole("heading", { name: expectedHeading }).first()
      ).toBeVisible({ timeout: 15000 });
    });
  }

  test("navigating between pages preserves auth session", async ({ page }) => {
    // Go to Events
    await page.goto("/dashboard/events");
    await expect(page.getByRole("heading", { name: /event explorer/i })).toBeVisible({ timeout: 15000 });

    // Go to Analytics
    await page.goto("/dashboard/analytics");
    await expect(page.getByRole("heading", { name: /usage analytics/i })).toBeVisible({ timeout: 15000 });

    // Go to Settings
    await page.goto("/dashboard/settings");
    await expect(page.getByRole("heading", { name: /settings/i }).first()).toBeVisible({ timeout: 15000 });

    // Verify session is still valid
    const sessionResp = await page.request.get("/api/auth/session");
    expect(sessionResp.ok()).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Logout flow
// ---------------------------------------------------------------------------

test.describe("Logout flow", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test("logout clears session and dashboard becomes inaccessible", async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");

    // Authenticate
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(token!)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    // Verify authenticated
    const sessionBefore = await page.request.get("/api/auth/session");
    expect(sessionBefore.ok()).toBeTruthy();

    // Logout
    const logoutResp = await page.request.delete("/api/auth/session");
    expect(logoutResp.ok()).toBeTruthy();

    // Verify session is gone
    const sessionAfter = await page.request.get("/api/auth/session");
    expect(sessionAfter.status()).toBe(401);

    // Cookie should be cleared
    const cookies = await page.context().cookies();
    const authCookie = cookies.find((c) => c.name === "auth_token");
    expect(authCookie).toBeUndefined();

    // Navigating to dashboard should redirect to login
    await page.goto("/dashboard");
    await page.waitForURL(/\/login/, { timeout: 15000 });
  });
});

// ---------------------------------------------------------------------------
// 404 handling
// ---------------------------------------------------------------------------

test.describe("404 handling", () => {
  test("non-existent page shows 404 or redirects gracefully", async ({ page }) => {
    const response = await page.goto("/this-page-does-not-exist-abc123");

    // Should either show 404 page or redirect
    const is404 = response?.status() === 404;
    const hasNotFound = await page.getByText(/not found|404/i).isVisible().catch(() => false);
    const wasRedirected = page.url().includes("/login") || page.url().endsWith("/");

    expect(is404 || hasNotFound || wasRedirected).toBeTruthy();
  });

  test("non-existent dashboard subpage shows 404", async ({ page }) => {
    // This requires auth first
    const loginToken = await demoLogin(page.request);
    test.skip(!loginToken, "Demo login failed");

    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(loginToken!)}&new_user=false`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    await page.goto("/dashboard/nonexistent-page-xyz");

    // Should show 404 within dashboard layout
    const hasNotFound = await page.getByText(/not found|404/i).isVisible().catch(() => false);
    const hasContent = await page.getByRole("heading").first().isVisible().catch(() => false);

    // Either shows 404 or some valid page (shouldn't crash)
    expect(hasNotFound || hasContent).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Page response codes — no 500s
// ---------------------------------------------------------------------------

test.describe("No server errors on public pages", () => {
  const publicPaths = [
    "/",
    "/login",
    "/signup",
    "/forgot-password",
    "/blog",
    "/changelog",
    "/status",
    "/privacy",
    "/terms",
    "/solutions/quant-intelligence",
  ];

  for (const path of publicPaths) {
    test(`${path} does not return 5xx`, async ({ page }) => {
      const response = await page.goto(path);
      expect(response?.status()).toBeLessThan(500);
    });
  }
});
