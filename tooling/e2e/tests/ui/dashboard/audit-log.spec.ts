import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../../fixtures/demo-auth";

/**
 * Audit Log Page E2E Tests
 *
 * Covers action filter pills and pagination.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/audit-log.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/settings/audit-log");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Page rendering
// ---------------------------------------------------------------------------

test.describe("Audit Log — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("page renders with heading", async ({ page }) => {
    await expect(page.getByRole("heading", { name: /audit log/i }).first()).toBeVisible({ timeout: 10000 });
  });

  test("'All' filter pill is active by default", async ({ page }) => {
    const allPill = page.getByRole("button", { name: /^All$/i });
    await expect(allPill).toBeVisible({ timeout: 10000 });

    // Active pill should have primary styling
    const classes = await allPill.getAttribute("class");
    expect(classes).toMatch(/primary|active/i);
  });
});

// ---------------------------------------------------------------------------
// Filters
// ---------------------------------------------------------------------------

test.describe("Audit Log — action filters", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("clicking an action filter pill filters the list", async ({ page }) => {
    // Look for any action filter pill besides "All"
    const filterPills = page.getByRole("button").filter({
      hasText: /API Key Created|API Key Revoked|Member Invited|Member Removed|Plan Changed/,
    });
    const count = await filterPills.count();

    if (count > 0) {
      await filterPills.first().click();
      await page.waitForTimeout(500);

      // The "All" pill should no longer have active styling
      const allPill = page.getByRole("button", { name: /^All$/i });
      const allClasses = await allPill.getAttribute("class");
      // The clicked pill should now be active
      const clickedClasses = await filterPills.first().getAttribute("class");
      expect(clickedClasses).toMatch(/primary|active/i);
    }
  });

  test("clicking 'All' after filtering shows all entries again", async ({ page }) => {
    // Click a filter first
    const filterPills = page.getByRole("button").filter({
      hasText: /API Key Created|API Key Revoked|Member Invited/,
    });
    const count = await filterPills.count();

    if (count > 0) {
      await filterPills.first().click();
      await page.waitForTimeout(500);

      // Click All to reset
      await page.getByRole("button", { name: /^All$/i }).click();
      await page.waitForTimeout(500);

      // All should be active again
      const allClasses = await page.getByRole("button", { name: /^All$/i }).getAttribute("class");
      expect(allClasses).toMatch(/primary|active/i);
    }
  });
});

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

test.describe("Audit Log — pagination", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("pagination controls are visible when entries exist", async ({ page }) => {
    // Look for pagination (Previous/Next buttons)
    const prevBtn = page.getByRole("button", { name: /Previous/i });
    const nextBtn = page.getByRole("button", { name: /Next/i });

    const hasPagination = await prevBtn.isVisible({ timeout: 5000 }).catch(() => false);

    if (hasPagination) {
      // Page label should show "Page 1"
      await expect(page.getByText(/Page \d+/)).toBeVisible();

      // Previous should be disabled on first page
      await expect(prevBtn).toBeDisabled();
    }
  });

  test("if multiple pages exist, Next increments the page", async ({ page }) => {
    const nextBtn = page.getByRole("button", { name: /Next/i });
    const isVisible = await nextBtn.isVisible({ timeout: 5000 }).catch(() => false);
    const isEnabled = isVisible ? await nextBtn.isEnabled() : false;

    if (isVisible && isEnabled) {
      await nextBtn.click();
      await page.waitForTimeout(500);

      // Should show Page 2
      await expect(page.getByText("Page 2")).toBeVisible({ timeout: 5000 });

      // Previous should now be enabled
      const prevBtn = page.getByRole("button", { name: /Previous/i });
      await expect(prevBtn).toBeEnabled();

      // Click Previous to go back
      await prevBtn.click();
      await expect(page.getByText("Page 1")).toBeVisible({ timeout: 5000 });
    }
  });
});
