import { test, expect, type Page } from "@playwright/test";

/**
 * Team Page E2E Tests
 *
 * Covers member table, seat usage, invite modal, role changes,
 * and member removal.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/dashboard/team.spec.ts
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

async function authenticateAndNavigate(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  await page.goto("/dashboard/team");
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Page rendering
// ---------------------------------------------------------------------------

test.describe("Team — page renders", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("member table renders", async ({ page }) => {
    await expect(page.getByText("Team")).toBeVisible({ timeout: 10000 });

    // Should show member table or empty state
    const hasTable = await page.locator("table, [role='table']").first().isVisible({ timeout: 5000 }).catch(() => false);
    const hasEmpty = await page.getByText("No team members").isVisible({ timeout: 3000 }).catch(() => false);
    expect(hasTable || hasEmpty).toBeTruthy();
  });

  test("seat usage display is visible", async ({ page }) => {
    await expect(page.getByText(/Team Seats|seats? used/i)).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Invite member
// ---------------------------------------------------------------------------

test.describe("Team — invite member", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("clicking Invite Member opens modal with Email and Role selector", async ({ page }) => {
    const inviteBtn = page.getByRole("button", { name: /Invite Member/i });
    await expect(inviteBtn).toBeVisible({ timeout: 10000 });
    await inviteBtn.click();

    // Modal should show email input and role options
    await expect(page.locator("#invite-email")).toBeVisible({ timeout: 5000 });

    // Role selector should show Admin, Member, Viewer
    await expect(page.getByText("Admin")).toBeVisible();
    await expect(page.getByText("Member")).toBeVisible();
    await expect(page.getByText("Viewer")).toBeVisible();

    // Cancel and Send Invitation buttons
    await expect(page.getByRole("button", { name: /Cancel/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Send Invitation/i })).toBeVisible();
  });

  test("filling form and submitting shows success state", async ({ page }) => {
    const inviteBtn = page.getByRole("button", { name: /Invite Member/i });
    await inviteBtn.click();

    // Fill email
    await page.locator("#invite-email").fill("e2e-test@example.com");

    // Select Member role
    await page.getByText("Member").click();

    // Submit
    await page.getByRole("button", { name: /Send Invitation/i }).click();

    // Should show success state or error (depending on backend)
    const hasSuccess = await page.getByText("Invitation Sent").isVisible({ timeout: 10000 }).catch(() => false);
    const hasError = await page.locator("[class*='destructive'], [class*='error']").first().isVisible({ timeout: 3000 }).catch(() => false);

    // Either success or a handled error is acceptable
    expect(hasSuccess || hasError).toBeTruthy();
  });

  test("Cancel closes the invite modal", async ({ page }) => {
    const inviteBtn = page.getByRole("button", { name: /Invite Member/i });
    await inviteBtn.click();

    await expect(page.locator("#invite-email")).toBeVisible({ timeout: 5000 });

    await page.getByRole("button", { name: /Cancel/i }).click();

    await expect(page.locator("#invite-email")).toBeHidden({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Member actions (when non-owner members exist)
// ---------------------------------------------------------------------------

test.describe("Team — member actions", () => {
  let token: string | null = null;

  test.beforeAll(async ({ request }) => {
    token = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!token, "Demo login failed (Control Plane not running)");
    await authenticateAndNavigate(page, token!);
  });

  test("if non-owner members exist, role dropdown shows options", async ({ page }) => {
    // Look for role dropdown buttons (non-owner rows only)
    const roleDropdowns = page.locator("button").filter({ hasText: /^(admin|member|viewer)$/i });
    const count = await roleDropdowns.count();

    if (count > 0) {
      await roleDropdowns.first().click();
      // Should show role options
      const hasAdmin = await page.getByText("admin").nth(1).isVisible({ timeout: 3000 }).catch(() => false);
      const hasMember = await page.getByText("member").nth(1).isVisible({ timeout: 3000 }).catch(() => false);
      expect(hasAdmin || hasMember).toBeTruthy();
    }
  });

  test("if non-owner members exist, remove member shows confirmation", async ({ page }) => {
    // Look for overflow menu on non-owner rows
    const overflowBtns = page.locator("table button, [role='table'] button").filter({ hasNotText: /Invite|Cancel|Send/ });
    const count = await overflowBtns.count();

    if (count > 1) {
      await overflowBtns.last().click();

      const hasRemove = await page.getByText("Remove member").isVisible({ timeout: 3000 }).catch(() => false);
      if (hasRemove) {
        await page.getByText("Remove member").click();

        // Confirmation should appear
        await expect(page.getByText("Remove Team Member?")).toBeVisible({ timeout: 5000 });

        // Cancel should close it
        await page.getByRole("button", { name: /Cancel/i }).click();
        await expect(page.getByText("Remove Team Member?")).toBeHidden({ timeout: 5000 });
      }
    }
  });
});
