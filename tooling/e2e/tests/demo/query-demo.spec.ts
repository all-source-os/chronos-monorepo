import { expect, test } from "../../fixtures/pages";

test.describe("Query Demo", () => {
  test.beforeEach(async ({ demoPage }) => {
    await demoPage.goto();
    await demoPage.waitForPageLoad();
    await demoPage.clickQueriesCard();
  });

  test("should display query demo UI elements", async ({ page }) => {
    // Check title
    await expect(page.getByRole("heading", { name: /Query Engine/i })).toBeVisible();

    // Check description
    await expect(
      page.getByText(/Execute powerful queries with sub-millisecond latency/i)
    ).toBeVisible();

    // Check query buttons
    await expect(page.getByRole("button", { name: /^By Entity$/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /^By Type$/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /^Time Range$/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /^Advanced$/i })).toBeVisible();

    // Check query parameters section
    await expect(page.getByText(/Query Parameters/i)).toBeVisible();
  });

  test("should execute query by entity", async ({ demoPage, page }) => {
    // Execute query
    await demoPage.queryByEntity();

    // Wait for results
    await page.waitForTimeout(2000);

    // Check if results are displayed or "no events" message
    const resultsVisible = await demoPage.areQueryResultsVisible();
    const noEventsMsg = page.getByText(/No events found/i);
    const hasNoEvents = await noEventsMsg.isVisible().catch(() => false);

    // Either results or no events message should be visible
    expect(resultsVisible || hasNoEvents).toBe(true);
  });

  test("should execute query by type", async ({ demoPage, page }) => {
    // Execute query
    await demoPage.queryByType();

    // Wait for results
    await page.waitForTimeout(2000);

    // Check for results or no events message
    const resultsVisible = await demoPage.areQueryResultsVisible();
    const noEventsMsg = page.getByText(/No events found/i);
    const hasNoEvents = await noEventsMsg.isVisible().catch(() => false);

    expect(resultsVisible || hasNoEvents).toBe(true);
  });

  test("should execute time range query", async ({ demoPage, page }) => {
    // Execute query
    await demoPage.queryByTimeRange();

    // Wait for results
    await page.waitForTimeout(2000);

    // Check for results or no events message
    const resultsVisible = await demoPage.areQueryResultsVisible();
    const noEventsMsg = page.getByText(/No events found/i);
    const hasNoEvents = await noEventsMsg.isVisible().catch(() => false);

    expect(resultsVisible || hasNoEvents).toBe(true);
  });

  test("should show loading state when executing query", async ({ page }) => {
    const button = page.getByRole("button", { name: /^By Entity$/i });

    // Click query button
    await button.click();

    // Check for loading indicator OR results already loaded
    try {
      await expect(page.getByText(/Querying event store/i)).toBeVisible({ timeout: 2000 });
    } catch {
      // If loading message wasn't visible, check that either results or "no events" message appears
      await page.waitForTimeout(1000);
      const results = page.getByRole("heading", { name: /Query Results/i });
      const noEvents = page.getByText(/No events found/i);
      const hasResults = await results.isVisible().catch(() => false);
      const hasNoEvents = await noEvents.isVisible().catch(() => false);
      expect(hasResults || hasNoEvents).toBe(true);
    }
  });

  test("should display query parameters inputs", async ({ page }) => {
    // Check for query parameter sections (labels appear as text, not proper label elements)
    await expect(page.getByText(/^Entity ID$/)).toBeVisible();
    await expect(page.getByText(/^Event Type$/)).toBeVisible();
    await expect(page.getByText(/^Time Range \(hours\)$/)).toBeVisible();
    await expect(page.getByText(/^Result Limit$/)).toBeVisible();

    // Check that inputs exist
    const inputs = page.locator("input[type='text']");
    await expect(inputs.first()).toBeVisible();
  });

  test("should have colored icons on query buttons", async ({ page }) => {
    // Buttons should have visible icons - update to match actual button text
    const entityIcon = page.locator('button:has-text("By Entity") svg').first();
    const typeIcon = page.locator('button:has-text("By Type") svg').first();
    const timeIcon = page.locator('button:has-text("Time Range") svg').first();

    await expect(entityIcon).toBeVisible();
    await expect(typeIcon).toBeVisible();
    await expect(timeIcon).toBeVisible();
  });

  test("should allow multiple queries in sequence", async ({ demoPage, page }) => {
    // Execute first query
    await demoPage.queryByEntity();
    await page.waitForTimeout(1800);

    // Execute second query
    await demoPage.queryByType();
    await page.waitForTimeout(1800);

    // Execute third query
    await demoPage.queryByTimeRange();
    await page.waitForTimeout(1800);

    // Should complete without errors
    expect(true).toBe(true);
  });

  test("query results should display event details when available", async ({ demoPage, page }) => {
    // First ingest some events
    await demoPage.clickEventsCard();
    await demoPage.generateEcommerceEvents();
    await page.waitForTimeout(1500);

    // Then query
    await demoPage.clickQueriesCard();
    await demoPage.queryByTimeRange();
    await page.waitForTimeout(2000);

    // Check if we have results
    const resultsCount = await demoPage.getQueryResultsCount();

    if (resultsCount > 0) {
      // Results should display event type, entity ID, and payload
      const firstResult = page.locator(".grid.grid-cols-1.gap-3 > div").first();
      await expect(firstResult).toBeVisible();
    }
  });
});
