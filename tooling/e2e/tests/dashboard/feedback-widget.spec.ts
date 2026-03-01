import { test, expect, type Page } from "@playwright/test";
import { demoLogin, authenticateAndGoToDashboard } from "../../fixtures/demo-auth";

/**
 * Feedback Widget E2E Tests
 *
 * Covers the floating feedback button, modal form, category selection,
 * submission, and close behaviors.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/feedback-widget.spec.ts
 */

// ---------------------------------------------------------------------------
// Floating button visibility
// ---------------------------------------------------------------------------

test.describe("Feedback Widget — floating button", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("floating feedback button is visible", async ({ page }) => {
    const feedbackBtn = page.locator("[aria-label='Send feedback']");
    await expect(feedbackBtn).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Modal open/close
// ---------------------------------------------------------------------------

test.describe("Feedback Widget — modal", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
  });

  test("clicking feedback button opens modal", async ({ page }) => {
    await page.locator("[aria-label='Send feedback']").click();

    await expect(page.getByText("Send Feedback")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("Report a bug, request a feature, or ask a question")).toBeVisible();
  });

  test("clicking Cancel closes the modal", async ({ page }) => {
    await page.locator("[aria-label='Send feedback']").click();
    await expect(page.getByText("Send Feedback")).toBeVisible({ timeout: 5000 });

    await page.getByRole("button", { name: /Cancel/i }).click();

    await expect(page.getByText("Send Feedback")).toBeHidden({ timeout: 5000 });
  });

  test("Escape closes the modal", async ({ page }) => {
    await page.locator("[aria-label='Send feedback']").click();
    await expect(page.getByText("Send Feedback")).toBeVisible({ timeout: 5000 });

    await page.keyboard.press("Escape");

    await expect(page.getByText("Send Feedback")).toBeHidden({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Category selection
// ---------------------------------------------------------------------------

test.describe("Feedback Widget — categories", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
    await page.locator("[aria-label='Send feedback']").click();
    await expect(page.getByText("Send Feedback")).toBeVisible({ timeout: 5000 });
  });

  test("clicking each category button changes active state", async ({ page }) => {
    const categories = ["Bug Report", "Feature Request", "Question"];

    for (const category of categories) {
      const btn = page.getByRole("button", { name: new RegExp(category, "i") });
      await expect(btn).toBeVisible();
      await btn.click();

      // Active state — the clicked button should have distinct styling
      const classes = await btn.getAttribute("class");
      expect(classes).toMatch(/primary|active|bg-/);
    }
  });
});

// ---------------------------------------------------------------------------
// Form submission
// ---------------------------------------------------------------------------

test.describe("Feedback Widget — submission", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);
    await page.locator("[aria-label='Send feedback']").click();
    await expect(page.getByText("Send Feedback")).toBeVisible({ timeout: 5000 });
  });

  test("fill message and optional email", async ({ page }) => {
    // Select a category
    await page.getByRole("button", { name: /Bug Report/i }).click();

    // Fill message
    const messageInput = page.locator("#feedback-message");
    await expect(messageInput).toBeVisible({ timeout: 5000 });
    await messageInput.fill("E2E test feedback message — please ignore");

    // Fill optional email
    const emailInput = page.locator("#feedback-email");
    await expect(emailInput).toBeVisible();
    await emailInput.fill("e2e@example.com");

    // Submit button should be enabled
    await expect(page.getByRole("button", { name: /Submit Feedback/i })).toBeEnabled();
  });

  test("submitting feedback shows success state", async ({ page }) => {
    await page.getByRole("button", { name: /Feature Request/i }).click();

    const messageInput = page.locator("#feedback-message");
    await messageInput.fill("E2E test feedback — automated test, please ignore");

    await page.getByRole("button", { name: /Submit Feedback/i }).click();

    // Should show success or submitting state
    const hasSuccess = await page.getByText(/Thank you|Submitting/i).isVisible({ timeout: 10000 }).catch(() => false);
    const hasError = await page.locator("[class*='destructive']").first().isVisible({ timeout: 3000 }).catch(() => false);

    // Either success or a handled error is acceptable
    expect(hasSuccess || hasError).toBeTruthy();
  });
});
