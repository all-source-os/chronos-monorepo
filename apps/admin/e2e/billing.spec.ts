import { test, expect } from "@playwright/test";

/**
 * Billing dashboard e2e tests.
 *
 * These tests mock the Control Plane billing API endpoints so the page
 * can render fully without a running backend.
 */

const mockRevenue = {
  mrr: 48500,
  arr: 582000,
  growth_rate: 12.3,
  churn_rate: 1.8,
  trend: Array.from({ length: 12 }, (_, i) => ({
    date: new Date(2026, 0 + i, 1).toISOString(),
    mrr: 38000 + i * 1000 + Math.round(Math.random() * 500),
  })),
};

const mockInvoices = {
  invoices: [
    {
      id: "inv-001",
      tenant_id: "t-acme",
      tenant_name: "Acme Corp",
      amount: 1200,
      currency: "USD",
      status: "paid",
      created_at: "2026-02-15T10:00:00Z",
    },
    {
      id: "inv-002",
      tenant_id: "t-globex",
      tenant_name: "Globex Inc",
      amount: 850,
      currency: "USD",
      status: "pending",
      created_at: "2026-02-20T14:00:00Z",
    },
    {
      id: "inv-003",
      tenant_id: "t-initech",
      tenant_name: "Initech LLC",
      amount: 2400,
      currency: "USD",
      status: "failed",
      created_at: "2026-02-25T09:00:00Z",
    },
  ],
  total: 3,
  page: 1,
  per_page: 20,
};

const mockDunning = {
  entries: [
    {
      tenant_id: "t-initech",
      tenant_name: "Initech LLC",
      retry_count: 3,
      last_attempt: "2026-03-01T08:30:00Z",
      amount_due: 2400,
      currency: "USD",
    },
    {
      tenant_id: "t-umbrella",
      tenant_name: "Umbrella Corp",
      retry_count: 1,
      last_attempt: "2026-03-05T16:00:00Z",
      amount_due: 900,
      currency: "USD",
    },
  ],
};

test.describe("Billing dashboard", () => {
  test.beforeEach(async ({ page }) => {
    // Mock auth session
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

    // Mock revenue endpoint
    await page.route("**/api/v1/admin/billing/revenue*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockRevenue),
      })
    );

    // Mock invoices endpoint
    await page.route("**/api/v1/admin/billing/invoices*", (route) => {
      const url = new URL(route.request().url());
      const statusParam = url.searchParams.get("status");

      if (statusParam) {
        const filtered = mockInvoices.invoices.filter(
          (inv) => inv.status === statusParam
        );
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({
            invoices: filtered,
            total: filtered.length,
            page: 1,
            per_page: 20,
          }),
        });
      }

      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockInvoices),
      });
    });

    // Mock dunning endpoint
    await page.route("**/api/v1/admin/billing/dunning", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockDunning),
      })
    );

    // Mock refund endpoint
    await page.route("**/api/v1/admin/billing/refund", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ success: true }),
      })
    );
  });

  test("dashboard loads revenue stat cards", async ({ page }) => {
    await page.goto("/billing");

    await expect(page.getByTestId("billing-page")).toBeVisible();
    await expect(page.getByTestId("billing-stat-cards")).toBeVisible();

    // MRR card
    const mrrCard = page.getByTestId("stat-card-mrr");
    await expect(mrrCard).toBeVisible();
    await expect(mrrCard).toContainText("$48,500.00");

    // ARR card
    const arrCard = page.getByTestId("stat-card-arr");
    await expect(arrCard).toBeVisible();
    await expect(arrCard).toContainText("$582,000.00");

    // Growth rate card
    const growthCard = page.getByTestId("stat-card-growth-rate");
    await expect(growthCard).toBeVisible();
    await expect(growthCard).toContainText("+12.3%");

    // Churn rate card
    const churnCard = page.getByTestId("stat-card-churn-rate");
    await expect(churnCard).toBeVisible();
    await expect(churnCard).toContainText("-1.8%");
  });

  test("revenue chart renders with range selector", async ({ page }) => {
    await page.goto("/billing");
    await expect(page.getByTestId("billing-page")).toBeVisible();

    const chart = page.getByTestId("revenue-chart");
    await expect(chart).toBeVisible();
    await expect(chart).toContainText("Revenue Trend");

    // Range selector buttons should be visible
    const rangeSelector = page.getByTestId("revenue-range-selector");
    await expect(rangeSelector).toBeVisible();
    await expect(page.getByTestId("range-30d")).toBeVisible();
    await expect(page.getByTestId("range-90d")).toBeVisible();
    await expect(page.getByTestId("range-1y")).toBeVisible();
  });

  test("range selector triggers new data fetch", async ({ page }) => {
    let revenueCalls = 0;

    await page.route("**/api/v1/admin/billing/revenue*", (route) => {
      revenueCalls++;
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockRevenue),
      });
    });

    await page.goto("/billing");
    await expect(page.getByTestId("billing-page")).toBeVisible();

    const initialCalls = revenueCalls;

    // Click 90D range
    await page.getByTestId("range-90d").click();
    await page.waitForTimeout(300);

    expect(revenueCalls).toBeGreaterThan(initialCalls);
  });

  test("invoices table displays data", async ({ page }) => {
    await page.goto("/billing");
    await expect(page.getByTestId("billing-page")).toBeVisible();

    const invoicesSection = page.getByTestId("invoices-section");
    await expect(invoicesSection).toBeVisible();

    // Table should render all 3 invoices
    await expect(page.getByTestId("invoice-inv-001")).toBeVisible();
    await expect(page.getByTestId("invoice-inv-002")).toBeVisible();
    await expect(page.getByTestId("invoice-inv-003")).toBeVisible();

    // Verify tenant names
    await expect(page.getByTestId("invoice-inv-001")).toContainText("Acme Corp");
    await expect(page.getByTestId("invoice-inv-002")).toContainText("Globex Inc");

    // Verify status badges
    await expect(page.getByTestId("invoice-inv-001")).toContainText("paid");
    await expect(page.getByTestId("invoice-inv-003")).toContainText("failed");
  });

  test("invoices table filters by status", async ({ page }) => {
    await page.goto("/billing");
    await expect(page.getByTestId("billing-page")).toBeVisible();

    // Select "paid" status filter
    const statusFilter = page.getByTestId("status-filter");
    await statusFilter.selectOption("paid");

    // Wait for filter to apply
    await page.waitForTimeout(300);

    // Only paid invoices should show
    await expect(page.getByTestId("invoice-inv-001")).toBeVisible();
    await expect(page.getByTestId("invoice-inv-002")).not.toBeVisible();
    await expect(page.getByTestId("invoice-inv-003")).not.toBeVisible();
  });

  test("refund button opens confirmation dialog", async ({ page }) => {
    await page.goto("/billing");
    await expect(page.getByTestId("billing-page")).toBeVisible();

    // Refund button should only appear on paid invoices
    await expect(page.getByTestId("refund-btn-inv-001")).toBeVisible();
    // No refund button on pending/failed invoices
    expect(await page.getByTestId("refund-btn-inv-002").count()).toBe(0);
    expect(await page.getByTestId("refund-btn-inv-003").count()).toBe(0);

    // Click refund
    await page.getByTestId("refund-btn-inv-001").click();

    // Dialog should appear
    const dialog = page.getByTestId("refund-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText("Confirm Refund");
    await expect(dialog).toContainText("Acme Corp");
    await expect(dialog).toContainText("$1,200.00");

    // Cancel should close dialog
    await page.getByTestId("refund-cancel").click();
    await expect(dialog).not.toBeVisible();
  });

  test("refund confirmation processes refund", async ({ page }) => {
    let refundCalled = false;

    await page.route("**/api/v1/admin/billing/refund", (route) => {
      refundCalled = true;
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ success: true }),
      });
    });

    await page.goto("/billing");
    await expect(page.getByTestId("billing-page")).toBeVisible();

    // Open refund dialog
    await page.getByTestId("refund-btn-inv-001").click();
    await expect(page.getByTestId("refund-dialog")).toBeVisible();

    // Confirm refund
    await page.getByTestId("refund-confirm").click();

    // Wait for API call
    await page.waitForTimeout(500);

    expect(refundCalled).toBe(true);
    // Dialog should close after successful refund
    await expect(page.getByTestId("refund-dialog")).not.toBeVisible();
  });

  test("dunning section shows failed payment tenants", async ({ page }) => {
    await page.goto("/billing");
    await expect(page.getByTestId("billing-page")).toBeVisible();

    const dunningSection = page.getByTestId("dunning-section");
    await expect(dunningSection).toBeVisible();
    await expect(dunningSection).toContainText("Failed Payments");

    // Both dunning entries should be visible
    await expect(page.getByTestId("dunning-t-initech")).toBeVisible();
    await expect(page.getByTestId("dunning-t-umbrella")).toBeVisible();

    // Verify retry count
    await expect(page.getByTestId("dunning-t-initech")).toContainText("3 retries");
    await expect(page.getByTestId("dunning-t-umbrella")).toContainText("1 retries");

    // Verify amounts
    await expect(page.getByTestId("dunning-t-initech")).toContainText("$2,400.00");
  });
});
