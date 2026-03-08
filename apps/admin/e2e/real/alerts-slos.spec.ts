import { test, expect } from "@playwright/test";

test.describe("Alerts & SLOs — real stack", () => {
  test("alerts page loads", async ({ page }) => {
    await page.goto("/monitoring/alerts");

    await expect(
      page.getByText(/alert rules|alerts/i).first()
    ).toBeVisible({ timeout: 15_000 });
  });

  test("create alert rule flow", async ({ page }) => {
    await page.goto("/monitoring/alerts");

    await expect(
      page.getByText(/alert rules|alerts/i).first()
    ).toBeVisible({ timeout: 15_000 });

    // Click "Create Alert" or "Add Rule" button
    const createBtn = page.getByRole("button", {
      name: /create|add/i,
    });

    if (await createBtn.isVisible()) {
      await createBtn.click();

      // Fill the form
      const nameInput = page.getByLabel(/name/i).or(
        page.getByPlaceholder(/name/i)
      );
      if (await nameInput.isVisible()) {
        await nameInput.fill("E2E Test Alert");
      }
    }
  });

  test("SLOs page loads", async ({ page }) => {
    await page.goto("/monitoring/slos");

    await expect(
      page.getByText(/service level|slo/i).first()
    ).toBeVisible({ timeout: 15_000 });
  });
});
