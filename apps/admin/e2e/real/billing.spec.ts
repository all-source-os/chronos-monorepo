import { test, expect } from "@playwright/test";

test.describe("Billing — real stack", () => {
  test("billing page loads with revenue section", async ({ page }) => {
    await page.goto("/billing");

    // Page should render (may show real or zero data)
    await expect(page.getByText(/revenue|billing/i).first()).toBeVisible({
      timeout: 15_000,
    });
  });

  test("revenue stat cards render", async ({ page }) => {
    await page.goto("/billing");

    // MRR card should exist
    await expect(page.getByText("MRR").first()).toBeVisible({
      timeout: 15_000,
    });
  });

  test("invoices section renders", async ({ page }) => {
    await page.goto("/billing");

    // Invoices section — either a table or empty state
    await expect(
      page.getByText(/invoices/i).first()
    ).toBeVisible({ timeout: 15_000 });
  });

  test("dunning section renders", async ({ page }) => {
    await page.goto("/billing");

    // Dunning section should be visible
    await expect(
      page.getByText(/dunning|failed payments/i).first()
    ).toBeVisible({ timeout: 15_000 });
  });
});
