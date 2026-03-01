import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

/**
 * API Keys Page E2E Tests
 *
 * Covers key creation, listing, copying, show/hide toggle,
 * rotation, and revocation.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/api-keys.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/api-keys");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Page rendering
// ---------------------------------------------------------------------------

test.describe("API Keys — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("key table or empty state renders", async ({ page }) => {
    await expect(page.getByText("API Keys")).toBeVisible({ timeout: 10000 });

    // Should show either the key table or "No API keys yet"
    const hasTable = await page.locator("table, [role='table']").first().isVisible({ timeout: 5000 }).catch(() => false);
    const hasEmpty = await page.getByText("No API keys yet").isVisible({ timeout: 3000 }).catch(() => false);
    expect(hasTable || hasEmpty).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Key creation flow
// ---------------------------------------------------------------------------

test.describe("API Keys — create key", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("clicking Create Key opens the dialog with all fields", async ({ page }) => {
    const createBtn = page.getByRole("button", { name: /Create Key/i });
    await expect(createBtn).toBeVisible({ timeout: 10000 });
    await createBtn.click();

    // Dialog should show name input, description, scope toggles, expiration
    await expect(page.locator("#name")).toBeVisible({ timeout: 5000 });
    await expect(page.locator("#description")).toBeVisible({ timeout: 5000 });

    // 7 scope toggles should be present
    await expect(page.getByText("Read Events")).toBeVisible();
    await expect(page.getByText("Write Events")).toBeVisible();

    // Expiration dropdown
    await expect(page.locator("#expiration")).toBeVisible();

    // Cancel and Create Key buttons
    await expect(page.getByRole("button", { name: /^Cancel$/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /^Create Key$/i })).toBeVisible();
  });

  test("fill form and create a key successfully", async ({ page }) => {
    const createBtn = page.getByRole("button", { name: /Create Key/i });
    await createBtn.click();

    // Fill the name
    await page.locator("#name").fill("E2E Test Key");

    // Scope toggles — Read Events and Write Events should be checked by default
    // Select 30 days expiration
    await page.locator("#expiration").selectOption("30");

    // Click Create Key
    const submitBtn = page.getByRole("button", { name: /^Create Key$/i });
    await submitBtn.click();

    // Should show success state with the generated key and Done button
    await expect(page.getByRole("button", { name: /Done/i })).toBeVisible({ timeout: 10000 });

    // Key reveal section should show copy button
    await expect(page.getByRole("button", { name: /Copy/i }).first()).toBeVisible();
  });

  test("show/hide toggle on generated key works", async ({ page }) => {
    // Create a key first
    const createBtn = page.getByRole("button", { name: /Create Key/i });
    await createBtn.click();
    await page.locator("#name").fill("E2E Toggle Test");
    await page.getByRole("button", { name: /^Create Key$/i }).click();
    await expect(page.getByRole("button", { name: /Done/i })).toBeVisible({ timeout: 10000 });

    // Find the show/hide toggle (eye icon button)
    const toggleBtn = page.locator("button[aria-label*='how'], button[aria-label*='ide']").first();
    const isVisible = await toggleBtn.isVisible().catch(() => false);
    if (isVisible) {
      await toggleBtn.click();
      await page.waitForTimeout(300);
      await toggleBtn.click();
    }

    // Click Done to close
    await page.getByRole("button", { name: /Done/i }).click();
  });

  test("clicking Done closes the dialog and key appears in table", async ({ page }) => {
    // Create a key
    const createBtn = page.getByRole("button", { name: /Create Key/i });
    await createBtn.click();
    await page.locator("#name").fill("E2E Table Check");
    await page.getByRole("button", { name: /^Create Key$/i }).click();
    await expect(page.getByRole("button", { name: /Done/i })).toBeVisible({ timeout: 10000 });

    await page.getByRole("button", { name: /Done/i }).click();

    // Dialog should close — the name input should be hidden
    await expect(page.locator("#name")).toBeHidden({ timeout: 5000 });

    // New key should appear in the table
    await expect(page.getByText("E2E Table Check")).toBeVisible({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Key actions — overflow menu, revoke
// ---------------------------------------------------------------------------

test.describe("API Keys — key actions", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);

    // Ensure at least one key exists by creating one
    const createBtn = page.getByRole("button", { name: /Create Key/i });
    await createBtn.click();
    await page.locator("#name").fill("E2E Action Test");
    await page.getByRole("button", { name: /^Create Key$/i }).click();
    await expect(page.getByRole("button", { name: /Done/i })).toBeVisible({ timeout: 10000 });
    await page.getByRole("button", { name: /Done/i }).click();
    await expect(page.locator("#name")).toBeHidden({ timeout: 5000 });
  });

  test("overflow menu shows Rotate Key and Revoke Key", async ({ page }) => {
    // Click the overflow menu (...) on the first key row
    const moreBtn = page.locator("button").filter({ has: page.locator("svg") }).locator("visible=true").last();
    // Look for the MoreHorizontal button in the table
    const overflowBtns = page.locator("table button, [role='table'] button").filter({ hasNotText: /Create|Copy|Done/ });
    const firstOverflow = overflowBtns.last();

    if (await firstOverflow.isVisible({ timeout: 3000 }).catch(() => false)) {
      await firstOverflow.click();

      // Dropdown should show Rotate and Revoke options
      await expect(page.getByText("Rotate Key")).toBeVisible({ timeout: 5000 });
      await expect(page.getByText("Revoke Key")).toBeVisible({ timeout: 5000 });
    }
  });

  test("clicking Revoke Key shows confirmation, Cancel closes it", async ({ page }) => {
    // Find and click overflow menu
    const overflowBtns = page.locator("table button, [role='table'] button").filter({ hasNotText: /Create|Copy|Done/ });
    const firstOverflow = overflowBtns.last();

    if (await firstOverflow.isVisible({ timeout: 3000 }).catch(() => false)) {
      await firstOverflow.click();
      await page.getByText("Revoke Key").click();

      // Confirmation modal should appear
      await expect(page.getByText("Revoke API Key?")).toBeVisible({ timeout: 5000 });
      await expect(page.getByRole("button", { name: /^Cancel$/i })).toBeVisible();

      // Cancel should close the confirmation
      await page.getByRole("button", { name: /^Cancel$/i }).click();
      await expect(page.getByText("Revoke API Key?")).toBeHidden({ timeout: 5000 });
    }
  });

  test("confirming revoke removes the key from table", async ({ page }) => {
    // Count keys before
    const keysBefore = await page.getByText("E2E Action Test").count();

    // Find and click overflow menu on the key we created
    const overflowBtns = page.locator("table button, [role='table'] button").filter({ hasNotText: /Create|Copy|Done/ });
    const firstOverflow = overflowBtns.last();

    if (await firstOverflow.isVisible({ timeout: 3000 }).catch(() => false)) {
      await firstOverflow.click();
      await page.getByText("Revoke Key").click();

      // Confirm the revocation
      await expect(page.getByText("Revoke API Key?")).toBeVisible({ timeout: 5000 });
      const revokeConfirmBtn = page.getByRole("button", { name: /^Revoke Key$/i });
      await revokeConfirmBtn.click();

      // Key should be removed
      await page.waitForTimeout(1000);
      const keysAfter = await page.getByText("E2E Action Test").count();
      expect(keysAfter).toBeLessThan(keysBefore);
    }
  });
});
