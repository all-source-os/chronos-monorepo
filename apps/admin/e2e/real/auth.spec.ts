import { test, expect } from "@playwright/test";

test.describe("Auth — real stack", () => {
  test("unauthenticated user is redirected to /login", async ({ browser }) => {
    // Fresh context with no cookies
    const ctx = await browser.newContext();
    const page = await ctx.newPage();

    await page.goto("/tenants");
    await expect(page).toHaveURL(/\/login/);
    await expect(page.getByText("Admin Sign In")).toBeVisible();

    await ctx.close();
  });

  test("authenticated admin can access /tenants", async ({ page }) => {
    // storageState from global-setup provides admin_token cookie
    await page.goto("/tenants");

    // Should NOT redirect to login
    await expect(page).not.toHaveURL(/\/login/);
    await expect(page.getByTestId("tenants-page")).toBeVisible();
  });

  test("login page renders OAuth buttons", async ({ browser }) => {
    const ctx = await browser.newContext();
    const page = await ctx.newPage();

    await page.goto("/login");
    await expect(page.getByText("Continue with Google")).toBeVisible();
    await expect(page.getByText("Continue with GitHub")).toBeVisible();

    await ctx.close();
  });
});
