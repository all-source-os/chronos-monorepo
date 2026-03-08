import { test, expect } from "@playwright/test";

test.describe("Monitoring — real stack", () => {
  test("monitoring page loads", async ({ page }) => {
    await page.goto("/monitoring");
    await expect(page.getByTestId("monitoring-page")).toBeVisible({
      timeout: 15_000,
    });
  });

  test("stat cards section renders", async ({ page }) => {
    await page.goto("/monitoring");
    await expect(page.getByTestId("monitoring-page")).toBeVisible({
      timeout: 15_000,
    });

    // Stat cards should appear (values depend on real metrics)
    await expect(page.getByTestId("stat-cards")).toBeVisible();

    // At minimum, the uptime card should exist
    await expect(page.getByTestId("stat-card-uptime")).toBeVisible();
  });

  test("charts section renders with SVGs", async ({ page }) => {
    await page.goto("/monitoring");
    await expect(page.getByTestId("monitoring-page")).toBeVisible({
      timeout: 15_000,
    });

    await expect(page.getByTestId("charts-section")).toBeVisible();

    const throughputChart = page.getByTestId("chart-throughput");
    await expect(throughputChart).toBeVisible();
    await expect(throughputChart).toContainText("Throughput");

    // Recharts should render SVGs
    await expect(
      throughputChart.locator("svg.recharts-surface")
    ).toBeVisible({ timeout: 10_000 });
  });

  test("cluster health section shows at least one member", async ({
    page,
  }) => {
    await page.goto("/monitoring");
    await expect(page.getByTestId("monitoring-page")).toBeVisible({
      timeout: 15_000,
    });

    const clusterSection = page.getByTestId("cluster-health");
    await expect(clusterSection).toBeVisible();
    await expect(clusterSection).toContainText("Cluster Health");

    // At least one cluster member should be visible
    // The data-testid pattern is cluster-member-<id>
    const members = clusterSection.locator("[data-testid^='cluster-member-']");
    await expect(members.first()).toBeVisible({ timeout: 10_000 });
  });

  test("auto-refresh indicator is visible", async ({ page }) => {
    await page.goto("/monitoring");
    await expect(page.getByTestId("monitoring-page")).toBeVisible({
      timeout: 15_000,
    });

    await expect(page.getByTestId("auto-refresh-indicator")).toBeVisible();
  });
});
