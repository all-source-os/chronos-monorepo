import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

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
    await expect(page.getByText("Settings")).toBeVisible({ timeout: 10000 });

    // All 4 tabs should be visible
    await expect(page.getByText("Profile")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("Workspace")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("Security")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("Notifications")).toBeVisible({ timeout: 5000 });
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

  test("Full Name input is editable, email is disabled", async ({ page }) => {
    const nameInput = page.locator("#name");
    await expect(nameInput).toBeVisible({ timeout: 10000 });
    await expect(nameInput).toBeEnabled();

    const emailInput = page.locator("#email");
    await expect(emailInput).toBeVisible();
    await expect(emailInput).toBeDisabled();
  });

  test("changing name and saving shows success indicator", async ({ page }) => {
    const nameInput = page.locator("#name");
    await expect(nameInput).toBeVisible({ timeout: 10000 });

    // Store original name
    const originalName = await nameInput.inputValue();

    // Change the name
    await nameInput.clear();
    await nameInput.fill("E2E Test Name");

    // Click Save Changes
    const saveBtn = page.getByRole("button", { name: /Save Changes/i });
    await saveBtn.click();

    // Should show success indicator (Saved text or checkmark)
    await expect(page.getByText(/Saved|Success/i)).toBeVisible({ timeout: 5000 });

    // Revert the name back
    await nameInput.clear();
    await nameInput.fill(originalName || "Demo User");
    await saveBtn.click();
    await expect(page.getByText(/Saved|Success/i)).toBeVisible({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Workspace tab
// ---------------------------------------------------------------------------

test.describe("Settings — Workspace tab", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("Workspace tab shows name, slug, and Tenant ID", async ({ page }) => {
    // Click Workspace tab
    await page.getByText("Workspace").click();
    await page.waitForTimeout(500);

    // Workspace Name input
    await expect(page.locator("#workspace-name")).toBeVisible({ timeout: 5000 });

    // URL slug input
    await expect(page.locator("#workspace-slug")).toBeVisible({ timeout: 5000 });

    // Tenant ID should be displayed (read-only code block)
    await expect(page.getByText("Tenant ID")).toBeVisible();
    const tenantCode = page.locator("code");
    await expect(tenantCode.first()).toBeVisible();
  });

  test("typing in URL slug sanitizes to lowercase", async ({ page }) => {
    await page.getByText("Workspace").click();
    await page.waitForTimeout(500);

    const slugInput = page.locator("#workspace-slug");
    await slugInput.clear();
    await slugInput.fill("My Test Workspace");
    await page.waitForTimeout(300);

    const value = await slugInput.inputValue();
    // Should be sanitized (lowercase, no spaces)
    expect(value).not.toMatch(/[A-Z ]/);
  });
});

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
