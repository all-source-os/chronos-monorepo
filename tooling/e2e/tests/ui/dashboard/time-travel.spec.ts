import { test, expect, type Page } from "@playwright/test";
import { demoLogin, authenticateAndGoToDashboard } from "../../../fixtures/demo-auth";

/**
 * Dashboard Time Travel Picker E2E Tests
 *
 * Covers the time travel popover: trigger button, quick presets,
 * custom date/time, historical mode banner, keyboard shortcuts,
 * and return to present.
 *
 * Requires Control Plane for demo login.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/time-travel.spec.ts
 */

// ---------------------------------------------------------------------------
// Time Travel — popover open/close
// ---------------------------------------------------------------------------

test.describe("Time Travel — popover", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("clicking trigger button opens popover", async ({ page }) => {
    const trigger = page.getByRole("button", { name: "Time Travel" });
    await expect(trigger).toBeVisible({ timeout: 10000 });

    await trigger.click();

    // Popover heading should be visible
    const popoverHeading = page.locator("h3", { hasText: "Time Travel" });
    await expect(popoverHeading).toBeVisible({ timeout: 5000 });

    // Quick presets section should be visible
    await expect(page.getByText("Quick presets")).toBeVisible();
    // Custom date & time section should be visible
    await expect(page.getByText("Custom date & time")).toBeVisible();
  });

  test("Escape closes popover", async ({ page }) => {
    const trigger = page.getByRole("button", { name: "Time Travel" });
    await trigger.click();

    const popoverHeading = page.locator("h3", { hasText: "Time Travel" });
    await expect(popoverHeading).toBeVisible({ timeout: 5000 });

    await page.keyboard.press("Escape");

    await expect(popoverHeading).toBeHidden({ timeout: 5000 });
  });

  test("Cmd+T opens and closes popover", async ({ page }) => {
    // Open with Cmd+T
    await page.keyboard.press("Meta+t");

    const popoverHeading = page.locator("h3", { hasText: "Time Travel" });
    await expect(popoverHeading).toBeVisible({ timeout: 5000 });

    // Close with Cmd+T
    await page.keyboard.press("Meta+t");
    await expect(popoverHeading).toBeHidden({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Time Travel — quick presets
// ---------------------------------------------------------------------------

test.describe("Time Travel — quick presets", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("clicking '1 hour ago' preset closes popover and shows historical date in amber", async ({
    page,
  }) => {
    const trigger = page.getByRole("button", { name: "Time Travel" });
    await trigger.click();

    const popoverHeading = page.locator("h3", { hasText: "Time Travel" });
    await expect(popoverHeading).toBeVisible({ timeout: 5000 });

    // Click the "1 hour ago" preset
    await page.getByRole("button", { name: "1 hour ago" }).click();

    // Popover should close
    await expect(popoverHeading).toBeHidden({ timeout: 5000 });

    // Trigger button should no longer say "Time Travel" — it shows the historical date
    await expect(
      page.getByRole("button", { name: "Time Travel" })
    ).toBeHidden({ timeout: 5000 });

    // The trigger should have amber styling (border-amber-500/50 class)
    const triggerBtn = page.locator("button").filter({ has: page.locator("svg") }).filter({ hasText: /\d/ }).first();
    await expect(triggerBtn).toHaveClass(/amber/, { timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Time Travel — historical mode banner
// ---------------------------------------------------------------------------

test.describe("Time Travel — historical mode banner", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("historical mode banner appears when time travel is active", async ({
    page,
  }) => {
    // Activate time travel via preset
    const trigger = page.getByRole("button", { name: "Time Travel" });
    await trigger.click();

    const popoverHeading = page.locator("h3", { hasText: "Time Travel" });
    await expect(popoverHeading).toBeVisible({ timeout: 5000 });

    await page.getByRole("button", { name: "1 hour ago" }).click();
    await expect(popoverHeading).toBeHidden({ timeout: 5000 });

    // Historical mode banner should appear
    await expect(page.getByText("Time Travel Active")).toBeVisible({
      timeout: 5000,
    });
  });

  test("clicking 'Return to Present' in banner resets trigger to 'Time Travel'", async ({
    page,
  }) => {
    // Activate time travel
    const trigger = page.getByRole("button", { name: "Time Travel" });
    await trigger.click();

    await page.getByRole("button", { name: "1 hour ago" }).click();

    // Banner should appear
    await expect(page.getByText("Time Travel Active")).toBeVisible({
      timeout: 5000,
    });

    // Click "Return to Present" in the banner
    await page
      .getByRole("button", { name: "Return to Present" })
      .click();

    // Banner should disappear
    await expect(page.getByText("Time Travel Active")).toBeHidden({
      timeout: 5000,
    });

    // Trigger button should show "Time Travel" text again
    await expect(
      page.getByRole("button", { name: "Time Travel" })
    ).toBeVisible({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Time Travel — custom date & time
// ---------------------------------------------------------------------------

test.describe("Time Travel — custom date & time", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("setting custom date and time then clicking 'Travel to this time' activates time travel", async ({
    page,
  }) => {
    const trigger = page.getByRole("button", { name: "Time Travel" });
    await trigger.click();

    const popoverHeading = page.locator("h3", { hasText: "Time Travel" });
    await expect(popoverHeading).toBeVisible({ timeout: 5000 });

    // The time input already has a value — just update it
    const timeInput = page.locator("input[type='time']");
    await timeInput.fill("14:30");

    // Click "Travel to this time"
    await page.getByRole("button", { name: "Travel to this time" }).click();

    // Popover should close
    await expect(popoverHeading).toBeHidden({ timeout: 5000 });

    // Trigger should show amber styling (historical mode)
    await expect(
      page.getByRole("button", { name: "Time Travel" })
    ).toBeHidden({ timeout: 5000 });

    // Historical mode banner should appear
    await expect(page.getByText("Time Travel Active")).toBeVisible({
      timeout: 5000,
    });
  });
});
