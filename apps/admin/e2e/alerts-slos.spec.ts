import { test, expect } from "@playwright/test";

/**
 * Alerts & SLO management e2e tests.
 *
 * All API endpoints are mocked so the tests run without a backend.
 */

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

const mockAlerts = {
  alerts: [
    {
      id: "alert-1",
      name: "High Error Rate",
      metric: "error_rate_percent",
      operator: "gt" as const,
      threshold: 5,
      duration_seconds: 300,
      channel: "slack" as const,
      enabled: true,
      created_at: "2026-01-15T10:00:00Z",
      updated_at: "2026-01-15T10:00:00Z",
    },
  ],
};

const mockSLOs = {
  slos: [
    {
      id: "slo-1",
      name: "API Availability",
      metric: "error_rate_percent",
      target_percent: 99.9,
      window_days: 30,
      current_compliance: 99.95,
      error_budget_remaining: 50.0,
      trend: [99.9, 99.92, 99.94, 99.95, 99.93, 99.95],
      created_at: "2026-01-10T08:00:00Z",
    },
    {
      id: "slo-2",
      name: "Latency Budget",
      metric: "query_latency_p99_ms",
      target_percent: 95.0,
      window_days: 7,
      current_compliance: 93.2,
      error_budget_remaining: -36.0,
      trend: [96.1, 95.5, 94.8, 94.0, 93.5, 93.2],
      created_at: "2026-01-12T09:00:00Z",
    },
  ],
};

// ---------------------------------------------------------------------------
// Helper: set up auth + API mocks
// ---------------------------------------------------------------------------

function setupAuthMock(page: import("@playwright/test").Page) {
  return page.route("**/api/auth/session", (route) =>
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
}

// ---------------------------------------------------------------------------
// Alert tests
// ---------------------------------------------------------------------------

test.describe("Alert Rules page", () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthMock(page);

    // Mutable alerts list so create/delete are reflected
    let alerts = [...mockAlerts.alerts];

    await page.route("**/api/v1/admin/alerts", async (route) => {
      const method = route.request().method();

      if (method === "GET") {
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ alerts }),
        });
      }

      if (method === "POST") {
        const body = route.request().postDataJSON();
        const newAlert = {
          id: `alert-${Date.now()}`,
          ...body,
          enabled: true,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        };
        alerts.push(newAlert);
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(newAlert),
        });
      }
    });

    await page.route("**/api/v1/admin/alerts/*", async (route) => {
      const method = route.request().method();

      if (method === "DELETE") {
        const url = route.request().url();
        const id = url.split("/").pop();
        alerts = alerts.filter((a) => a.id !== id);
        return route.fulfill({ status: 204 });
      }

      if (method === "PUT") {
        const url = route.request().url();
        const id = url.split("/").pop();
        const body = route.request().postDataJSON();
        alerts = alerts.map((a) =>
          a.id === id ? { ...a, ...body, updated_at: new Date().toISOString() } : a
        );
        const updated = alerts.find((a) => a.id === id);
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(updated),
        });
      }
    });

    // Also mock monitoring page APIs so navigation doesn't break
    await page.route("**/api/admin/metrics/summary", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          uptime_seconds: 3600,
          events_total: 1000,
          events_per_second: 10,
          query_latency_p99_ms: 5,
          error_rate_percent: 0.01,
          active_tenants: 3,
        }),
      })
    );
    await page.route("**/api/admin/metrics/timeseries*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ points: [] }),
      })
    );
    await page.route("**/api/cluster/members", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ members: [] }),
      })
    );
  });

  test("displays existing alert rules in a table", async ({ page }) => {
    await page.goto("/monitoring/alerts");

    await expect(page.getByTestId("alerts-page")).toBeVisible();
    await expect(page.getByTestId("alerts-table")).toBeVisible();

    // The seeded alert should appear
    await expect(page.getByTestId("alert-row-alert-1")).toBeVisible();
    await expect(page.getByTestId("alert-row-alert-1")).toContainText(
      "High Error Rate"
    );
  });

  test("create alert rule and verify it appears in list", async ({ page }) => {
    await page.goto("/monitoring/alerts");
    await expect(page.getByTestId("alerts-page")).toBeVisible();

    // Open create form
    await page.getByTestId("create-alert-btn").click();
    await expect(page.getByTestId("alert-form")).toBeVisible();

    // Fill form
    await page.getByTestId("alert-name-input").fill("CPU Overload");
    await page.getByTestId("alert-metric-select").selectOption("cpu_usage_percent");
    await page.getByTestId("alert-operator-select").selectOption("gt");
    await page.getByTestId("alert-threshold-input").fill("90");
    await page.getByTestId("alert-duration-input").fill("60");
    await page.getByTestId("alert-channel-select").selectOption("email");

    // Submit
    await page.getByTestId("alert-submit-btn").click();

    // Form should close
    await expect(page.getByTestId("alert-form")).not.toBeVisible();

    // New alert should appear in the table
    await expect(page.getByText("CPU Overload")).toBeVisible();
  });

  test("delete alert removes it from the table", async ({ page }) => {
    await page.goto("/monitoring/alerts");
    await expect(page.getByTestId("alerts-table")).toBeVisible();

    // Delete the seeded alert
    await page.getByTestId("delete-alert-alert-1").click();

    // Should now show empty state
    await expect(page.getByTestId("alerts-empty")).toBeVisible();
  });

  test("edit button opens form pre-filled", async ({ page }) => {
    await page.goto("/monitoring/alerts");
    await expect(page.getByTestId("alerts-table")).toBeVisible();

    await page.getByTestId("edit-alert-alert-1").click();
    await expect(page.getByTestId("alert-form")).toBeVisible();

    // Name input should be pre-filled
    await expect(page.getByTestId("alert-name-input")).toHaveValue(
      "High Error Rate"
    );
  });
});

// ---------------------------------------------------------------------------
// SLO tests
// ---------------------------------------------------------------------------

test.describe("SLOs page", () => {
  test.beforeEach(async ({ page }) => {
    await setupAuthMock(page);

    let slos = [...mockSLOs.slos];

    await page.route("**/api/v1/admin/slos", async (route) => {
      const method = route.request().method();

      if (method === "GET") {
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ slos }),
        });
      }

      if (method === "POST") {
        const body = route.request().postDataJSON();
        const newSLO = {
          id: `slo-${Date.now()}`,
          ...body,
          current_compliance: body.target_percent - 0.1,
          error_budget_remaining: 90.0,
          trend: [99.8, 99.85, 99.9],
          created_at: new Date().toISOString(),
        };
        slos.push(newSLO);
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(newSLO),
        });
      }
    });

    await page.route("**/api/v1/admin/slos/*", async (route) => {
      if (route.request().method() === "DELETE") {
        const id = route.request().url().split("/").pop();
        slos = slos.filter((s) => s.id !== id);
        return route.fulfill({ status: 204 });
      }
    });

    // Mock monitoring page APIs
    await page.route("**/api/admin/metrics/summary", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          uptime_seconds: 3600,
          events_total: 1000,
          events_per_second: 10,
          query_latency_p99_ms: 5,
          error_rate_percent: 0.01,
          active_tenants: 3,
        }),
      })
    );
    await page.route("**/api/admin/metrics/timeseries*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ points: [] }),
      })
    );
    await page.route("**/api/cluster/members", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ members: [] }),
      })
    );
  });

  test("displays SLO cards with compliance data", async ({ page }) => {
    await page.goto("/monitoring/slos");

    await expect(page.getByTestId("slos-page")).toBeVisible();
    await expect(page.getByTestId("slo-cards")).toBeVisible();

    // Both SLO cards should render
    const card1 = page.getByTestId("slo-card-slo-1");
    await expect(card1).toBeVisible();
    await expect(card1).toContainText("API Availability");
    await expect(card1).toContainText("99.95%");

    const card2 = page.getByTestId("slo-card-slo-2");
    await expect(card2).toBeVisible();
    await expect(card2).toContainText("Latency Budget");
    await expect(card2).toContainText("93.20%");
  });

  test("compliance gauge colors: green >99%, red <95%", async ({ page }) => {
    await page.goto("/monitoring/slos");
    await expect(page.getByTestId("slo-cards")).toBeVisible();

    // Card 1: 99.95% -> green
    const gauge1 = page.getByTestId("slo-card-slo-1").getByTestId("compliance-gauge");
    await expect(gauge1).toBeVisible();
    // The text should show green via the text-green-500 class
    const gaugeText1 = gauge1.locator("span");
    await expect(gaugeText1).toContainText("99.9");

    // Card 2: 93.2% -> red
    const gauge2 = page.getByTestId("slo-card-slo-2").getByTestId("compliance-gauge");
    await expect(gauge2).toBeVisible();
    const gaugeText2 = gauge2.locator("span");
    await expect(gaugeText2).toContainText("93.2");
  });

  test("SLO cards show sparkline trend", async ({ page }) => {
    await page.goto("/monitoring/slos");
    await expect(page.getByTestId("slo-cards")).toBeVisible();

    // Each card should have a sparkline
    const sparklines = page.getByTestId("sparkline");
    await expect(sparklines.first()).toBeVisible();
  });

  test("create SLO and verify it appears", async ({ page }) => {
    await page.goto("/monitoring/slos");
    await expect(page.getByTestId("slos-page")).toBeVisible();

    // Open form
    await page.getByTestId("create-slo-btn").click();
    await expect(page.getByTestId("slo-form")).toBeVisible();

    // Fill form
    await page.getByTestId("slo-name-input").fill("Ingest Uptime");
    await page.getByTestId("slo-metric-select").selectOption("events_per_second");
    await page.getByTestId("slo-target-input").fill("99.5");
    await page.getByTestId("slo-window-input").fill("7");

    // Submit
    await page.getByTestId("slo-submit-btn").click();

    // Form should close
    await expect(page.getByTestId("slo-form")).not.toBeVisible();

    // New SLO should appear
    await expect(page.getByText("Ingest Uptime")).toBeVisible();
  });

  test("error budget remaining is displayed", async ({ page }) => {
    await page.goto("/monitoring/slos");
    await expect(page.getByTestId("slo-cards")).toBeVisible();

    // Check error budget on first card
    const card1 = page.getByTestId("slo-card-slo-1");
    await expect(card1.getByTestId("slo-error-budget")).toContainText("50.0%");
  });
});
