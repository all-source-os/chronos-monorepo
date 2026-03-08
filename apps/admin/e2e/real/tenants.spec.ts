import { test, expect } from "@playwright/test";

test.describe("Tenants — real stack", () => {
  test("tenants page loads and shows table structure", async ({ page }) => {
    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-page")).toBeVisible();

    // Table should render (may have 0 or more rows from seed/existing data)
    const table = page.getByTestId("tenants-table").or(
      page.getByTestId("tenants-empty")
    );
    await expect(table).toBeVisible({ timeout: 15_000 });
  });

  test("table has expected column headers", async ({ page }) => {
    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-page")).toBeVisible();

    // Wait for either table or empty state
    const table = page.getByTestId("tenants-table");
    const empty = page.getByTestId("tenants-empty");

    await expect(table.or(empty)).toBeVisible({ timeout: 15_000 });

    // If table is visible, check headers
    if (await table.isVisible()) {
      await expect(table.getByText("Name")).toBeVisible();
      await expect(table.getByText("Plan")).toBeVisible();
      await expect(table.getByText("Status")).toBeVisible();
    }
  });

  test("search input is interactive", async ({ page }) => {
    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-page")).toBeVisible();

    const search = page.getByTestId("tenants-search");
    await expect(search).toBeVisible();
    await search.fill("nonexistent-tenant-xyz");

    // Wait for debounce
    await page.waitForTimeout(600);

    // Should show empty state or filtered results
    // (the search was for a nonsense string, so likely empty)
    const tableOrEmpty = page.getByTestId("tenants-table").or(
      page.getByTestId("tenants-empty")
    );
    await expect(tableOrEmpty).toBeVisible({ timeout: 10_000 });
  });

  test("filter dropdowns are present", async ({ page }) => {
    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-page")).toBeVisible();

    await expect(page.getByTestId("tenants-plan-filter")).toBeVisible();
    await expect(page.getByTestId("tenants-status-filter")).toBeVisible();
  });

  test("pagination info is visible", async ({ page }) => {
    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-page")).toBeVisible();

    // Wait for data to load
    const tableOrEmpty = page.getByTestId("tenants-table").or(
      page.getByTestId("tenants-empty")
    );
    await expect(tableOrEmpty).toBeVisible({ timeout: 15_000 });

    await expect(page.getByTestId("tenants-page-info")).toBeVisible();
  });
});
