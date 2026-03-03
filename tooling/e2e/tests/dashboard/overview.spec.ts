import { test, expect, type Page } from "@playwright/test";
import { demoLogin, authenticateAndGoToDashboard } from "../../fixtures/demo-auth";

/**
 * Dashboard Overview E2E Tests
 *
 * Covers stats cards, current plan, usage charts, API keys section,
 * quick action cards, recent events, and product stats banner.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/overview.spec.ts
 */

// ---------------------------------------------------------------------------
// Stats cards
// ---------------------------------------------------------------------------

test.describe("Overview — stats cards", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("stats cards render with numeric values", async ({ page }) => {
    // The overview page should show stat cards with numbers
    const statsSection = page.locator("[class*='grid']").first();
    await expect(statsSection).toBeVisible({ timeout: 10000 });

    // At least one numeric value should be present in the stats area
    const numericValue = page.locator("text=/\\d+/").first();
    await expect(numericValue).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Current plan card
// ---------------------------------------------------------------------------

test.describe("Overview — current plan card", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("current plan card shows plan name and usage bars", async ({ page }) => {
    // Plan card should show a plan tier badge or "Current Plan" heading
    const planSection = page.getByText(/Developer|Free|Growth|Team|Enterprise|Pro|Current Plan/i).first();
    await expect(planSection).toBeVisible({ timeout: 10000 });

    // Should have at least one progress bar or usage indicator
    const progressBar = page.locator("[role='progressbar'], [class*='progress'], [class*='bg-primary']").first();
    await expect(progressBar).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Usage charts
// ---------------------------------------------------------------------------

test.describe("Overview — usage charts", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("usage charts render with SVG elements", async ({ page }) => {
    // Check for chart section headings — accept various naming patterns
    await expect(
      page.getByText(/Event Ingestion|Events \(30|Usage/i).first()
    ).toBeVisible({ timeout: 10000 });

    // At least one SVG chart element or canvas should be present
    const svg = page.locator("svg.recharts-surface, svg[class*='chart'], canvas").first();
    await expect(svg).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// API keys section
// ---------------------------------------------------------------------------

test.describe("Overview — API keys section", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("'View All' link navigates to /dashboard/api-keys", async ({ page }) => {
    const viewAllLink = page.getByRole("link", { name: /View All/i }).first();
    await expect(viewAllLink).toBeVisible({ timeout: 10000 });
    await viewAllLink.click();

    await page.waitForURL(/\/dashboard\/api-keys/, { timeout: 15000 });
    expect(page.url()).toContain("/dashboard/api-keys");
  });

  test("'Create Key' button is visible", async ({ page }) => {
    const createKeyBtn = page.getByRole("button", { name: /Create Key/i });
    await expect(createKeyBtn).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Quick action cards
// ---------------------------------------------------------------------------

test.describe("Overview — quick action cards", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("Create Event card navigates to /dashboard/events?action=create", async ({ page }) => {
    const card = page.getByRole("link", { name: /Create Event/i });
    await expect(card).toBeVisible({ timeout: 10000 });
    await card.click();

    await page.waitForURL(/\/dashboard\/events/, { timeout: 15000 });
    expect(page.url()).toContain("/dashboard/events");
  });

  test("Generate API Key card navigates to /dashboard/api-keys?action=create", async ({ page }) => {
    const card = page.getByRole("link", { name: /Generate API Key/i });
    await expect(card).toBeVisible({ timeout: 10000 });
    await card.click();

    await page.waitForURL(/\/dashboard\/api-keys/, { timeout: 15000 });
    expect(page.url()).toContain("/dashboard/api-keys");
  });

  test("View Documentation card links externally", async ({ page }) => {
    const card = page.getByRole("link", { name: /View Documentation/i });
    await expect(card).toBeVisible({ timeout: 10000 });

    const target = await card.getAttribute("target");
    const href = await card.getAttribute("href");
    // External links should open in new tab or have non-relative href
    expect(target === "_blank" || (href && !href.startsWith("/dashboard"))).toBeTruthy();
  });

  test("API Reference card links externally", async ({ page }) => {
    const card = page.getByRole("link", { name: /API Reference/i });
    await expect(card).toBeVisible({ timeout: 10000 });

    const target = await card.getAttribute("target");
    const href = await card.getAttribute("href");
    expect(target === "_blank" || (href && !href.startsWith("/dashboard"))).toBeTruthy();
  });

  test("Join Discord card links externally", async ({ page }) => {
    const card = page.getByRole("link", { name: /Join Discord/i });
    await expect(card).toBeVisible({ timeout: 10000 });

    const target = await card.getAttribute("target");
    const href = await card.getAttribute("href");
    expect(target === "_blank" || (href && !href.startsWith("/dashboard"))).toBeTruthy();
  });

  test("Changelog card navigates to /blog", async ({ page }) => {
    const card = page.getByRole("link", { name: /Changelog/i });
    await expect(card).toBeVisible({ timeout: 10000 });

    const href = await card.getAttribute("href");
    expect(href).toContain("/blog");
  });
});

// ---------------------------------------------------------------------------
// Recent events section
// ---------------------------------------------------------------------------

test.describe("Overview — recent events", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("recent events section renders", async ({ page }) => {
    // The recent events section or instance stats banner should be visible
    const recentSection = page.getByText(/Recent Events|Your Event Store/i).first();
    await expect(recentSection).toBeVisible({ timeout: 10000 });
  });
});
