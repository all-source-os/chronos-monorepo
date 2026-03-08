import { test, expect } from "@playwright/test";

test.describe("Security — real stack", () => {
  test("security page loads", async ({ page }) => {
    await page.goto("/security");

    await expect(
      page.getByText(/security|ip rules/i).first()
    ).toBeVisible({ timeout: 15_000 });
  });

  test("IP rules section renders", async ({ page }) => {
    await page.goto("/security");

    // Should see an "Add Rule" button or the rules table
    await expect(
      page.getByText(/add rule|ip rules/i).first()
    ).toBeVisible({ timeout: 15_000 });
  });

  test("suspicious activity page loads", async ({ page }) => {
    await page.goto("/security/alerts");

    // Should render the alerts page (may have 0 alerts on clean stack)
    await expect(
      page.getByText(/suspicious|security alerts/i).first()
    ).toBeVisible({ timeout: 15_000 });
  });
});
