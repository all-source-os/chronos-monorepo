import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../../fixtures/demo-auth";

/**
 * Settings Page E2E Tests
 *
 * Covers all 4 tabs: Profile, Workspace, Security, Notifications.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/settings.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/settings");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Page rendering and tab navigation
// ---------------------------------------------------------------------------

test.describe("Settings — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("page renders with heading and tab navigation", async ({ page }) => {
    await expect(page.getByRole("heading", { name: /settings/i }).first()).toBeVisible({ timeout: 10000 });

    // All 3 tabs should be visible — use exact match and first() to avoid sidebar matches
    await expect(page.getByText("Profile", { exact: true }).first()).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("Security", { exact: true }).first()).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("Notifications", { exact: true }).first()).toBeVisible({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Profile tab
// ---------------------------------------------------------------------------

test.describe("Settings — Profile tab", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("Profile tab shows read-only name and email fields", async ({ page }) => {
    await expect(page.getByText("Profile Information")).toBeVisible({ timeout: 10000 });

    // Full Name and Email Address labels should be visible
    await expect(page.getByText("Full Name")).toBeVisible();
    await expect(page.getByText("Email Address")).toBeVisible();

    // Both inputs are disabled (managed by OAuth provider)
    const inputs = page.locator("input[disabled]");
    const count = await inputs.count();
    expect(count).toBeGreaterThanOrEqual(2);
  });

  test("Tenant ID is displayed", async ({ page }) => {
    await expect(page.getByText("Tenant ID")).toBeVisible({ timeout: 10000 });
    await expect(page.locator("code").first()).toBeVisible();
    await expect(page.getByText("Use this ID for API authentication")).toBeVisible();
  });
});

// Workspace tab was removed — tenant info is now shown on Profile tab

// ---------------------------------------------------------------------------
// Security tab
// ---------------------------------------------------------------------------

test.describe("Settings — Security tab", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("Security tab shows connected accounts and Delete Account", async ({ page }) => {
    await page.getByText("Security").click();
    await page.waitForTimeout(500);

    // Connected accounts section
    const hasConnected = await page.getByText(/Connected Accounts|Google|GitHub/i).first().isVisible({ timeout: 5000 }).catch(() => false);
    expect(hasConnected).toBeTruthy();

    // Delete Account button
    await expect(page.getByRole("button", { name: /Delete Account/i })).toBeVisible({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Notifications tab
// ---------------------------------------------------------------------------

test.describe("Settings — Notifications tab", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("Notifications tab shows 5 toggle switches", async ({ page }) => {
    await page.getByText("Notifications").click();
    await page.waitForTimeout(500);

    // 5 notification toggles
    await expect(page.getByText("Usage Alerts")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("Pipeline Errors")).toBeVisible();
    await expect(page.getByText("Security Alerts")).toBeVisible();
    await expect(page.getByText("Product Updates")).toBeVisible();
    await expect(page.getByText("Tips & Tutorials")).toBeVisible();
  });

  test("toggling notification switches changes state", async ({ page }) => {
    await page.getByText("Notifications").click();
    await page.waitForTimeout(500);

    // Find the toggle switches (checkbox inputs that are sr-only, with styled div siblings)
    const toggles = page.locator("input[type='checkbox']");
    const count = await toggles.count();

    // Should have at least 5 toggles
    expect(count).toBeGreaterThanOrEqual(5);

    // Toggle the first one
    const firstToggle = toggles.first();
    const wasCheked = await firstToggle.isChecked();
    await firstToggle.click({ force: true }); // force because sr-only
    const isNowChecked = await firstToggle.isChecked();
    expect(isNowChecked).not.toBe(wasCheked);

    // Toggle back
    await firstToggle.click({ force: true });
  });
});
