import { test, expect } from "@playwright/test";

/**
 * Smoke Test Suite
 *
 * Basic health check: navigate to the root URL and verify the page loads.
 */
test.describe("Smoke", () => {
  test("homepage loads successfully", async ({ page }) => {
    const response = await page.goto("/");
    expect(response?.status()).toBeLessThan(400);
    await expect(page).toHaveTitle(/.+/);
  });
});
