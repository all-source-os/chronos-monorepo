import { test, expect } from "@playwright/test";

/**
 * Monitoring dashboard e2e tests.
 *
 * These tests mock the API endpoints so the page can render fully
 * without a running Query Service backend.
 */

const mockSummary = {
  uptime_seconds: 93600,
  events_total: 1_420_000,
  events_per_second: 312.5,
  query_latency_p99_ms: 11.9,
  error_rate_percent: 0.03,
  active_tenants: 42,
};

const mockTimeseries = {
  points: Array.from({ length: 12 }, (_, i) => ({
    timestamp: new Date(Date.now() - (11 - i) * 5 * 60_000).toISOString(),
    value: 280 + Math.random() * 60,
  })),
};

const mockClusterMembers = {
  members: [
    {
      id: "core-leader-1",
      role: "leader",
      address: "10.0.1.10:3900",
      status: "healthy",
      uptime_seconds: 93600,
    },
    {
      id: "core-follower-1",
      role: "follower",
      address: "10.0.1.11:3900",
      status: "healthy",
      lag_ms: 12,
      uptime_seconds: 93200,
    },
    {
      id: "core-follower-2",
      role: "follower",
      address: "10.0.1.12:3900",
      status: "healthy",
      lag_ms: 8,
      uptime_seconds: 93100,
    },
  ],
};

test.describe("Monitoring dashboard", () => {
  test.beforeEach(async ({ page }) => {
    // Mock the auth session so the authenticated layout doesn't redirect
    await page.route("**/api/auth/session", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          data: {
            user: { id: "admin-1", email: "admin@allsource.co", role: "admin" },
          },
        }),
      })
    );

    // Mock metrics summary
    await page.route("**/api/admin/metrics/summary", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockSummary),
      })
    );

    // Mock timeseries
    await page.route("**/api/admin/metrics/timeseries*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockTimeseries),
      })
    );

    // Mock cluster members
    await page.route("**/api/cluster/members", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockClusterMembers),
      })
    );
  });

  test("dashboard loads with stat cards showing real data", async ({
    page,
  }) => {
    await page.goto("/monitoring");

    // Wait for the monitoring page to render
    await expect(page.getByTestId("monitoring-page")).toBeVisible();

    // Stat cards section should be visible
    await expect(page.getByTestId("stat-cards")).toBeVisible();

    // Verify stat cards display correct data
    const uptimeCard = page.getByTestId("stat-card-uptime");
    await expect(uptimeCard).toBeVisible();
    await expect(uptimeCard).toContainText("1d 2h 0m");

    const eventsCard = page.getByTestId("stat-card-events/sec");
    await expect(eventsCard).toBeVisible();
    await expect(eventsCard).toContainText("312.5");

    const latencyCard = page.getByTestId("stat-card-latency-p99");
    await expect(latencyCard).toBeVisible();
    await expect(latencyCard).toContainText("11.9ms");

    const errorCard = page.getByTestId("stat-card-error-rate");
    await expect(errorCard).toBeVisible();
    await expect(errorCard).toContainText("0.03%");

    const tenantsCard = page.getByTestId("stat-card-active-tenants");
    await expect(tenantsCard).toBeVisible();
    await expect(tenantsCard).toContainText("42");
  });

  test("charts render with data", async ({ page }) => {
    await page.goto("/monitoring");

    await expect(page.getByTestId("monitoring-page")).toBeVisible();

    // Charts section should be visible
    await expect(page.getByTestId("charts-section")).toBeVisible();

    // Both charts should render
    const throughputChart = page.getByTestId("chart-throughput");
    await expect(throughputChart).toBeVisible();
    await expect(throughputChart).toContainText("Throughput");

    const errorChart = page.getByTestId("chart-error-rate");
    await expect(errorChart).toBeVisible();
    await expect(errorChart).toContainText("Error Rate");

    // Recharts renders SVGs — verify at least one SVG exists inside chart containers
    await expect(throughputChart.locator("svg.recharts-surface")).toBeVisible();
    await expect(errorChart.locator("svg.recharts-surface")).toBeVisible();
  });

  test("cluster health section shows members", async ({ page }) => {
    await page.goto("/monitoring");

    await expect(page.getByTestId("monitoring-page")).toBeVisible();

    const clusterSection = page.getByTestId("cluster-health");
    await expect(clusterSection).toBeVisible();
    await expect(clusterSection).toContainText("Cluster Health");

    // Verify all three members render
    await expect(
      page.getByTestId("cluster-member-core-leader-1")
    ).toBeVisible();
    await expect(
      page.getByTestId("cluster-member-core-follower-1")
    ).toBeVisible();
    await expect(
      page.getByTestId("cluster-member-core-follower-2")
    ).toBeVisible();

    // Leader badge should be visible
    await expect(clusterSection.getByText("leader")).toBeVisible();
  });

  test("auto-refresh indicator is visible", async ({ page }) => {
    await page.goto("/monitoring");

    await expect(page.getByTestId("monitoring-page")).toBeVisible();

    // The auto-refresh indicator should be present
    const indicator = page.getByTestId("auto-refresh-indicator");
    await expect(indicator).toBeVisible();
    await expect(indicator).toContainText(/refresh/i);
  });

  test("range selector changes chart data", async ({ page }) => {
    let timeseriesCallCount = 0;

    // Track timeseries API calls
    await page.route("**/api/admin/metrics/timeseries*", (route) => {
      timeseriesCallCount++;
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockTimeseries),
      });
    });

    await page.goto("/monitoring");
    await expect(page.getByTestId("monitoring-page")).toBeVisible();

    // Wait for initial load (2 timeseries calls: throughput + error rate)
    await expect(page.getByTestId("chart-throughput")).toBeVisible();

    const initialCalls = timeseriesCallCount;

    // Click 24H range on the throughput chart
    const throughputChart = page.getByTestId("chart-throughput");
    await throughputChart.getByText("24H").click();

    // Wait for new API call
    await page.waitForTimeout(500);

    // Should have made additional timeseries calls
    expect(timeseriesCallCount).toBeGreaterThan(initialCalls);
  });
});
