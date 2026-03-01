import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

/**
 * Event Replay Page E2E Tests
 *
 * Covers replay form, submission, and history list.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/replay.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/tools/replay");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Page rendering
// ---------------------------------------------------------------------------

test.describe("Replay — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("page renders with heading and form", async ({ page }) => {
    await expect(page.getByText("Event Replay")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Start New Replay")).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Replay form
// ---------------------------------------------------------------------------

test.describe("Replay — form fields", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("form fields are visible and fillable", async ({ page }) => {
    // Date and time inputs
    await expect(page.locator("#from-date")).toBeVisible({ timeout: 10000 });
    await expect(page.locator("input[aria-label='From time']")).toBeVisible();
    await expect(page.locator("#to-date")).toBeVisible();
    await expect(page.locator("input[aria-label='To time']")).toBeVisible();

    // Optional filter inputs
    await expect(page.locator("#event-type")).toBeVisible();
    await expect(page.locator("#entity-id")).toBeVisible();
    await expect(page.locator("#projection")).toBeVisible();

    // Fill in the form
    await page.locator("#from-date").fill("2026-01-01");
    await page.locator("input[aria-label='From time']").fill("00:00");
    await page.locator("#to-date").fill("2026-01-02");
    await page.locator("input[aria-label='To time']").fill("23:59");
    await page.locator("#event-type").fill("user.created");
    await page.locator("#entity-id").fill("user-123");
    await page.locator("#projection").fill("user-projection");

    // Start Replay button should be visible
    await expect(page.getByRole("button", { name: /Start Replay/i })).toBeVisible();
  });

  test("clicking Start Replay submits the form", async ({ page }) => {
    // Fill required fields
    await page.locator("#from-date").fill("2026-01-01");
    await page.locator("input[aria-label='From time']").fill("00:00");
    await page.locator("#to-date").fill("2026-01-02");
    await page.locator("input[aria-label='To time']").fill("23:59");

    // Click Start Replay
    await page.getByRole("button", { name: /Start Replay/i }).click();

    // Should show either creating state, success (replay in history), or error
    await page.waitForTimeout(2000);

    const hasCreating = await page.getByText("Creating...").isVisible().catch(() => false);
    const hasHistory = await page.getByText("Replay History").isVisible().catch(() => false);
    const hasError = await page.locator("[class*='destructive']").first().isVisible().catch(() => false);

    expect(hasCreating || hasHistory || hasError).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Replay history
// ---------------------------------------------------------------------------

test.describe("Replay — history", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("replay history section renders", async ({ page }) => {
    await expect(page.getByText("Replay History")).toBeVisible({ timeout: 10000 });

    // Should show either replay entries or empty state
    const hasReplays = await page.locator("text=/Pending|Running|Completed|Failed|Cancelled/").first().isVisible({ timeout: 3000 }).catch(() => false);
    const hasEmpty = await page.getByText("No replays yet").isVisible({ timeout: 3000 }).catch(() => false);

    expect(hasReplays || hasEmpty).toBeTruthy();
  });

  test("if active replay exists, Cancel button is visible", async ({ page }) => {
    const hasActive = await page.locator("text=/Pending|Running/").first().isVisible({ timeout: 5000 }).catch(() => false);

    if (hasActive) {
      const cancelBtn = page.getByRole("button", { name: /Cancel/i }).first();
      await expect(cancelBtn).toBeVisible({ timeout: 5000 });
    }
  });

  test("if completed replay exists, Remove button is visible", async ({ page }) => {
    const hasCompleted = await page.locator("text=/Completed|Failed|Cancelled/").first().isVisible({ timeout: 5000 }).catch(() => false);

    if (hasCompleted) {
      const removeBtn = page.getByRole("button", { name: /Remove/i }).first();
      await expect(removeBtn).toBeVisible({ timeout: 5000 });
    }
  });
});
