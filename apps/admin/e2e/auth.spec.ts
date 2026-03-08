import { test, expect } from "@playwright/test";

test.describe("Admin authentication", () => {
  test("unauthenticated user is redirected to /login", async ({ page }) => {
    // Visit a protected route without a session
    await page.goto("/tenants");

    // Should be redirected to the login page
    await expect(page).toHaveURL(/\/login/);

    // The login page should display the admin sign-in form
    await expect(page.getByText("Admin Sign In")).toBeVisible();
  });

  test("root path redirects unauthenticated user to /login", async ({ page }) => {
    await page.goto("/");

    await expect(page).toHaveURL(/\/login/);
  });

  test("login page renders OAuth buttons", async ({ page }) => {
    await page.goto("/login");

    await expect(page.getByText("Continue with Google")).toBeVisible();
    await expect(page.getByText("Continue with GitHub")).toBeVisible();
    await expect(page.getByText("Continue with Email")).toBeVisible();
  });
});
