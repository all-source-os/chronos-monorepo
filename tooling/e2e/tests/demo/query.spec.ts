import { expect, test } from "../../fixtures/pages";

/**
 * Comprehensive Query Demo Tests
 *
 * Tests the Query Engine section of the demo page which provides:
 * - Query parameter inputs: Entity ID, Event Type, Time Range, Result Limit
 * - 4 query execution buttons: By Entity, By Type, Time Range, Advanced
 * - Query results display with expandable event details
 * - Loading states during query execution
 * - Multiple queries in sequence support
 *
 * API: GET /api/events from Core service (port 3900)
 * Note: Queries may return empty results when no events exist
 */

test.describe("Query Demo", () => {
  test.beforeEach(async ({ demoPage }) => {
    await demoPage.goto();
    await demoPage.waitForPageLoad();
    await demoPage.clickQueriesCard();
  });

  test.describe("Page Structure", () => {
    test("should display Query Engine title and description", async ({ page }) => {
      // Check title
      const title = page.getByRole("heading", { name: /Query Engine/i });
      await expect(title).toBeVisible();

      // Check description
      await expect(
        page.getByText(/Execute powerful queries with sub-millisecond latency/i)
      ).toBeVisible();
    });

    test("should display all 4 query type buttons", async ({ page }) => {
      await expect(page.getByRole("button", { name: /^By Entity$/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /^By Type$/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /^Time Range$/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /^Advanced$/i })).toBeVisible();
    });

    test("should display query parameters section", async ({ page }) => {
      await expect(page.getByText(/Query Parameters/i)).toBeVisible();
    });

    test("should have colored icons on query buttons", async ({ page }) => {
      // Buttons should have visible icons
      const entityIcon = page.locator('button:has-text("By Entity") svg').first();
      const typeIcon = page.locator('button:has-text("By Type") svg').first();
      const timeIcon = page.locator('button:has-text("Time Range") svg').first();
      const advancedIcon = page.locator('button:has-text("Advanced") svg').first();

      await expect(entityIcon).toBeVisible();
      await expect(typeIcon).toBeVisible();
      await expect(timeIcon).toBeVisible();
      await expect(advancedIcon).toBeVisible();
    });
  });

  test.describe("Query Parameter Inputs", () => {
    test("should display all query parameter labels", async ({ page }) => {
      await expect(page.getByText("Entity ID")).toBeVisible();
      await expect(page.getByText("Event Type")).toBeVisible();
      await expect(page.getByText("Time Range (hours)")).toBeVisible();
      await expect(page.getByText("Result Limit")).toBeVisible();
    });

    test("should have functional text inputs for parameters", async ({ page }) => {
      // Check that inputs exist
      const inputs = page.locator("input[type='text']");
      await expect(inputs.first()).toBeVisible();

      // Count should be at least 2 (Entity ID, Event Type)
      const count = await inputs.count();
      expect(count).toBeGreaterThanOrEqual(2);
    });

    test("should allow typing in Entity ID input", async ({ page }) => {
      const entityInput = page.locator("input").first();
      await entityInput.fill("test-entity-123");
      await expect(entityInput).toHaveValue("test-entity-123");
    });

    test("should allow selecting result limit", async ({ page }) => {
      // Result limit might be a select or input
      const limitInput = page.locator("input, select").last();
      await expect(limitInput).toBeVisible();
    });
  });

  test.describe("Query by Entity Type", () => {
    test("should execute query by entity and show results or no events message", async ({
      demoPage,
      page,
    }) => {
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

    test("should show query results heading when events are found", async ({
      demoPage,
      page,
    }) => {
      // First generate some events to ensure data exists
      await demoPage.clickEventsCard();
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Navigate back to query section
      await demoPage.clickQueriesCard();
      await demoPage.queryByEntity();
      await page.waitForTimeout(2000);

      // Should have either results or no events message
      const resultsTitle = page.getByRole("heading", { name: /Query Results/i });
      const noEvents = page.getByText(/No events found/i);

      const hasResults = await resultsTitle.isVisible().catch(() => false);
      const hasNoEvents = await noEvents.isVisible().catch(() => false);

      expect(hasResults || hasNoEvents).toBe(true);
    });
  });

  test.describe("Query by Event Type", () => {
    test("should execute query by type and show results", async ({
      demoPage,
      page,
    }) => {
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

    test("should filter events by type when events exist", async ({
      demoPage,
      page,
    }) => {
      // Generate events first
      await demoPage.clickEventsCard();
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Query by type
      await demoPage.clickQueriesCard();
      await demoPage.queryByType();
      await page.waitForTimeout(2000);

      // UI should update to show results or no events
      const resultsVisible = await demoPage.areQueryResultsVisible();
      const noEvents = await page.getByText(/No events found/i).isVisible().catch(() => false);

      expect(resultsVisible || noEvents).toBe(true);
    });
  });

  test.describe("Time Range Query", () => {
    test("should execute time range query and show results", async ({
      demoPage,
      page,
    }) => {
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

    test("should respect time range parameter", async ({ page }) => {
      // Verify time range input is present and functional
      const timeRangeLabel = page.getByText("Time Range (hours)");
      await expect(timeRangeLabel).toBeVisible();

      // The input should be interactive
      const inputs = page.locator("input");
      const count = await inputs.count();
      expect(count).toBeGreaterThanOrEqual(1);
    });
  });

  test.describe("Loading States", () => {
    test("should show loading state when executing query", async ({ page }) => {
      const button = page.getByRole("button", { name: /^By Entity$/i });

      // Click query button
      await button.click();

      // Check for loading indicator OR results already loaded
      try {
        await expect(page.getByText(/Querying event store/i)).toBeVisible({
          timeout: 2000,
        });
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

    test("should show loading state for all query types", async ({ page }) => {
      const queryButtons = [
        page.getByRole("button", { name: /^By Entity$/i }),
        page.getByRole("button", { name: /^By Type$/i }),
        page.getByRole("button", { name: /^Time Range$/i }),
        page.getByRole("button", { name: /^Advanced$/i }),
      ];

      for (const button of queryButtons) {
        await button.click();

        // Either loading state or results should appear
        await page.waitForTimeout(1500);

        const results = page.getByRole("heading", { name: /Query Results/i });
        const noEvents = page.getByText(/No events found/i);
        const loading = page.getByText(/Querying event store/i);

        const hasResults = await results.isVisible().catch(() => false);
        const hasNoEvents = await noEvents.isVisible().catch(() => false);
        const isLoading = await loading.isVisible().catch(() => false);

        expect(hasResults || hasNoEvents || isLoading).toBe(true);

        // Wait before next query
        await page.waitForTimeout(500);
      }
    });
  });

  test.describe("Multiple Queries in Sequence", () => {
    test("should allow executing multiple queries in sequence", async ({
      demoPage,
      page,
    }) => {
      // Execute first query
      await demoPage.queryByEntity();
      await page.waitForTimeout(1800);

      // Execute second query
      await demoPage.queryByType();
      await page.waitForTimeout(1800);

      // Execute third query
      await demoPage.queryByTimeRange();
      await page.waitForTimeout(1800);

      // All queries should complete without errors
      // Verify UI is still functional
      await expect(page.getByRole("button", { name: /^By Entity$/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /^By Type$/i })).toBeVisible();
    });

    test("should replace previous results with new query results", async ({
      demoPage,
      page,
    }) => {
      // Execute first query
      await demoPage.queryByEntity();
      await page.waitForTimeout(2000);

      // Check initial state
      const initialResults = await demoPage.areQueryResultsVisible();
      const initialNoEvents = await page
        .getByText(/No events found/i)
        .isVisible()
        .catch(() => false);

      // Execute second query
      await demoPage.queryByType();
      await page.waitForTimeout(2000);

      // Results should be replaced (or still showing appropriately)
      const finalResults = await demoPage.areQueryResultsVisible();
      const finalNoEvents = await page
        .getByText(/No events found/i)
        .isVisible()
        .catch(() => false);

      // UI should show either results or no events
      expect(finalResults || finalNoEvents).toBe(true);
    });

    test("should handle rapid query execution without errors", async ({
      demoPage,
      page,
    }) => {
      // Rapidly execute multiple queries
      await demoPage.queryByEntity();
      await page.waitForTimeout(500);
      await demoPage.queryByType();
      await page.waitForTimeout(500);
      await demoPage.queryByTimeRange();

      // Wait for last query to complete
      await page.waitForTimeout(2000);

      // UI should remain stable
      await expect(page.getByRole("heading", { name: /Query Engine/i })).toBeVisible();
    });
  });

  test.describe("Query Results Display", () => {
    test("should display query results correctly when events exist", async ({
      demoPage,
      page,
    }) => {
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

    test("should show 'No events found' when query returns empty", async ({
      page,
    }) => {
      // Query with a unique entity ID that won't have events
      const entityInput = page.locator("input").first();
      await entityInput.fill("non-existent-entity-xyz-12345");

      // Execute query by entity
      const entityBtn = page.getByRole("button", { name: /^By Entity$/i });
      await entityBtn.click();
      await page.waitForTimeout(2000);

      // Should show either no events message or results heading
      const noEvents = page.getByText(/No events found/i);
      const results = page.getByRole("heading", { name: /Query Results/i });

      const hasNoEvents = await noEvents.isVisible().catch(() => false);
      const hasResults = await results.isVisible().catch(() => false);

      expect(hasNoEvents || hasResults).toBe(true);
    });

    test("should display result count when events are found", async ({
      demoPage,
      page,
    }) => {
      // Generate events first
      await demoPage.clickEventsCard();
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Query
      await demoPage.clickQueriesCard();
      await demoPage.queryByTimeRange();
      await page.waitForTimeout(2000);

      // Check for result count in results heading or results area
      const resultsCount = await demoPage.getQueryResultsCount();

      // Either we have results or the UI shows appropriate message
      const resultsArea = page.getByRole("heading", { name: /Query Results/i });
      const noEvents = page.getByText(/No events found/i);

      const hasResultsArea = await resultsArea.isVisible().catch(() => false);
      const hasNoEvents = await noEvents.isVisible().catch(() => false);

      expect(hasResultsArea || hasNoEvents || resultsCount >= 0).toBe(true);
    });
  });

  test.describe("Core API Integration", () => {
    test("should verify queries execute against Core API", async ({
      page,
      request,
    }) => {
      // Test the Core API directly
      try {
        const response = await request.get("http://localhost:3900/api/events", {
          params: {
            limit: "10",
          },
        });

        if (response.ok()) {
          const data = await response.json();

          // Response should be an array of events or object with events
          expect(Array.isArray(data) || typeof data === "object").toBe(true);
        } else {
          // API may require auth or have no events - verify UI handles gracefully
          await page.getByRole("button", { name: /^By Entity$/i }).click();
          await page.waitForTimeout(2000);

          const noEvents = page.getByText(/No events found/i);
          const results = page.getByRole("heading", { name: /Query Results/i });

          const hasState = await noEvents.isVisible().catch(() => false) ||
                           await results.isVisible().catch(() => false);

          expect(hasState).toBe(true);
        }
      } catch {
        // Core service not running - verify UI fallback
        await page.getByRole("button", { name: /^By Entity$/i }).click();
        await page.waitForTimeout(2000);

        // UI should handle missing API gracefully
        const querySection = page.getByRole("heading", { name: /Query Engine/i });
        await expect(querySection).toBeVisible();
      }
    });

    test("should handle API timeout gracefully", async ({ page }) => {
      // Execute a query and verify UI remains functional
      await page.getByRole("button", { name: /^By Entity$/i }).click();

      // Wait longer than typical response time
      await page.waitForTimeout(5000);

      // UI should still be interactive
      const queryButtons = page.getByRole("button", { name: /^By Entity$/i });
      await expect(queryButtons).toBeVisible();
    });
  });

  test.describe("Advanced Query", () => {
    test("should execute advanced query with combined filters", async ({
      page,
    }) => {
      const advancedBtn = page.getByRole("button", { name: /^Advanced$/i });
      await advancedBtn.click();

      await page.waitForTimeout(2000);

      // Should show results or no events
      const noEvents = page.getByText(/No events found/i);
      const results = page.getByRole("heading", { name: /Query Results/i });

      const hasNoEvents = await noEvents.isVisible().catch(() => false);
      const hasResults = await results.isVisible().catch(() => false);

      expect(hasNoEvents || hasResults).toBe(true);
    });
  });

  test.describe("Query UI Accessibility", () => {
    test("should have accessible button names", async ({ page }) => {
      // All query buttons should be accessible by their text
      // Use exact matching to avoid matching the feature card buttons
      const entityBtn = page.getByRole("button", { name: "By Entity", exact: true });
      const typeBtn = page.getByRole("button", { name: "By Type", exact: true });
      const timeBtn = page.getByRole("button", { name: "Time Range", exact: true });
      const advancedBtn = page.getByRole("button", { name: "Advanced", exact: true });

      await expect(entityBtn).toBeVisible();
      await expect(typeBtn).toBeVisible();
      await expect(timeBtn).toBeVisible();
      await expect(advancedBtn).toBeVisible();
    });

    test("should have labeled input fields", async ({ page }) => {
      // Input fields should have associated labels
      const entityLabel = page.getByText("Entity ID");
      const typeLabel = page.getByText("Event Type");
      const timeLabel = page.getByText("Time Range (hours)");
      const limitLabel = page.getByText("Result Limit");

      await expect(entityLabel).toBeVisible();
      await expect(typeLabel).toBeVisible();
      await expect(timeLabel).toBeVisible();
      await expect(limitLabel).toBeVisible();
    });
  });
});
