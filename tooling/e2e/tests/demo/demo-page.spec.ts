import { expect, test } from "../../fixtures/pages";

/**
 * Comprehensive Demo Page tests
 * Consolidates all demo page UI, navigation, and layout tests
 */
test.describe("Demo Page", () => {
  test.beforeEach(async ({ demoPage }) => {
    await demoPage.goto();
    await demoPage.waitForPageLoad();
  });

  test.describe("Page Load", () => {
    test("should load the demo page successfully", async ({ page }) => {
      await expect(page).toHaveURL(/\/demo$/);
      await expect(page.getByRole("heading", { name: /AllSource Event Store/i })).toBeVisible();
    });

    test("should display hero section with correct content", async ({ page }) => {
      // Check hero title
      await expect(page.getByRole("heading", { name: /AllSource Event Store/i })).toBeVisible();

      // Check hero description
      await expect(page.getByText(/AI-Native Event Sourcing Platform/i)).toBeVisible();
      await expect(
        page.getByText(/Experience the next generation of event streaming/i)
      ).toBeVisible();

      // Check performance stats
      await expect(page.getByText(/469K\/s/i)).toBeVisible();
      await expect(page.getByText(/Events Ingested/i)).toBeVisible();
      await expect(page.getByText(/<100ms/i)).toBeVisible();
      await expect(page.getByText(/80%/i)).toBeVisible();
      await expect(page.getByText(/Compression/i)).toBeVisible();
    });
  });

  test.describe("Feature Cards Display", () => {
    test("should display all 8 feature cards", async ({ page }) => {
      await expect(page.getByRole("button", { name: /Event Ingestion/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Powerful Queries/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Real-time Metrics/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Projections/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Enterprise Security/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Time Travel/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Event Analytics/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Event Pipelines/i })).toBeVisible();
    });

    test("should have exactly 8 feature cards", async ({ page }) => {
      // Feature cards are in a 2-column grid
      const featureGrid = page.locator(".grid.grid-cols-2");
      await expect(featureGrid.first()).toBeVisible();

      // Count all buttons that are feature cards
      const featureCardButtons = page.locator(".grid.grid-cols-2 button");
      await expect(featureCardButtons).toHaveCount(8);
    });
  });

  test.describe("Card Navigation", () => {
    test("should switch to Event Ingestion demo when clicking Events card", async ({
      demoPage,
      page,
    }) => {
      await demoPage.clickEventsCard();

      await expect(page.getByRole("heading", { name: /Live Event Ingestion/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Generate E-Commerce Events/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Generate IoT Sensor Data/i })).toBeVisible();
    });

    test("should switch to Query demo when clicking Queries card", async ({ demoPage, page }) => {
      await demoPage.clickQueriesCard();

      await expect(page.getByRole("heading", { name: /Query Engine/i })).toBeVisible();
      await expect(page.getByRole("button", { name: "By Entity", exact: true })).toBeVisible();
      await expect(page.getByRole("button", { name: "By Type", exact: true })).toBeVisible();
      await expect(page.getByRole("button", { name: "Time Range", exact: true })).toBeVisible();
    });

    test("should switch to Metrics demo when clicking Metrics card", async ({ demoPage, page }) => {
      await demoPage.clickMetricsCard();

      await expect(page.getByText("System Metrics").first()).toBeVisible();
      await expect(page.getByRole("button", { name: /Refresh/i })).toBeVisible();
    });

    test("should switch to Projections section when clicking Projections card", async ({
      demoPage,
      page,
    }) => {
      await demoPage.clickProjectionsCard();

      await expect(page.locator("h2").filter({ hasText: "Projections" })).toBeVisible();
    });

    test("should switch to Security section when clicking Security card", async ({
      demoPage,
      page,
    }) => {
      await demoPage.clickSecurityCard();

      await expect(page.getByRole("heading", { name: "Security", exact: true })).toBeVisible();
    });

    test("should switch to Time Travel section when clicking Time Travel card", async ({
      demoPage,
      page,
    }) => {
      await demoPage.clickTimeTravelCard();

      await expect(page.locator("h2").filter({ hasText: "Time Travel" })).toBeVisible();
    });

    test("should switch to Analytics section when clicking Analytics card", async ({
      demoPage,
      page,
    }) => {
      await demoPage.clickAnalyticsCard();

      await expect(page.locator("h2").filter({ hasText: "Analytics" })).toBeVisible();
    });

    test("should switch to Pipelines section when clicking Pipelines card", async ({
      demoPage,
      page,
    }) => {
      await demoPage.clickPipelinesCard();

      await expect(page.locator("h2").filter({ hasText: "Pipelines" })).toBeVisible();
    });
  });

  test.describe("Navigation Between Sections", () => {
    test("should navigate between different demo sections", async ({ demoPage, page }) => {
      // Start with Events
      await demoPage.clickEventsCard();
      await expect(page.getByRole("heading", { name: /Live Event Ingestion/i })).toBeVisible();

      // Switch to Queries
      await demoPage.clickQueriesCard();
      await expect(page.getByRole("heading", { name: /Query Engine/i })).toBeVisible();

      // Switch to Metrics
      await demoPage.clickMetricsCard();
      await expect(page.getByText("System Metrics").first()).toBeVisible();

      // Switch back to Events
      await demoPage.clickEventsCard();
      await expect(page.getByRole("heading", { name: /Live Event Ingestion/i })).toBeVisible();
    });

    test("should navigate through all 8 sections sequentially", async ({ demoPage, page }) => {
      // Events
      await demoPage.clickEventsCard();
      await expect(page.getByRole("heading", { name: /Live Event Ingestion/i })).toBeVisible();

      // Queries
      await demoPage.clickQueriesCard();
      await expect(page.getByRole("heading", { name: /Query Engine/i })).toBeVisible();

      // Metrics
      await demoPage.clickMetricsCard();
      await expect(page.getByText("System Metrics").first()).toBeVisible();

      // Projections
      await demoPage.clickProjectionsCard();
      await expect(page.locator("h2").filter({ hasText: "Projections" })).toBeVisible();

      // Security
      await demoPage.clickSecurityCard();
      await expect(page.getByRole("heading", { name: "Security", exact: true })).toBeVisible();

      // Time Travel
      await demoPage.clickTimeTravelCard();
      await expect(page.locator("h2").filter({ hasText: "Time Travel" })).toBeVisible();

      // Analytics
      await demoPage.clickAnalyticsCard();
      await expect(page.locator("h2").filter({ hasText: "Analytics" })).toBeVisible();

      // Pipelines
      await demoPage.clickPipelinesCard();
      await expect(page.locator("h2").filter({ hasText: "Pipelines" })).toBeVisible();
    });
  });

  test.describe("Responsive Layout", () => {
    test("should have responsive grid for feature cards", async ({ page }) => {
      const featureGrid = page.locator(".grid.grid-cols-2");
      await expect(featureGrid.first()).toBeVisible();
    });

    test("should maintain proper card alignment on smaller screens", async ({ page }) => {
      // Set viewport to tablet size
      await page.setViewportSize({ width: 768, height: 1024 });

      // Feature cards should still be visible
      await expect(page.getByRole("button", { name: /Event Ingestion/i })).toBeVisible();
      await expect(page.getByRole("button", { name: /Event Pipelines/i })).toBeVisible();
    });
  });

  test.describe("Accessibility", () => {
    test("should have accessible button roles for all feature cards", async ({ page }) => {
      const buttons = await page.getByRole("button").all();
      expect(buttons.length).toBeGreaterThan(5);
    });

    test("should have semantic headings for sections", async ({ page }) => {
      // Main page heading
      await expect(page.getByRole("heading", { name: /AllSource Event Store/i })).toBeVisible();
    });

    test("feature cards should have descriptive accessible names", async ({ page }) => {
      // Each feature card button should have accessible name including title
      const eventsCard = page.getByRole("button", { name: /Event Ingestion/i });
      await expect(eventsCard).toBeVisible();

      const queriesCard = page.getByRole("button", { name: /Powerful Queries/i });
      await expect(queriesCard).toBeVisible();

      // Verify tabindex is set for keyboard accessibility
      await expect(eventsCard).toHaveAttribute("tabindex", "0");
    });

    test("should support keyboard navigation", async ({ page }) => {
      // Tab to first feature card
      await page.keyboard.press("Tab");
      await page.keyboard.press("Tab");
      await page.keyboard.press("Tab");

      // The focus should be on a button (feature card)
      const focusedElement = page.locator(":focus");
      await expect(focusedElement).toBeVisible();
    });
  });
});
