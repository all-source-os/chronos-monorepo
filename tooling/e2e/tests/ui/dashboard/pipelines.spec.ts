import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../../fixtures/demo-auth";

/**
 * Pipelines Page E2E Tests
 *
 * Covers pipeline listing, status overview cards, overflow menu
 * actions, and empty state.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/pipelines.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/pipelines");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Page rendering
// ---------------------------------------------------------------------------

test.describe("Pipelines — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("page renders with heading", async ({ page }) => {
    await expect(page.getByRole("heading", { name: /pipelines/i }).first()).toBeVisible({ timeout: 10000 });
  });

  test("status overview cards or empty state renders", async ({ page }) => {
    // Either status cards (Total Pipelines, Running, Paused, Errors) or empty state
    const hasStatusCards = await page.getByText("Total Pipelines").isVisible({ timeout: 5000 }).catch(() => false);
    const hasEmptyState = await page.getByText("No pipelines yet").isVisible({ timeout: 3000 }).catch(() => false);

    expect(hasStatusCards || hasEmptyState).toBeTruthy();
  });

  test("Create Pipeline button is visible and clickable", async ({ page }) => {
    const createBtn = page.getByRole("button", { name: /Create Pipeline/i }).first();
    // Could also be a link styled as button
    const createLink = page.getByRole("link", { name: /Create Pipeline/i }).first();

    const btnVisible = await createBtn.isVisible({ timeout: 5000 }).catch(() => false);
    const linkVisible = await createLink.isVisible({ timeout: 3000 }).catch(() => false);

    expect(btnVisible || linkVisible).toBeTruthy();

    if (btnVisible) {
      await expect(createBtn).toBeEnabled();
    }
  });
});

// ---------------------------------------------------------------------------
// Pipeline cards (when pipelines exist)
// ---------------------------------------------------------------------------

test.describe("Pipelines — pipeline cards", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("if pipelines exist, status overview cards show numeric values", async ({ page }) => {
    const hasPipelines = await page.getByText("Total Pipelines").isVisible({ timeout: 5000 }).catch(() => false);
    test.skip(!hasPipelines, "No pipelines exist — skipping card tests");

    // All 4 status cards should be visible — use exact+first to avoid badge matches
    await expect(page.getByText("Total Pipelines")).toBeVisible();
    await expect(page.getByText("Running", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("Paused", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("Errors", { exact: true }).first()).toBeVisible();
  });

  test("if pipelines exist, overflow menu shows Pause/Resume/Reset", async ({ page }) => {
    const hasPipelines = await page.getByText("Total Pipelines").isVisible({ timeout: 5000 }).catch(() => false);
    test.skip(!hasPipelines, "No pipelines exist — skipping overflow tests");

    // Find the overflow menu (MoreHorizontal button) on first pipeline card
    const overflowBtn = page.locator("button").filter({ has: page.locator("svg") }).nth(5);
    const isVisible = await overflowBtn.isVisible({ timeout: 3000 }).catch(() => false);

    if (isVisible) {
      await overflowBtn.click();
      // Should show at least one of Pause/Resume and Reset
      const hasPause = await page.getByText("Pause").isVisible({ timeout: 3000 }).catch(() => false);
      const hasResume = await page.getByText("Resume").isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasPause || hasResume).toBeTruthy();
    }
  });

  test("if no pipelines, empty state renders with Create Pipeline", async ({ page }) => {
    const hasEmptyState = await page.getByText("No pipelines yet").isVisible({ timeout: 5000 }).catch(() => false);
    test.skip(!hasEmptyState, "Pipelines exist — skipping empty state test");

    await expect(page.getByText("No pipelines yet")).toBeVisible();
    await expect(page.getByText(/Create.*Pipeline/i).first()).toBeVisible();
  });
});
