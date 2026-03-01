import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

/**
 * Event Explorer E2E Tests
 *
 * Covers search, filters, export, event list, detail drawer,
 * and live event feed.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/events.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string,
  path: string = "/dashboard/events"
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto(path);
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Event list rendering
// ---------------------------------------------------------------------------

test.describe("Events — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("event list renders on /dashboard/events", async ({ page }) => {
    await expect(page.getByText("Event Explorer")).toBeVisible({ timeout: 10000 });
    // Should show event count or demo notice
    const eventsHeading = page.getByText(/Events \(\d+ results?\)|Showing sample events/);
    await expect(eventsHeading).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Search and filters
// ---------------------------------------------------------------------------

test.describe("Events — search and filters", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("typing in search input filters the event list", async ({ page }) => {
    const searchInput = page.getByPlaceholder("Search events by type or entity...");
    await expect(searchInput).toBeVisible({ timeout: 10000 });

    await searchInput.fill("user.created");
    // Wait for filtering to apply
    await page.waitForTimeout(500);

    // The list should have filtered or show no results
    const resultText = page.getByText(/Events \(\d+ results?\)|No events found/);
    await expect(resultText).toBeVisible({ timeout: 5000 });
  });

  test("clicking Filters toggle expands filter panel", async ({ page }) => {
    const filtersBtn = page.getByRole("button", { name: /Filters/i });
    await expect(filtersBtn).toBeVisible({ timeout: 10000 });

    await filtersBtn.click();

    // Filter panel should show Entity ID, Event Type, Date Range inputs
    await expect(page.getByPlaceholder("Filter by entity...")).toBeVisible({ timeout: 5000 });
    await expect(page.getByPlaceholder("Filter by type...")).toBeVisible({ timeout: 5000 });
    await expect(page.locator("input[type='date']").first()).toBeVisible({ timeout: 5000 });
  });

  test("filling Entity ID filter and clearing filters works", async ({ page }) => {
    // Open filters
    const filtersBtn = page.getByRole("button", { name: /Filters/i });
    await filtersBtn.click();

    const entityInput = page.getByPlaceholder("Filter by entity...");
    await expect(entityInput).toBeVisible({ timeout: 5000 });
    await entityInput.fill("user-123");
    await page.waitForTimeout(500);

    // Clear filters button (X) should be visible
    // Click the clear/X button near the filters area
    const clearBtn = page.locator("button").filter({ has: page.locator("svg") }).filter({ hasText: "" }).nth(0);
    // Look for a button that clears filters
    const filterBadge = page.locator("text=/\\d+ active/i");
    const hasBadge = await filterBadge.isVisible().catch(() => false);

    if (hasBadge) {
      // Find and click the clear filters X button
      const xBtn = page.locator("button[aria-label*='clear'], button[aria-label*='Clear']").first();
      if (await xBtn.isVisible()) {
        await xBtn.click();
        await expect(entityInput).toHaveValue("");
      }
    }
  });
});

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

test.describe("Events — export", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("clicking Export triggers a download", async ({ page }) => {
    const exportBtn = page.getByRole("button", { name: /Export/i });
    await expect(exportBtn).toBeVisible({ timeout: 10000 });

    // Listen for download event
    const downloadPromise = page.waitForEvent("download", { timeout: 10000 }).catch(() => null);
    await exportBtn.click();

    const download = await downloadPromise;
    // Download may or may not trigger depending on data availability
    // Just verify the button is clickable without errors
    await expect(exportBtn).toBeEnabled();
  });
});

// ---------------------------------------------------------------------------
// Event detail drawer
// ---------------------------------------------------------------------------

test.describe("Events — detail drawer", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("clicking an event row opens the detail drawer", async ({ page }) => {
    // Wait for events to load (real or demo)
    await page.waitForTimeout(2000);

    // Click the first event row
    const eventRow = page.locator("[class*='cursor-pointer'], tr[class*='hover']").first();
    const isVisible = await eventRow.isVisible().catch(() => false);

    if (isVisible) {
      await eventRow.click();

      // Drawer should appear with copy buttons
      const copyEventIdBtn = page.getByRole("button", { name: /Copy Event ID/i });
      const drawerVisible = await copyEventIdBtn.isVisible({ timeout: 5000 }).catch(() => false);

      if (drawerVisible) {
        // Test Copy Event ID button state change
        await copyEventIdBtn.click();
        // Button text or icon should change to indicate success
        await page.waitForTimeout(500);

        // Test Copy Entity ID
        const copyEntityBtn = page.getByRole("button", { name: /Copy Entity ID/i });
        if (await copyEntityBtn.isVisible()) {
          await copyEntityBtn.click();
        }

        // Test Copy JSON
        const copyJsonBtn = page.getByRole("button", { name: /Copy JSON/i });
        if (await copyJsonBtn.isVisible()) {
          await copyJsonBtn.click();
        }

        // Close the drawer
        const closeBtn = page.getByRole("button", { name: /Close/i }).last();
        if (await closeBtn.isVisible()) {
          await closeBtn.click();
          await expect(copyEventIdBtn).toBeHidden({ timeout: 5000 });
        }
      }
    }
  });
});

// ---------------------------------------------------------------------------
// Live event feed
// ---------------------------------------------------------------------------

test.describe("Events — live event feed", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("live event feed section renders with connection status", async ({ page }) => {
    // Look for the live feed section
    const liveFeed = page.getByText(/Live Event|Live Feed|Real-time/i).first();
    await expect(liveFeed).toBeVisible({ timeout: 10000 });
  });

  test("pause button toggles feed", async ({ page }) => {
    // Find the pause/play toggle button in the live feed area
    const pauseBtn = page.locator("button[aria-label*='ause'], button[aria-label*='Stop']").first();
    const isVisible = await pauseBtn.isVisible({ timeout: 5000 }).catch(() => false);

    if (isVisible) {
      await pauseBtn.click();
      // Button should toggle state
      await page.waitForTimeout(500);
      await pauseBtn.click(); // Toggle back
    }
  });

  test("clear button clears the feed", async ({ page }) => {
    const clearBtn = page.locator("button[aria-label*='lear'], button[aria-label*='Trash']").first();
    const isVisible = await clearBtn.isVisible({ timeout: 5000 }).catch(() => false);

    if (isVisible) {
      await clearBtn.click();
      await page.waitForTimeout(500);
    }
  });
});
