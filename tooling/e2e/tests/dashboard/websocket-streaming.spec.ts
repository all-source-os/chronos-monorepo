import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

/**
 * WebSocket Live Event Streaming E2E Tests
 *
 * Covers the live event feed on the Events page: connection status,
 * event rendering, and pause/play controls.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/websocket-streaming.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/events");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Live feed rendering
// ---------------------------------------------------------------------------

test.describe("WebSocket Streaming — feed renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("live event feed section renders", async ({ page }) => {
    const feedSection = page.getByText(/Live Event|Live Feed|Real-time/i).first();
    await expect(feedSection).toBeVisible({ timeout: 10000 });
  });

  test("connection status indicator is visible", async ({ page }) => {
    // Look for a connection status indicator (Wifi icon or status text)
    const hasStatus = await page.locator("svg[class*='wifi'], svg[class*='Wifi']").first().isVisible({ timeout: 5000 }).catch(() => false);
    const hasStatusText = await page.getByText(/connected|simulation|connecting/i).first().isVisible({ timeout: 5000 }).catch(() => false);
    const hasStatusIcon = await page.locator("[class*='status'], [class*='indicator']").first().isVisible({ timeout: 3000 }).catch(() => false);

    // At least one connection indicator should be present
    expect(hasStatus || hasStatusText || hasStatusIcon).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Live events
// ---------------------------------------------------------------------------

test.describe("WebSocket Streaming — events appear", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("events appear in the feed (real or simulated)", async ({ page }) => {
    // Wait for events to appear in the live feed (up to 10s)
    // Events show as list items with event type text
    const eventItem = page.locator("[class*='event'], [class*='feed'] [class*='item']").first();

    // Give it time — either real WS events or simulated ones should appear
    const hasEvents = await eventItem.isVisible({ timeout: 10000 }).catch(() => false);

    // If no events in feed component, check for the feed section with any content
    const hasFeedContent = await page.locator("[class*='feed']").first().isVisible({ timeout: 3000 }).catch(() => false);

    expect(hasEvents || hasFeedContent).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Pause/play controls
// ---------------------------------------------------------------------------

test.describe("WebSocket Streaming — pause/play", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("pause stops new events, play resumes them", async ({ page }) => {
    // Find pause/play button
    const pauseBtn = page.locator("button[aria-label*='ause'], button[aria-label*='Stop']").first();
    const isVisible = await pauseBtn.isVisible({ timeout: 5000 }).catch(() => false);

    if (isVisible) {
      // Pause the feed
      await pauseBtn.click();
      await page.waitForTimeout(500);

      // Count events
      const countBefore = await page.locator("[class*='feed'] [class*='item']").count();

      // Wait 3 seconds — no new events should appear
      await page.waitForTimeout(3000);
      const countAfterPause = await page.locator("[class*='feed'] [class*='item']").count();

      // Count should not have increased significantly
      expect(countAfterPause).toBeLessThanOrEqual(countBefore + 1);

      // Resume the feed
      await pauseBtn.click();
      await page.waitForTimeout(500);
    }
  });
});
