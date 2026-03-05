import { test, expect, type Page } from "@playwright/test";
import { demoLogin, authenticateAndGoToDashboard } from "../../../fixtures/demo-auth";

/**
 * Logout Flow E2E Tests
 *
 * Covers user menu logout, session destruction, and redirect behavior.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/logout.spec.ts
 */

// ---------------------------------------------------------------------------
// Logout flow
// ---------------------------------------------------------------------------

test.describe("Logout flow", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test("clicking user avatar opens menu with Log out option", async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);

    const userMenuBtn = page.getByRole("button", { name: "User menu" });
    await expect(userMenuBtn).toBeVisible({ timeout: 10000 });
    await userMenuBtn.click();

    await expect(page.getByRole("button", { name: "Log out" })).toBeVisible({ timeout: 5000 });
  });

  test("clicking Log out redirects away from dashboard", async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);

    // Open user menu
    const userMenuBtn = page.getByRole("button", { name: /user menu/i }).or(
      page.locator("[aria-label='User menu']")
    ).first();
    await userMenuBtn.click();

    // Click Log out
    await page.getByRole("button", { name: /log out/i }).or(
      page.getByText("Log out")
    ).first().click();

    // Should redirect away from dashboard (to /login or homepage)
    await page.waitForURL((url) => !url.pathname.startsWith("/dashboard"), { timeout: 30000 });
    expect(page.url()).not.toContain("/dashboard");
  });

  test("after logout, navigating to /dashboard redirects away", async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);

    // Log out
    const userMenuBtn = page.getByRole("button", { name: /user menu/i }).or(
      page.locator("[aria-label='User menu']")
    ).first();
    await userMenuBtn.click();
    await page.getByRole("button", { name: /log out/i }).or(
      page.getByText("Log out")
    ).first().click();
    await page.waitForURL((url) => !url.pathname.startsWith("/dashboard"), { timeout: 30000 });

    // Try to navigate to /dashboard
    await page.goto("/dashboard");

    // Should be redirected away from /dashboard (to /login or homepage)
    await page.waitForURL((url) => !url.pathname.startsWith("/dashboard"), { timeout: 30000 });
    expect(page.url()).not.toContain("/dashboard");
  });
});
