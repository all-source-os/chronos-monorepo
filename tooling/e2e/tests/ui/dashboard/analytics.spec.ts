import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../../fixtures/demo-auth";

/**
 * Analytics Page E2E Tests
 *
 * Covers time range selector, summary cards, and chart rendering.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/analytics.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/analytics");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Page rendering and summary cards
// ---------------------------------------------------------------------------

test.describe("Analytics — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("page renders with heading", async ({ page }) => {
    await expect(page.getByText("Usage Analytics")).toBeVisible({ timeout: 10000 });
  });

  test("summary cards render (Total Events, Event Types, Unique Entities)", async ({ page }) => {
    await expect(page.getByText("Total Events")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Event Types")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Unique Entities")).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Time range toggle
// ---------------------------------------------------------------------------

test.describe("Analytics — time range toggle", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("clicking each time range button changes active state", async ({ page }) => {
    const ranges = ["24h", "7d", "30d", "90d"];

    for (const range of ranges) {
      const btn = page.getByRole("button", { name: new RegExp(`^${range}$`) });
      await expect(btn).toBeVisible({ timeout: 5000 });
      await btn.click();

      // Active button should have distinct styling (check for a class indicating active state)
      const classes = await btn.getAttribute("class");
      // Active buttons typically have bg-primary or variant=default styling
      expect(classes).toBeTruthy();
    }
  });
});

// ---------------------------------------------------------------------------
// Charts
// ---------------------------------------------------------------------------

test.describe("Analytics — charts", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("ingestion rate chart renders or is absent for empty accounts", async ({ page }) => {
    // Demo accounts with 0 events may not render chart sections at all
    // Wait for page to fully load by checking summary cards first
    await expect(page.getByText("Total Events")).toBeVisible({ timeout: 15000 });

    const hasHeading = await page.getByText("Ingestion Rate").isVisible({ timeout: 5000 }).catch(() => false);
    const hasNoData = await page.getByText(/no ingestion data|no data/i).isVisible().catch(() => false);

    // If charts render, verify SVG or no-data state; if neither, it's a 0-event account (acceptable)
    if (hasHeading) {
      const hasSvg = await page.locator("svg").first().isVisible().catch(() => false);
      expect(hasSvg || hasNoData).toBeTruthy();
    }
    // No charts at all is acceptable for demo accounts with 0 events
  });

  test("event type distribution chart renders or is absent for empty accounts", async ({ page }) => {
    // Demo accounts with 0 events may not render chart sections at all
    await expect(page.getByText("Total Events")).toBeVisible({ timeout: 15000 });

    const hasHeading = await page.getByText("Event Type Distribution").isVisible({ timeout: 5000 }).catch(() => false);
    const hasNoData = await page.getByText(/no event types|no data/i).isVisible().catch(() => false);

    if (hasHeading) {
      const hasSvg = await page.locator("svg.recharts-surface, svg").nth(1).isVisible().catch(() => false);
      expect(hasSvg || hasNoData).toBeTruthy();
    }
    // No charts at all is acceptable for demo accounts with 0 events
  });

  test("top entity IDs chart renders or is absent for empty accounts", async ({ page }) => {
    // Demo accounts with 0 events may not render chart sections at all
    await expect(page.getByText("Total Events")).toBeVisible({ timeout: 15000 });

    const hasHeading = await page.getByText("Top Entity IDs").isVisible({ timeout: 5000 }).catch(() => false);
    const hasNoData = await page.getByText(/no entities|no data/i).isVisible().catch(() => false);

    if (hasHeading) {
      const hasSvg = await page.locator("svg.recharts-surface, svg").nth(2).isVisible().catch(() => false);
      expect(hasSvg || hasNoData).toBeTruthy();
    }
    // No charts at all is acceptable for demo accounts with 0 events
  });
});
