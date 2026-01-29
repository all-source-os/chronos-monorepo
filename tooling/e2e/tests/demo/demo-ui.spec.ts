import { expect, test } from "../../fixtures/pages";

test.describe("Demo Page UI and Navigation", () => {
  test.beforeEach(async ({ demoPage }) => {
    await demoPage.goto();
    await demoPage.waitForPageLoad();
  });

  test("should load the demo page successfully", async ({ page }) => {
    await expect(page).toHaveTitle(/AllSource/i);
    await expect(page).toHaveURL(/\/demo$/);
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
    await expect(page.getByText(/Query Latency/i)).toBeVisible();
    await expect(page.getByText(/80%/i)).toBeVisible();
    await expect(page.getByText(/Compression/i)).toBeVisible();
  });

  test("should display all feature cards", async ({ page }) => {
    await expect(page.getByRole("button", { name: /Event Ingestion/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Powerful Queries/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Real-time Metrics/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /^Projections$/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Enterprise Security/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Time Travel/i })).toBeVisible();
  });

  test("should switch to Event Ingestion demo when clicking Events card", async ({
    demoPage,
    page,
  }) => {
    await demoPage.clickEventsCard();

    // Check that Event Ingestion demo is displayed
    await expect(page.getByRole("heading", { name: /Live Event Ingestion/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Generate E-Commerce Events/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Generate IoT Sensor Data/i })).toBeVisible();
  });

  test("should switch to Query demo when clicking Queries card", async ({ demoPage, page }) => {
    await demoPage.clickQueriesCard();

    // Check that Query demo is displayed
    await expect(page.getByRole("heading", { name: /Query Engine/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Query by Entity/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Query by Type/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Time Range Query/i })).toBeVisible();
  });

  test("should switch to Metrics demo when clicking Metrics card", async ({ demoPage, page }) => {
    await demoPage.clickMetricsCard();

    // Check that Metrics demo is displayed
    await expect(page.getByRole("heading", { name: /System Metrics/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Refresh/i })).toBeVisible();
  });

  test("should switch to Projections section when clicking Projections card", async ({
    demoPage,
    page,
  }) => {
    await demoPage.clickProjectionsCard();

    // Check that Projections section is displayed
    await expect(page.getByRole("heading", { name: /Projection Builder/i })).toBeVisible();
    await expect(page.getByText(/Entity Snapshots/i)).toBeVisible();
    await expect(page.getByText(/Event Counters/i)).toBeVisible();
    await expect(page.getByText(/Time Series/i)).toBeVisible();
  });

  test("should switch to Security section when clicking Security card", async ({
    demoPage,
    page,
  }) => {
    await demoPage.clickSecurityCard();

    // Check that Security section is displayed
    await expect(page.getByRole("heading", { name: /Enterprise Security/i })).toBeVisible();
    await expect(page.getByText(/Multi-Tenancy/i)).toBeVisible();
    await expect(page.getByText(/RBAC/i)).toBeVisible();
    await expect(page.getByText(/JWT Auth/i)).toBeVisible();
  });

  test("should switch to Time Travel section when clicking Time Travel card", async ({
    demoPage,
    page,
  }) => {
    await demoPage.clickTimeTravelCard();

    // Check that Time Travel section is displayed
    await expect(page.getByRole("heading", { name: /Temporal Queries/i })).toBeVisible();
    await expect(page.getByText(/Query historical state/i)).toBeVisible();
  });

  test("should navigate between different demo sections", async ({ demoPage, page }) => {
    // Start with Events
    await demoPage.clickEventsCard();
    await expect(page.getByRole("heading", { name: /Live Event Ingestion/i })).toBeVisible();

    // Switch to Queries
    await demoPage.clickQueriesCard();
    await expect(page.getByRole("heading", { name: /Query Engine/i })).toBeVisible();

    // Switch to Metrics
    await demoPage.clickMetricsCard();
    await expect(page.getByRole("heading", { name: /System Metrics/i })).toBeVisible();

    // Switch back to Events
    await demoPage.clickEventsCard();
    await expect(page.getByRole("heading", { name: /Live Event Ingestion/i })).toBeVisible();
  });

  test("should have responsive design elements", async ({ page }) => {
    // Check for responsive grid classes
    const featureGrid = page.locator(".grid-cols-2.md\\:grid-cols-3.lg\\:grid-cols-6");
    await expect(featureGrid).toBeVisible();
  });

  test("should have accessible button roles", async ({ page }) => {
    // All feature cards should be buttons
    const buttons = await page.getByRole("button").all();
    expect(buttons.length).toBeGreaterThan(5);
  });
});
