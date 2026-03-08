import { test, expect } from "@playwright/test";

/**
 * Suspicious Activity alerts page e2e tests.
 *
 * Mocks the Control Plane API so the page renders without a running backend.
 */

const mockAlerts = [
  {
    id: "sa-1",
    type: "excessive_failed_auth",
    tenant_id: "t-1",
    tenant_name: "Acme Corp",
    description: "142 failed authentication attempts in the last 15 minutes",
    severity: "high",
    timestamp: "2026-03-08T09:30:00Z",
    details: { attempts: 142, window_minutes: 15 },
  },
  {
    id: "sa-2",
    type: "rate_limit_exceeded",
    tenant_id: "t-2",
    tenant_name: "Globex Inc",
    description: "Rate limit exceeded 12 times in the past hour",
    severity: "medium",
    timestamp: "2026-03-08T08:45:00Z",
    details: { breaches: 12, window_hours: 1 },
  },
  {
    id: "sa-3",
    type: "token_abuse",
    tenant_id: "t-1",
    tenant_name: "Acme Corp",
    description: "API token used from 5 different countries within 10 minutes",
    severity: "high",
    timestamp: "2026-03-08T07:15:00Z",
    details: { countries: 5, window_minutes: 10 },
  },
  {
    id: "sa-4",
    type: "geo_anomaly",
    tenant_id: "t-3",
    tenant_name: "Initech",
    description: "Login from unusual geographic location: Antarctica",
    severity: "low",
    timestamp: "2026-03-07T23:00:00Z",
    details: { location: "Antarctica" },
  },
  {
    id: "sa-5",
    type: "unusual_query_pattern",
    tenant_id: "t-2",
    tenant_name: "Globex Inc",
    description: "Query volume 10x above baseline for the past 30 minutes",
    severity: "medium",
    timestamp: "2026-03-08T06:00:00Z",
    details: { multiplier: 10, window_minutes: 30 },
  },
];

function filterAlerts(params: URLSearchParams) {
  let filtered = [...mockAlerts];
  const severity = params.get("severity");
  const type = params.get("type");
  if (severity) {
    filtered = filtered.filter((a) => a.severity === severity);
  }
  if (type) {
    filtered = filtered.filter((a) => a.type === type);
  }
  return filtered;
}

test.describe("Suspicious Activity alerts", () => {
  test.beforeEach(async ({ page }) => {
    // Mock auth session
    await page.route("**/api/auth/session", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          data: {
            user: {
              id: "admin-1",
              email: "admin@allsource.co",
              role: "admin",
            },
          },
        }),
      })
    );

    // Mock suspicious activity endpoint
    await page.route(
      "**/api/v1/admin/security/suspicious-activity**",
      (route) => {
        const url = new URL(route.request().url());
        const filtered = filterAlerts(url.searchParams);
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ alerts: filtered, total: filtered.length }),
        });
      }
    );
  });

  test("alerts page loads and displays alert cards", async ({ page }) => {
    await page.goto("/security/alerts");
    await expect(page.getByTestId("suspicious-activity-page")).toBeVisible();

    // Grid of alerts should be visible
    await expect(page.getByTestId("alerts-grid")).toBeVisible();

    // All 5 alert cards should render
    await expect(page.getByTestId("alert-card-sa-1")).toBeVisible();
    await expect(page.getByTestId("alert-card-sa-2")).toBeVisible();
    await expect(page.getByTestId("alert-card-sa-3")).toBeVisible();
    await expect(page.getByTestId("alert-card-sa-4")).toBeVisible();
    await expect(page.getByTestId("alert-card-sa-5")).toBeVisible();

    // Verify card content
    await expect(page.getByTestId("alert-card-sa-1")).toContainText(
      "Excessive Failed Auth"
    );
    await expect(page.getByTestId("alert-card-sa-1")).toContainText(
      "Acme Corp"
    );
    await expect(page.getByTestId("alert-card-sa-1")).toContainText(
      "142 failed authentication attempts"
    );

    // Verify severity badges
    await expect(page.getByTestId("alert-severity-sa-1")).toContainText(
      "High"
    );
    await expect(page.getByTestId("alert-severity-sa-2")).toContainText(
      "Medium"
    );
    await expect(page.getByTestId("alert-severity-sa-4")).toContainText(
      "Low"
    );
  });

  test("summary badges show correct counts", async ({ page }) => {
    await page.goto("/security/alerts");
    await expect(page.getByTestId("alert-summary")).toBeVisible();

    await expect(page.getByTestId("summary-high")).toContainText("2 High");
    await expect(page.getByTestId("summary-medium")).toContainText(
      "2 Medium"
    );
    await expect(page.getByTestId("summary-low")).toContainText("1 Low");
  });

  test("severity filter works", async ({ page }) => {
    await page.goto("/security/alerts");
    await expect(page.getByTestId("alerts-grid")).toBeVisible();

    // Filter to high severity only
    await page.getByTestId("severity-filter").selectOption("high");

    // Wait for reload
    await page.waitForTimeout(500);

    // Only high severity alerts should be visible
    await expect(page.getByTestId("alert-card-sa-1")).toBeVisible();
    await expect(page.getByTestId("alert-card-sa-3")).toBeVisible();
    await expect(page.getByTestId("alert-card-sa-2")).not.toBeVisible();
    await expect(page.getByTestId("alert-card-sa-4")).not.toBeVisible();
    await expect(page.getByTestId("alert-card-sa-5")).not.toBeVisible();

    // Switch to low
    await page.getByTestId("severity-filter").selectOption("low");
    await page.waitForTimeout(500);

    await expect(page.getByTestId("alert-card-sa-4")).toBeVisible();
    await expect(page.getByTestId("alert-card-sa-1")).not.toBeVisible();
  });

  test("type filter works", async ({ page }) => {
    await page.goto("/security/alerts");
    await expect(page.getByTestId("alerts-grid")).toBeVisible();

    // Filter by token_abuse type
    await page.getByTestId("type-filter").selectOption("token_abuse");
    await page.waitForTimeout(500);

    await expect(page.getByTestId("alert-card-sa-3")).toBeVisible();
    await expect(page.getByTestId("alert-card-sa-1")).not.toBeVisible();
    await expect(page.getByTestId("alert-card-sa-2")).not.toBeVisible();
  });

  test("sidebar bell icon shows alert count badge", async ({ page }) => {
    await page.goto("/security/alerts");
    await expect(page.getByTestId("suspicious-activity-page")).toBeVisible();

    // Sidebar should show the alerts link with a badge
    await expect(page.getByTestId("sidebar-alerts-link")).toBeVisible();
    await expect(page.getByTestId("sidebar-alert-badge")).toBeVisible();
    await expect(page.getByTestId("sidebar-alert-badge")).toContainText("2");
  });

  test("alert card links to tenant detail page", async ({ page }) => {
    await page.goto("/security/alerts");
    await expect(page.getByTestId("alerts-grid")).toBeVisible();

    // Click on a non-token-abuse alert -> should link to tenant detail
    const card = page.getByTestId("alert-card-sa-1");
    const href = await card.getAttribute("href");
    expect(href).toBe("/tenants/t-1");
  });

  test("token abuse alert card links to security page", async ({ page }) => {
    await page.goto("/security/alerts");
    await expect(page.getByTestId("alerts-grid")).toBeVisible();

    // Token abuse alert links to security (token audit)
    const card = page.getByTestId("alert-card-sa-3");
    const href = await card.getAttribute("href");
    expect(href).toBe("/security");
  });

  test("shows empty state when no alerts match filters", async ({ page }) => {
    // Override route for this test to return empty
    await page.route(
      "**/api/v1/admin/security/suspicious-activity**",
      (route) => {
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ alerts: [], total: 0 }),
        });
      }
    );

    await page.goto("/security/alerts");
    await expect(page.getByTestId("suspicious-activity-page")).toBeVisible();
    await expect(page.getByTestId("alerts-empty")).toBeVisible();
    await expect(page.getByTestId("alerts-empty")).toContainText(
      "No suspicious activity alerts found"
    );
  });
});
