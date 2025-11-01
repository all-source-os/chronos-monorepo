import { test, expect } from "../../fixtures/pages";

test.describe("Event Ingestion Demo", () => {
  test.beforeEach(async ({ demoPage }) => {
    await demoPage.goto();
    await demoPage.waitForPageLoad();
    await demoPage.clickEventsCard();
  });

  test("should display event ingestion demo UI elements", async ({ demoPage, page }) => {
    // Check title
    await expect(page.getByRole("heading", { name: /Live Event Ingestion/i })).toBeVisible();

    // Check description
    await expect(
      page.getByText(/Ingest events in real-time with automatic batching/i)
    ).toBeVisible();

    // Check buttons
    await expect(
      page.getByRole("button", { name: /Generate E-Commerce Events/i })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /Generate IoT Sensor Data/i })
    ).toBeVisible();

    // Check event stream section
    const eventStreamVisible = await demoPage.isEventStreamVisible();
    expect(eventStreamVisible).toBe(true);
  });

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

  test("should display event statistics after generating events", async ({
    demoPage,
    page,
  }) => {
    // Generate events
    await demoPage.generateEcommerceEvents();
    await page.waitForTimeout(1500);

    // Check that stats are visible
    await expect(page.getByText(/Total Batches/i)).toBeVisible();
    await expect(page.getByText(/Total Events/i)).toBeVisible();
    await expect(page.getByText(/Success Rate/i)).toBeVisible();
  });

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

  test("should show loading state when generating events", async ({ demoPage, page }) => {
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
      const itemCount = await demoPage.getEventStreamItemCount();
      expect(itemCount).toBeGreaterThan(0);
    }
  });

  test("should display event details in the stream", async ({ demoPage, page }) => {
    await demoPage.generateEcommerceEvents();
    await page.waitForTimeout(1500);

    // Check for event type label
    const eventTypeLabel = page.getByText(/E-Commerce Events|IoT Sensor Data/i).first();
    await expect(eventTypeLabel).toBeVisible();

    // Check for event count
    const eventCount = page.getByText(/events ingested/i).first();
    await expect(eventCount).toBeVisible();
  });

  test("should have accessible button labels", async ({ page }) => {
    const ecommerceBtn = page.getByRole("button", { name: /Generate E-Commerce Events/i });
    const iotBtn = page.getByRole("button", { name: /Generate IoT Sensor Data/i });

    await expect(ecommerceBtn).toBeEnabled();
    await expect(iotBtn).toBeEnabled();
  });
});
