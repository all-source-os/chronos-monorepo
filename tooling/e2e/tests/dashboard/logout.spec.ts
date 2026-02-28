import { test, expect, type Page } from "@playwright/test";

/**
 * Logout Flow E2E Tests
 *
 * Covers user menu logout, session destruction, and redirect behavior.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/logout.spec.ts
 */

const CP_URL =
  process.env.CONTROL_PLANE_URL || "http://localhost:3901";

async function demoLogin(request: any): Promise<string | null> {
  const demoResp = await request.post(`${CP_URL}/api/v1/demo/start`, {
    headers: { "Content-Type": "application/json" },
  });
  if (!demoResp.ok()) return null;
  const demoData = await demoResp.json();

  const loginResp = await request.post(`${CP_URL}/api/v1/auth/login`, {
    headers: { "Content-Type": "application/json" },
    data: { email: demoData.email, password: demoData.password },
  });
  if (!loginResp.ok()) return null;
  const loginData = await loginResp.json();
  return loginData.token as string;
}

async function authenticateAndGoToDashboard(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  if (!page.url().includes("/dashboard")) {
    await page.goto("/dashboard");
  }
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

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

  test("clicking Log out redirects to /login", async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);

    // Open user menu
    const userMenuBtn = page.getByRole("button", { name: "User menu" });
    await userMenuBtn.click();

    // Click Log out
    await page.getByRole("button", { name: "Log out" }).click();

    // Should redirect to /login
    await page.waitForURL(/\/login/, { timeout: 15000 });
    expect(page.url()).toContain("/login");
  });

  test("after logout, navigating to /dashboard redirects to /login", async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndGoToDashboard(page, token!);

    // Log out
    const userMenuBtn = page.getByRole("button", { name: "User menu" });
    await userMenuBtn.click();
    await page.getByRole("button", { name: "Log out" }).click();
    await page.waitForURL(/\/login/, { timeout: 15000 });

    // Try to navigate to /dashboard
    await page.goto("/dashboard");

    // Should be redirected back to /login
    await page.waitForURL(/\/login/, { timeout: 15000 });
    expect(page.url()).toContain("/login");
  });
});
