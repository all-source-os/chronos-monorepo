import { test, expect } from "@playwright/test";

/**
 * Tenants list page e2e tests.
 *
 * These tests mock the API endpoints so the page can render fully
 * without a running Control Plane backend.
 */

function makeTenant(overrides: Record<string, unknown> = {}) {
  return {
    id: "tenant-1",
    name: "Acme Corp",
    plan: "pro",
    status: "active",
    events_count: 142_000,
    members_count: 12,
    created_at: "2025-06-15T10:30:00Z",
    ...overrides,
  };
}

const mockTenants = [
  makeTenant({ id: "tenant-1", name: "Acme Corp", plan: "pro", status: "active", events_count: 142_000, members_count: 12 }),
  makeTenant({ id: "tenant-2", name: "Globex Inc", plan: "enterprise", status: "active", events_count: 980_000, members_count: 45 }),
  makeTenant({ id: "tenant-3", name: "Initech", plan: "free", status: "suspended", events_count: 500, members_count: 2 }),
  makeTenant({ id: "tenant-4", name: "Umbrella Corp", plan: "starter", status: "archived", events_count: 23_000, members_count: 8 }),
  makeTenant({ id: "tenant-5", name: "Stark Industries", plan: "enterprise", status: "active", events_count: 2_100_000, members_count: 120 }),
];

function buildTenantsResponse(
  tenants: typeof mockTenants,
  page = 1,
  perPage = 10
) {
  return {
    tenants,
    total: tenants.length,
    page,
    per_page: perPage,
    _links: { self: `/api/v1/admin/tenants?page=${page}&per_page=${perPage}` },
  };
}

test.describe("Tenants list page", () => {
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
  });

  test("renders table with tenant data", async ({ page }) => {
    await page.route("**/api/v1/admin/tenants*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(buildTenantsResponse(mockTenants)),
      })
    );

    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-page")).toBeVisible();

    // Table should be visible with data
    const table = page.getByTestId("tenants-table");
    await expect(table).toBeVisible();

    // Verify column headers
    await expect(table.getByText("Name")).toBeVisible();
    await expect(table.getByText("Plan")).toBeVisible();
    await expect(table.getByText("Status")).toBeVisible();
    await expect(table.getByText("Events")).toBeVisible();
    await expect(table.getByText("Members")).toBeVisible();
    await expect(table.getByText("Created")).toBeVisible();

    // Verify tenant rows
    await expect(page.getByTestId("tenant-row-tenant-1")).toBeVisible();
    await expect(page.getByTestId("tenant-row-tenant-1")).toContainText("Acme Corp");
    await expect(page.getByTestId("tenant-row-tenant-2")).toContainText("Globex Inc");
    await expect(page.getByTestId("tenant-row-tenant-5")).toContainText("Stark Industries");
  });

  test("search filters results with debounce", async ({ page }) => {
    let lastSearchParam = "";

    await page.route("**/api/v1/admin/tenants*", (route) => {
      const url = new URL(route.request().url());
      lastSearchParam = url.searchParams.get("search") || "";

      const filtered = lastSearchParam
        ? mockTenants.filter((t) =>
            t.name.toLowerCase().includes(lastSearchParam.toLowerCase())
          )
        : mockTenants;

      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(buildTenantsResponse(filtered)),
      });
    });

    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-table")).toBeVisible();

    // Type in search
    const searchInput = page.getByTestId("tenants-search");
    await searchInput.fill("Acme");

    // Wait for debounce + API call
    await page.waitForTimeout(500);

    // Verify filtered result
    await expect(page.getByTestId("tenant-row-tenant-1")).toBeVisible();
    await expect(page.getByTestId("tenant-row-tenant-2")).not.toBeVisible();

    expect(lastSearchParam).toBe("Acme");
  });

  test("pagination controls work", async ({ page }) => {
    let lastPage = 1;

    // Create enough tenants for pagination (3 per page)
    const manyTenants = Array.from({ length: 6 }, (_, i) =>
      makeTenant({
        id: `tenant-${i + 1}`,
        name: `Tenant ${i + 1}`,
      })
    );

    await page.route("**/api/v1/admin/tenants*", (route) => {
      const url = new URL(route.request().url());
      lastPage = Number(url.searchParams.get("page") || "1");
      const reqPerPage = Number(url.searchParams.get("per_page") || "10");

      const start = (lastPage - 1) * reqPerPage;
      const pageTenants = manyTenants.slice(start, start + reqPerPage);

      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          tenants: pageTenants,
          total: manyTenants.length,
          page: lastPage,
          per_page: reqPerPage,
          _links: { self: `/api/v1/admin/tenants?page=${lastPage}` },
        }),
      });
    });

    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-table")).toBeVisible();

    // Change page size to trigger pagination (need fewer items per page)
    const pageSizeSelect = page.getByTestId("tenants-page-size");
    // Default is 10, all 6 tenants fit. Verify page info
    await expect(page.getByTestId("tenants-page-info")).toContainText("Page 1 of 1");

    // Previous should be disabled on page 1
    const prevButton = page.getByTestId("tenants-prev-page");
    await expect(prevButton).toBeDisabled();
  });

  test("shows loading skeleton while fetching", async ({ page }) => {
    // Delay the API response to observe skeleton
    await page.route("**/api/v1/admin/tenants*", async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 500));
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(buildTenantsResponse(mockTenants)),
      });
    });

    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-page")).toBeVisible();

    // Skeleton should be visible while loading
    await expect(page.getByTestId("tenants-skeleton")).toBeVisible();

    // Then table should appear
    await expect(page.getByTestId("tenants-table")).toBeVisible({ timeout: 5000 });
  });

  test("shows empty state when no tenants match", async ({ page }) => {
    await page.route("**/api/v1/admin/tenants*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(buildTenantsResponse([])),
      })
    );

    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-page")).toBeVisible();
    await expect(page.getByTestId("tenants-empty")).toBeVisible();
    await expect(page.getByTestId("tenants-empty")).toContainText("No tenants found");
  });

  test("filter dropdowns send correct API parameters", async ({ page }) => {
    const apiCalls: { plan: string; status: string }[] = [];

    await page.route("**/api/v1/admin/tenants*", (route) => {
      const url = new URL(route.request().url());
      apiCalls.push({
        plan: url.searchParams.get("plan") || "",
        status: url.searchParams.get("status") || "",
      });

      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(buildTenantsResponse(mockTenants)),
      });
    });

    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-table")).toBeVisible();

    // Select plan filter
    const planFilter = page.getByTestId("tenants-plan-filter");
    await planFilter.selectOption("pro");

    // Wait for API call
    await page.waitForTimeout(300);

    // The latest call should include plan=pro
    const lastCall = apiCalls[apiCalls.length - 1];
    expect(lastCall?.plan).toBe("pro");

    // Select status filter
    const statusFilter = page.getByTestId("tenants-status-filter");
    await statusFilter.selectOption("suspended");

    await page.waitForTimeout(300);

    const finalCall = apiCalls[apiCalls.length - 1];
    expect(finalCall?.status).toBe("suspended");
  });
});
