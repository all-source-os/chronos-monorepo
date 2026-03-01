import { test, expect, type Page } from "@playwright/test";
import { demoLogin, authenticateAndGoToDashboard } from "../../fixtures/demo-auth";

/**
 * Sidebar Navigation E2E Tests
 *
 * Verifies all 11 sidebar nav links route correctly, active state
 * highlighting, collapse/expand toggle, and logo link.
 *
 * Requires Control Plane for demo login.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/sidebar-navigation.spec.ts
 */

// All 11 sidebar navigation items matching apps/web sidebar component
const sidebarLinks = [
  { name: "Overview", href: "/dashboard" },
  { name: "Events", href: "/dashboard/events" },
  { name: "Pipelines", href: "/dashboard/pipelines" },
  { name: "Analytics", href: "/dashboard/analytics" },
  { name: "Demo Zone", href: "/dashboard/demo" },
  { name: "API Keys", href: "/dashboard/api-keys" },
  { name: "Team", href: "/dashboard/team" },
  { name: "Replay", href: "/dashboard/tools/replay" },
  { name: "Audit Log", href: "/dashboard/settings/audit-log" },
  { name: "Billing", href: "/dashboard/billing" },
  { name: "Settings", href: "/dashboard/settings" },
] as const;

// ---------------------------------------------------------------------------
// Sidebar link navigation — all 11 links
// ---------------------------------------------------------------------------

test.describe("Sidebar navigation — link routing", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  for (const { name, href } of sidebarLinks) {
    test(`clicking "${name}" navigates to ${href}`, async ({ page }) => {
      const link = page.getByRole("link", { name: new RegExp(`^${name}$`, "i") }).first();
      await expect(link).toBeVisible({ timeout: 10000 });
      await link.click();

      await page.waitForURL(new RegExp(href.replace(/\//g, "\\/")), {
        timeout: 15000,
      });
      expect(page.url()).toContain(href);
    });
  }
});

// ---------------------------------------------------------------------------
// Active state highlighting
// ---------------------------------------------------------------------------

test.describe("Sidebar navigation — active state", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("current page link has active styling", async ({ page }) => {
    // On /dashboard, "Overview" should be active (has bg-primary/10 text-primary)
    const overviewLink = page.getByRole("link", { name: /^Overview$/i }).first();
    await expect(overviewLink).toBeVisible({ timeout: 10000 });

    // Active link gets text-primary class — check computed color differs from inactive links
    const overviewClasses = await overviewLink.getAttribute("class");
    expect(overviewClasses).toContain("text-primary");
    expect(overviewClasses).toContain("bg-primary");

    // An inactive link should have text-muted-foreground
    const eventsLink = page.getByRole("link", { name: /^Events$/i }).first();
    const eventsClasses = await eventsLink.getAttribute("class");
    expect(eventsClasses).toContain("text-muted-foreground");
  });

  test("active state updates when navigating to a different page", async ({ page }) => {
    // Navigate to Events
    const eventsLink = page.getByRole("link", { name: /^Events$/i }).first();
    await eventsLink.click();
    await page.waitForURL(/\/dashboard\/events/, { timeout: 15000 });

    // Events should now be active
    const eventsClasses = await eventsLink.getAttribute("class");
    expect(eventsClasses).toContain("text-primary");

    // Overview should now be inactive
    const overviewLink = page.getByRole("link", { name: /^Overview$/i }).first();
    const overviewClasses = await overviewLink.getAttribute("class");
    expect(overviewClasses).toContain("text-muted-foreground");
  });
});

// ---------------------------------------------------------------------------
// Sidebar collapse / expand
// ---------------------------------------------------------------------------

test.describe("Sidebar navigation — collapse and expand", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("collapse button collapses the sidebar", async ({ page }) => {
    // Sidebar should start expanded with "Collapse sidebar" button
    const collapseBtn = page.getByRole("button", { name: "Collapse sidebar" });
    await expect(collapseBtn).toBeVisible({ timeout: 10000 });

    await collapseBtn.click();

    // After collapse, the button's aria-label should change to "Expand sidebar"
    const expandBtn = page.getByRole("button", { name: "Expand sidebar" });
    await expect(expandBtn).toBeVisible({ timeout: 5000 });
  });

  test("expand button expands the sidebar back", async ({ page }) => {
    // Collapse first
    const collapseBtn = page.getByRole("button", { name: "Collapse sidebar" });
    await expect(collapseBtn).toBeVisible({ timeout: 10000 });
    await collapseBtn.click();

    // Now expand
    const expandBtn = page.getByRole("button", { name: "Expand sidebar" });
    await expect(expandBtn).toBeVisible({ timeout: 5000 });
    await expandBtn.click();

    // Should be back to "Collapse sidebar"
    await expect(
      page.getByRole("button", { name: "Collapse sidebar" })
    ).toBeVisible({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Logo link
// ---------------------------------------------------------------------------

test.describe("Sidebar navigation — logo link", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("logo link navigates to /dashboard", async ({ page }) => {
    // Navigate away from dashboard first
    await page.goto("/dashboard/events");
    await page.waitForURL(/\/dashboard\/events/, { timeout: 15000 });

    // Click the logo link (contains the AllSource logo icon linking to /dashboard)
    const logoLink = page.locator('a[href="/dashboard"]').filter({ has: page.locator("svg") }).first();
    await expect(logoLink).toBeVisible({ timeout: 10000 });
    await logoLink.click();

    await page.waitForURL(/\/dashboard$/, { timeout: 15000 });
    expect(page.url()).toMatch(/\/dashboard$/);
  });
});
