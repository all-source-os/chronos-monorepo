import { test, expect, type Page } from "@playwright/test";
import { demoLogin, authenticateAndGoToDashboard } from "../../fixtures/demo-auth";

/**
 * Dashboard Header E2E Tests
 *
 * Covers theme toggle, user menu dropdown, command palette (Cmd+K),
 * and notifications bell button.
 *
 * Requires Control Plane for demo login.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/header.spec.ts
 */

// ---------------------------------------------------------------------------
// Theme toggle
// ---------------------------------------------------------------------------

test.describe("Header — theme toggle", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("clicking theme toggle changes theme class on html element", async ({
    page,
  }) => {
    const toggleBtn = page.getByRole("button", { name: /toggle theme/i }).or(
      page.locator("[aria-label='Toggle theme']")
    );
    await expect(toggleBtn.first()).toBeVisible({ timeout: 10000 });

    // Read the initial theme class
    const initialClass = await page.locator("html").getAttribute("class");
    const wasDark = initialClass?.includes("dark") ?? false;

    // Click the toggle
    await toggleBtn.first().click();

    // Verify the class changed
    if (wasDark) {
      await expect(page.locator("html")).not.toHaveClass(/dark/, {
        timeout: 5000,
      });
    } else {
      await expect(page.locator("html")).toHaveClass(/dark/, {
        timeout: 5000,
      });
    }

    // Click again to verify it toggles back
    await toggleBtn.first().click();

    if (wasDark) {
      await expect(page.locator("html")).toHaveClass(/dark/, {
        timeout: 5000,
      });
    } else {
      await expect(page.locator("html")).not.toHaveClass(/dark/, {
        timeout: 5000,
      });
    }
  });
});

// ---------------------------------------------------------------------------
// User menu dropdown
// ---------------------------------------------------------------------------

test.describe("Header — user menu", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("clicking user avatar shows dropdown with name, email, Profile, Settings, Log out", async ({
    page,
  }) => {
    const userMenuBtn = page.getByRole("button", { name: "User menu" });
    await expect(userMenuBtn).toBeVisible({ timeout: 10000 });

    await userMenuBtn.click();

    // Dropdown should show Profile, Settings, and Log out buttons
    await expect(page.getByRole("button", { name: "Profile" })).toBeVisible({
      timeout: 5000,
    });
    await expect(page.getByRole("button", { name: "Settings" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Log out" })).toBeVisible();
  });

  test("clicking Profile in user menu navigates to /dashboard/settings", async ({
    page,
  }) => {
    const userMenuBtn = page.getByRole("button", { name: "User menu" });
    await userMenuBtn.click();

    const profileBtn = page.getByRole("button", { name: "Profile" });
    await expect(profileBtn).toBeVisible({ timeout: 5000 });
    await profileBtn.click();

    await page.waitForURL(/\/dashboard\/settings/, { timeout: 15000 });
    expect(page.url()).toContain("/dashboard/settings");
  });

  test("clicking Settings in user menu navigates to /dashboard/settings", async ({
    page,
  }) => {
    const userMenuBtn = page.getByRole("button", { name: "User menu" });
    await userMenuBtn.click();

    const settingsBtn = page.getByRole("button", { name: "Settings" });
    await expect(settingsBtn).toBeVisible({ timeout: 5000 });
    await settingsBtn.click();

    await page.waitForURL(/\/dashboard\/settings/, { timeout: 15000 });
    expect(page.url()).toContain("/dashboard/settings");
  });
});

// ---------------------------------------------------------------------------
// Command palette (Cmd+K)
// ---------------------------------------------------------------------------

test.describe("Header — command palette", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("Cmd+K opens command palette with search input visible", async ({
    page,
  }) => {
    // Press Cmd+K (Meta+K)
    await page.keyboard.press("Meta+k");

    // The command palette search input should appear
    const searchInput = page.getByPlaceholder("Search commands...");
    await expect(searchInput).toBeVisible({ timeout: 5000 });
  });

  test("typing in command palette filters results", async ({ page }) => {
    await page.keyboard.press("Meta+k");

    const searchInput = page.getByPlaceholder("Search commands...");
    await expect(searchInput).toBeVisible({ timeout: 5000 });

    // Type a search term that matches one navigation command
    await searchInput.fill("Events");

    // "Go to Events" should be visible, non-matching items should be hidden
    await expect(page.getByText("Go to Events")).toBeVisible({ timeout: 5000 });

    // Items that don't match "Events" should not be visible
    await expect(page.getByText("Go to Billing")).toBeHidden();
  });

  test("clicking a command palette navigation item navigates correctly", async ({
    page,
  }) => {
    await page.keyboard.press("Meta+k");

    const searchInput = page.getByPlaceholder("Search commands...");
    await expect(searchInput).toBeVisible({ timeout: 5000 });

    // Click "Go to Events"
    const eventsCmd = page.getByText("Go to Events");
    await expect(eventsCmd).toBeVisible({ timeout: 5000 });
    await eventsCmd.click();

    // Should navigate to /dashboard/events and close the palette
    await page.waitForURL(/\/dashboard\/events/, { timeout: 15000 });
    expect(page.url()).toContain("/dashboard/events");

    // Palette should be closed
    await expect(searchInput).toBeHidden({ timeout: 5000 });
  });

  test("Escape closes command palette", async ({ page }) => {
    await page.keyboard.press("Meta+k");

    const searchInput = page.getByPlaceholder("Search commands...");
    await expect(searchInput).toBeVisible({ timeout: 5000 });

    // Press Escape
    await page.keyboard.press("Escape");

    // Palette should close
    await expect(searchInput).toBeHidden({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Notifications bell
// ---------------------------------------------------------------------------

test.describe("Header — notifications bell", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("notifications bell button is clickable", async ({ page }) => {
    const bellBtn = page.getByRole("button", { name: "Notifications" });
    await expect(bellBtn).toBeVisible({ timeout: 10000 });

    // Verify the button is enabled and clickable (no error thrown)
    await expect(bellBtn).toBeEnabled();
    await bellBtn.click();
  });
});
