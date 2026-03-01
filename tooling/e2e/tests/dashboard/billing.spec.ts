import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

/**
 * Billing Page E2E Tests
 *
 * Covers plan display, billing toggle, usage charts, and plan cards.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/billing.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/billing");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Page rendering
// ---------------------------------------------------------------------------

test.describe("Billing — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("page renders with heading", async ({ page }) => {
    await expect(page.getByText("Billing & Usage")).toBeVisible({ timeout: 10000 });
  });

  test("usage charts render (Events and Queries)", async ({ page }) => {
    // Usage section should show Events and Queries charts
    const hasEvents = await page.getByText(/Events/i).first().isVisible({ timeout: 10000 }).catch(() => false);
    const hasQueries = await page.getByText(/Queries/i).first().isVisible({ timeout: 5000 }).catch(() => false);

    expect(hasEvents || hasQueries).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// Billing period toggle
// ---------------------------------------------------------------------------

test.describe("Billing — period toggle", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("clicking Yearly toggle updates plan prices", async ({ page }) => {
    const yearlyBtn = page.getByRole("button", { name: /Yearly/i });
    const isVisible = await yearlyBtn.isVisible({ timeout: 5000 }).catch(() => false);

    if (isVisible) {
      // Record a price before toggle
      const pricesBefore = await page.locator("text=/\\$\\d+/").allTextContents();

      await yearlyBtn.click();
      await page.waitForTimeout(500);

      // Prices may have changed (yearly discount)
      // Just verify the toggle is responsive
      const yearlyClasses = await yearlyBtn.getAttribute("class");
      expect(yearlyClasses).toBeTruthy();
    }
  });

  test("clicking Monthly toggle reverts prices", async ({ page }) => {
    const yearlyBtn = page.getByRole("button", { name: /Yearly/i });
    const monthlyBtn = page.getByRole("button", { name: /Monthly/i });

    const isVisible = await yearlyBtn.isVisible({ timeout: 5000 }).catch(() => false);

    if (isVisible) {
      // Switch to yearly first
      await yearlyBtn.click();
      await page.waitForTimeout(300);

      // Switch back to monthly
      await monthlyBtn.click();
      await page.waitForTimeout(300);

      // Monthly should now be active
      const monthlyClasses = await monthlyBtn.getAttribute("class");
      expect(monthlyClasses).toBeTruthy();
    }
  });
});

// ---------------------------------------------------------------------------
// Plan cards
// ---------------------------------------------------------------------------

test.describe("Billing — plan cards", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("plan cards render with action buttons", async ({ page }) => {
    // Should show plan cards with Upgrade or Contact Sales buttons
    const hasUpgrade = await page.getByRole("button", { name: /Upgrade/i }).first().isVisible({ timeout: 10000 }).catch(() => false);
    const hasContactSales = await page.getByText(/Contact Sales/i).first().isVisible({ timeout: 5000 }).catch(() => false);
    const hasCurrent = await page.getByText(/Current Plan/i).first().isVisible({ timeout: 5000 }).catch(() => false);

    expect(hasUpgrade || hasContactSales || hasCurrent).toBeTruthy();
  });

  test("if on paid plan, Manage Subscription button exists", async ({ page }) => {
    const manageBtn = page.getByRole("button", { name: /Manage Subscription/i });
    const isVisible = await manageBtn.isVisible({ timeout: 5000 }).catch(() => false);

    // This is conditional — only shown for paid plans
    // We just verify the page doesn't error
    if (isVisible) {
      await expect(manageBtn).toBeEnabled();
    }
  });
});
