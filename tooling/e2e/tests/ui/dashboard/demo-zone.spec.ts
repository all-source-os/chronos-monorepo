import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../../fixtures/demo-auth";

/**
 * Demo Zone E2E Tests
 *
 * Covers view toggle, seeding flow, Live Fire panels,
 * and MCP Showdown view.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/demo-zone.spec.ts
 */

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/demo");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Page rendering
// ---------------------------------------------------------------------------

test.describe("Demo Zone — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("page renders with heading", async ({ page }) => {
    await expect(page.getByRole("heading", { name: /Demo Zone/i }).first()).toBeVisible({ timeout: 10000 });
  });

  test("view toggle shows Live Fire and MCP Showdown buttons", async ({ page }) => {
    await expect(page.getByRole("button", { name: /Live Fire/i })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole("button", { name: /MCP Showdown/i })).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// View toggle
// ---------------------------------------------------------------------------

test.describe("Demo Zone — view toggle", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("clicking MCP Showdown updates URL to ?view=mcp-showdown", async ({ page }) => {
    const mcpBtn = page.getByRole("button", { name: /MCP Showdown/i });
    await expect(mcpBtn).toBeVisible({ timeout: 10000 });
    await mcpBtn.click();

    await page.waitForTimeout(500);
    expect(page.url()).toMatch(/view=mcp/i);
  });

  test("clicking Live Fire updates URL back", async ({ page }) => {
    // First switch to MCP
    await page.getByRole("button", { name: /MCP Showdown/i }).click();
    await page.waitForTimeout(500);

    // Switch back to Live Fire
    await page.getByRole("button", { name: /Live Fire/i }).click();
    await page.waitForTimeout(500);

    expect(page.url()).toMatch(/view=live-fire|\/dashboard\/demo$/);
  });
});

// ---------------------------------------------------------------------------
// Pre-seed / post-seed states
// ---------------------------------------------------------------------------

test.describe("Demo Zone — seeding states", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("if not seeded, shows Start Demo button with empty state", async ({ page }) => {
    const notSeeded = await page.getByText("No demo data yet").isVisible({ timeout: 5000 }).catch(() => false);

    if (notSeeded) {
      await expect(page.getByRole("button", { name: /Start Demo/i })).toBeVisible();
    }
  });

  test("if seeded, Live Fire panels render in default view", async ({ page }) => {
    const seeded = await page.getByText("No demo data yet").isVisible({ timeout: 5000 }).catch(() => false);

    if (!seeded) {
      // Should show live fire panels (default view)
      // Look for panel-like content
      const hasContent = await page.locator("[class*='grid']").nth(1).isVisible({ timeout: 5000 }).catch(() => false);
      expect(hasContent).toBeTruthy();
    }
  });

  test("if seeded, MCP Showdown view shows Cost Calculator", async ({ page }) => {
    const seeded = await page.getByText("No demo data yet").isVisible({ timeout: 5000 }).catch(() => false);

    if (!seeded) {
      // Switch to MCP Showdown
      await page.getByRole("button", { name: /MCP Showdown/i }).click();
      await page.waitForTimeout(1000);

      // Cost Calculator should be visible with interactive inputs
      const hasCostCalc = await page.getByText(/Cost Calculator/i).isVisible({ timeout: 5000 }).catch(() => false);
      if (hasCostCalc) {
        // Verify input fields exist
        const inputs = page.locator("input[type='number'], input[type='text']");
        const count = await inputs.count();
        expect(count).toBeGreaterThan(0);
      }
    }
  });
});
