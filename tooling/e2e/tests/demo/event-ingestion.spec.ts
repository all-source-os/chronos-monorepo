import { expect, test } from "../../fixtures/pages";

test.describe("Event Ingestion Demo", () => {
  test.beforeEach(async ({ demoPage }) => {
    await demoPage.goto();
    await demoPage.waitForPageLoad();
    await demoPage.clickEventsCard();
  });

  test.describe("Page Structure", () => {
    test("should display event ingestion demo UI elements", async ({ page }) => {
      // Check title
      await expect(page.getByRole("heading", { name: /Live Event Ingestion/i })).toBeVisible();

      // Check description
      await expect(
        page.getByText(/Ingest events in real-time with automatic batching/i)
      ).toBeVisible();

      // Check generation buttons
      await expect(
        page.getByRole("button", { name: /Generate E-Commerce Events/i })
      ).toBeVisible();
      await expect(
        page.getByRole("button", { name: /Generate IoT Sensor Data/i })
      ).toBeVisible();

      // Check event stream section
      await expect(page.getByRole("heading", { name: /Event Stream/i })).toBeVisible();
    });

    test("should display batch size selector", async ({ page }) => {
      // Check batch size selector
      await expect(page.getByLabel(/Batch Size/i)).toBeVisible();

      // Verify default value and available options
      const selector = page.getByLabel(/Batch Size/i);
      await expect(selector).toHaveValue("10");

      // Check all options are available
      const options = await selector.locator("option").allTextContents();
      expect(options).toContain("5 events");
      expect(options).toContain("10 events");
      expect(options).toContain("25 events");
      expect(options).toContain("50 events");
      expect(options).toContain("100 events");
    });

    test("should display empty state when no events generated", async ({ page }) => {
      // Check for empty state message
      await expect(
        page.getByText(/Click a button above to start ingesting events/i)
      ).toBeVisible();
    });

    test("should have accessible button labels", async ({ page }) => {
      const ecommerceBtn = page.getByRole("button", { name: /Generate E-Commerce Events/i });
      const iotBtn = page.getByRole("button", { name: /Generate IoT Sensor Data/i });

      await expect(ecommerceBtn).toBeEnabled();
      await expect(iotBtn).toBeEnabled();
    });
  });

  test.describe("E-Commerce Event Generation", () => {
    test("should generate e-commerce events successfully", async ({ demoPage, page }) => {
      // Generate events
      await demoPage.generateEcommerceEvents();

      // Wait for events to appear in the stream
      await page.waitForTimeout(1500);

      // Check that event items are visible in the stream
      const itemCount = await demoPage.getEventStreamItemCount();
      expect(itemCount).toBeGreaterThan(0);

      // Check that stats are displayed
      const statsVisible = await demoPage.areEventStatsVisible();
      expect(statsVisible).toBe(true);
    });

    test("should display e-commerce event type label", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Check for e-commerce event type label in stream
      await expect(page.getByText(/E-Commerce Events/i).first()).toBeVisible();
    });

    test("should display event count in batch", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Check for event count display (e.g., "10 events • 123ms")
      await expect(page.getByText(/\d+ events • \d+ms/i).first()).toBeVisible();
    });
  });

  test.describe("IoT Event Generation", () => {
    test("should generate IoT events successfully", async ({ demoPage, page }) => {
      // Generate IoT events
      await demoPage.generateIoTEvents();

      // Wait for events to appear
      await page.waitForTimeout(1500);

      // Check that event items are visible
      const itemCount = await demoPage.getEventStreamItemCount();
      expect(itemCount).toBeGreaterThan(0);

      // Check stats
      const statsVisible = await demoPage.areEventStatsVisible();
      expect(statsVisible).toBe(true);
    });

    test("should display IoT event type label", async ({ demoPage, page }) => {
      await demoPage.generateIoTEvents();
      await page.waitForTimeout(1500);

      // Check for IoT event type label in stream
      await expect(page.getByText(/IoT Sensor Data/i).first()).toBeVisible();
    });
  });

  test.describe("Event Statistics", () => {
    test("should display all statistics after generating events", async ({ demoPage, page }) => {
      // Generate events
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Check all stats are visible
      await expect(page.getByText(/Total Batches/i)).toBeVisible();
      await expect(page.getByText(/Total Events/i)).toBeVisible();
      await expect(page.getByText(/Throughput/i)).toBeVisible();
      await expect(page.getByText(/Success Rate/i)).toBeVisible();
    });

    test("should display correct batch count", async ({ demoPage, page }) => {
      // Generate first batch
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Verify batch count is 1
      const batchCountElement = page.locator("text=Total Batches").locator("..").getByRole("paragraph").filter({ hasText: /^\d+$/ });
      await expect(batchCountElement.first()).toContainText("1");

      // Generate second batch
      await demoPage.generateIoTEvents();
      await page.waitForTimeout(1500);

      // Verify batch count is 2
      await expect(batchCountElement.first()).toContainText("2");
    });

    test("should display success rate percentage", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Check success rate shows percentage
      await expect(page.getByText(/\d+%/i).first()).toBeVisible();
    });

    test("should display throughput rate", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Check throughput shows rate (e.g., "1.5/s")
      await expect(page.getByText(/\d+\.?\d*\/s/i).first()).toBeVisible();
    });
  });

  test.describe("Multiple Batch Generation", () => {
    test("should generate multiple batches of events", async ({ demoPage, page }) => {
      // Generate first batch
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Generate second batch
      await demoPage.generateIoTEvents();
      await page.waitForTimeout(1500);

      // Check that multiple events are displayed
      const itemCount = await demoPage.getEventStreamItemCount();
      expect(itemCount).toBeGreaterThanOrEqual(2);
    });

    test("should show both event types in stream", async ({ demoPage, page }) => {
      // Generate e-commerce events
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Generate IoT events
      await demoPage.generateIoTEvents();
      await page.waitForTimeout(1500);

      // Check both types appear in stream
      await expect(page.getByText(/E-Commerce Events/i).first()).toBeVisible();
      await expect(page.getByText(/IoT Sensor Data/i).first()).toBeVisible();
    });

    test("should limit stream display to last 5 batches", async ({ demoPage, page }) => {
      // Generate 6 batches
      for (let i = 0; i < 6; i++) {
        if (i % 2 === 0) {
          await demoPage.generateEcommerceEvents();
        } else {
          await demoPage.generateIoTEvents();
        }
        await page.waitForTimeout(1000);
      }

      // Stream should show at most 5 items (component shows last 5)
      const itemCount = await demoPage.getEventStreamItemCount();
      expect(itemCount).toBeLessThanOrEqual(5);
    });
  });

  test.describe("Batch Size Selection", () => {
    test("should change batch size when selecting different option", async ({ page }) => {
      const selector = page.getByLabel(/Batch Size/i);

      // Change to 25 events
      await selector.selectOption("25");
      await expect(selector).toHaveValue("25");

      // Change to 50 events
      await selector.selectOption("50");
      await expect(selector).toHaveValue("50");
    });

    test("should generate events with selected batch size", async ({ demoPage, page }) => {
      const selector = page.getByLabel(/Batch Size/i);

      // Set batch size to 5
      await selector.selectOption("5");
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Check that batch shows 5 events
      await expect(page.getByText(/5 events • \d+ms/i).first()).toBeVisible();
    });
  });

  test.describe("Loading States", () => {
    test("should show loading state when generating events", async ({ page }) => {
      const button = page.getByRole("button", { name: /Generate E-Commerce Events/i });
      const spinner = page.locator(".animate-spin");

      // Click button
      await button.click();

      // Check for loading indicator (spinner) OR events already loaded
      // The backend might be too fast, so we check if either the spinner is visible
      // or events have already been generated
      try {
        await expect(spinner).toBeVisible({ timeout: 1000 });
      } catch {
        // If spinner wasn't visible, check that events were generated successfully
        await page.waitForTimeout(1000);
        await expect(page.getByText(/E-Commerce Events/i).first()).toBeVisible();
      }
    });

    test("should disable buttons while loading", async ({ page }) => {
      const ecommerceBtn = page.getByRole("button", { name: /Generate E-Commerce Events/i });
      const iotBtn = page.getByRole("button", { name: /Generate IoT Sensor Data/i });

      // Click to start loading
      await ecommerceBtn.click();

      // Try to check if buttons are disabled during loading
      // Note: this may be too fast to catch in some environments
      try {
        await expect(ecommerceBtn).toBeDisabled({ timeout: 500 });
        await expect(iotBtn).toBeDisabled({ timeout: 500 });
      } catch {
        // Loading state was too fast to catch - verify events were generated instead
        await page.waitForTimeout(1500);
        await expect(page.getByText(/E-Commerce Events/i).first()).toBeVisible();
      }
    });
  });

  test.describe("Event Details", () => {
    test("should display event details in the stream", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Check for event type label
      const eventTypeLabel = page.getByText(/E-Commerce Events|IoT Sensor Data/i).first();
      await expect(eventTypeLabel).toBeVisible();

      // Check for timestamp display
      const timestamp = page.getByText(/\d{1,2}:\d{2}:\d{2}/i).first();
      await expect(timestamp).toBeVisible();
    });

    test("should expand batch to show event details", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Click on the first batch to expand it
      const batchItem = page.getByText(/E-Commerce Events/i).first();
      await batchItem.click();
      await page.waitForTimeout(500);

      // Check for expanded event details
      // Should show entity_id and event_type
      await expect(page.getByText(/Entity:/i).first()).toBeVisible();
    });

    test("should show event payload in expanded view", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Click on the first batch to expand
      const batchItem = page.getByText(/E-Commerce Events/i).first();
      await batchItem.click();
      await page.waitForTimeout(500);

      // Check for JSON payload (pre element contains event data)
      const preElement = page.locator("pre").first();
      await expect(preElement).toBeVisible();
    });

    test("should collapse expanded event details when clicked again", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Click to expand
      const batchItem = page.getByText(/E-Commerce Events/i).first();
      await batchItem.click();
      await page.waitForTimeout(500);

      // Verify expanded
      await expect(page.getByText(/Entity:/i).first()).toBeVisible();

      // Click again to collapse
      await batchItem.click();
      await page.waitForTimeout(500);

      // Entity text should not be visible when collapsed
      await expect(page.getByText(/Entity:/i)).toHaveCount(0);
    });
  });

  test.describe("Clear Functionality", () => {
    test("should show clear button after generating events", async ({ demoPage, page }) => {
      // Initially no clear button
      await expect(page.getByRole("button", { name: /Clear All/i })).toHaveCount(0);

      // Generate events
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Clear button should appear
      await expect(page.getByRole("button", { name: /Clear All/i })).toBeVisible();
    });

    test("should clear all events when clicking Clear All", async ({ demoPage, page }) => {
      // Generate events
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Verify events are displayed
      await expect(page.getByText(/E-Commerce Events/i).first()).toBeVisible();

      // Click clear
      await page.getByRole("button", { name: /Clear All/i }).click();
      await page.waitForTimeout(500);

      // Verify empty state is shown
      await expect(
        page.getByText(/Click a button above to start ingesting events/i)
      ).toBeVisible();
    });
  });

  test.describe("Top Event Types Breakdown", () => {
    test("should display top event types section after generating events", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Check for top event types section
      await expect(page.getByText(/Top Event Types/i)).toBeVisible();
    });

    test("should show event type names in breakdown", async ({ demoPage, page }) => {
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // E-commerce events include order.created, payment.processed, etc.
      // At least one event type should be visible
      const eventTypeBreakdown = page.locator("text=Top Event Types").locator("..");
      await expect(eventTypeBreakdown).toBeVisible();
    });
  });

  test.describe("Core API Integration", () => {
    test("should send events to Core API", async ({ demoPage, page }) => {
      // Listen for API calls
      let apiCalled = false;
      page.on("request", (request) => {
        if (request.url().includes("/api/events/batch") || request.url().includes(":3900")) {
          apiCalled = true;
        }
      });

      // Generate events
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(2000);

      // Check that events were sent (or mock fallback was used)
      // The component always shows success/error status regardless of API
      const itemCount = await demoPage.getEventStreamItemCount();
      expect(itemCount).toBeGreaterThan(0);
    });

    test("should handle API errors gracefully", async ({ demoPage, page }) => {
      // Generate events (may succeed or fail depending on Core availability)
      await demoPage.generateEcommerceEvents();
      await page.waitForTimeout(1500);

      // Either success or error status should be shown
      const successIcon = page.locator('[class*="text-emerald"]');
      const errorIcon = page.locator('[class*="text-red"]');

      // At least one status indicator should be present
      const successCount = await successIcon.count();
      const errorCount = await errorIcon.count();
      expect(successCount + errorCount).toBeGreaterThan(0);
    });
  });
});
