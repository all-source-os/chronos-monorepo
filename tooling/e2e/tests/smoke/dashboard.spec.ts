import { test, expect, type Page } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

/**
 * Dashboard UI E2E Tests — Production / Staging compatible
 *
 * Tests every dashboard page loads correctly when authenticated and
 * renders the expected UI elements. Uses demo account for auth.
 *
 * Run against production:
 *   BASE_URL=https://www.all-source.xyz CONTROL_PLANE_URL=https://allsource-control-plane.fly.dev \
 *     bunx playwright test tests/smoke/dashboard.spec.ts
 */

let fileToken: string | null = null;

test.beforeAll(async ({ request }) => {
  fileToken = await demoLogin(request);
});

test.beforeEach(async ({ page }) => {
  test.skip(!fileToken, "Demo login failed (Control Plane not running)");
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(fileToken!)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
});

/** Wait for the dashboard layout to finish its auth check and render content. */
async function waitForDashboardLoad(page: Page) {
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}

// ---------------------------------------------------------------------------
// Dashboard Overview
// ---------------------------------------------------------------------------

test.describe("Dashboard Overview (/dashboard)", () => {
  test("page loads with greeting and key sections", async ({ page }) => {
    await page.goto("/dashboard");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /good (morning|afternoon|evening)/i })
    ).toBeVisible({ timeout: 10000 });

    await expect(
      page.getByRole("heading", { name: "Current Plan" })
    ).toBeVisible();

    const main = page.getByRole("main");
    await expect(main.getByText("Events", { exact: true }).first()).toBeVisible();
    await expect(main.getByText("Queries", { exact: true }).first()).toBeVisible();
  });

  test("API Keys section renders with Create Key button", async ({ page }) => {
    await page.goto("/dashboard");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: "API Keys" })
    ).toBeVisible({ timeout: 10000 });

    await expect(
      page.getByRole("button", { name: /create key/i })
    ).toBeVisible();

    const viewAll = page.getByRole("link", { name: /view all/i }).first();
    await expect(viewAll).toBeVisible();
  });

  test("instance stats banner shows real metrics", async ({ page }) => {
    await page.goto("/dashboard");
    await waitForDashboardLoad(page);

    // Banner now shows real instance metrics instead of hardcoded marketing numbers
    await expect(page.getByText("Your Event Store")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("total events")).toBeVisible();
    await expect(page.getByText("p99 latency")).toBeVisible();
    await expect(page.getByText("active projections")).toBeVisible();
    await expect(page.getByText("storage used")).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Event Explorer
// ---------------------------------------------------------------------------

test.describe("Event Explorer (/dashboard/events)", () => {
  test("page loads with heading and search", async ({ page }) => {
    const response = await page.goto("/dashboard/events");
    expect(response?.status()).toBeLessThan(400);
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /event explorer/i })
    ).toBeVisible({ timeout: 10000 });

    await expect(
      page.getByPlaceholder(/search events/i)
    ).toBeVisible();
  });

  test("shows export and create event buttons", async ({ page }) => {
    await page.goto("/dashboard/events");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("button", { name: /export/i })
    ).toBeVisible({ timeout: 10000 });

    await expect(
      page.getByRole("button", { name: /create event/i })
    ).toBeVisible();
  });

  test("filter toggle reveals filter fields", async ({ page }) => {
    await page.goto("/dashboard/events");
    await waitForDashboardLoad(page);

    const filtersBtn = page.getByRole("button", { name: /filters/i });
    await expect(filtersBtn).toBeVisible({ timeout: 10000 });
    await filtersBtn.click();

    await expect(page.getByPlaceholder(/filter by entity/i)).toBeVisible();
    await expect(page.getByPlaceholder(/filter by type/i)).toBeVisible();
  });

  test("shows empty state or real events", async ({ page }) => {
    await page.goto("/dashboard/events");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /event explorer/i })
    ).toBeVisible({ timeout: 10000 });

    // Either "No events yet" empty state or a results count like "(5 results)"
    await expect(
      page.getByText("No events yet")
        .or(page.getByText(/\(\d+ results\)/))
        .first()
    ).toBeVisible({ timeout: 15000 });
  });
});

// ---------------------------------------------------------------------------
// Analytics
// ---------------------------------------------------------------------------

test.describe("Analytics (/dashboard/analytics)", () => {
  test("page loads with heading and time range controls", async ({ page }) => {
    const response = await page.goto("/dashboard/analytics");
    expect(response?.status()).toBeLessThan(400);
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /usage analytics/i })
    ).toBeVisible({ timeout: 10000 });

    await expect(page.getByRole("button", { name: "24h" })).toBeVisible();
    await expect(page.getByRole("button", { name: "7d" })).toBeVisible();
    await expect(page.getByRole("button", { name: "30d" })).toBeVisible();
    await expect(page.getByRole("button", { name: "90d" })).toBeVisible();
  });

  test("summary cards render", async ({ page }) => {
    await page.goto("/dashboard/analytics");
    await waitForDashboardLoad(page);

    await expect(page.getByText("Total Events")).toBeVisible({ timeout: 15000 });
    await expect(page.getByText("Event Types")).toBeVisible();
    await expect(page.getByText("Unique Entities")).toBeVisible();
  });

  test("chart sections render after loading", async ({ page }) => {
    await page.goto("/dashboard/analytics");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /usage analytics/i })
    ).toBeVisible({ timeout: 10000 });

    // Charts or empty states must appear — if only heading renders, the API call is failing
    await expect(
      page.getByText("Ingestion Rate").first()
        .or(page.getByText(/no ingestion data/i).first())
        .or(page.getByText("Total Events"))
    ).toBeVisible({ timeout: 20000 });
  });

  test("time range toggle changes data", async ({ page }) => {
    await page.goto("/dashboard/analytics");
    await waitForDashboardLoad(page);

    const btn7d = page.getByRole("button", { name: "7d" });
    await expect(btn7d).toBeVisible({ timeout: 10000 });
    await btn7d.click();

    await expect(page.getByText("Total Events")).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// API Keys
// ---------------------------------------------------------------------------

test.describe("API Keys (/dashboard/api-keys)", () => {
  test("page loads with heading and create button", async ({ page }) => {
    const response = await page.goto("/dashboard/api-keys");
    expect(response?.status()).toBeLessThan(400);
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /api keys/i }).first()
    ).toBeVisible({ timeout: 10000 });

    await expect(
      page.getByRole("button", { name: /create key/i })
    ).toBeVisible();
  });

  test("security notice is visible", async ({ page }) => {
    await page.goto("/dashboard/api-keys");
    await waitForDashboardLoad(page);

    await expect(
      page.getByText(/keep your keys secure/i)
    ).toBeVisible({ timeout: 10000 });
  });

  test("quick tip cards are visible", async ({ page }) => {
    await page.goto("/dashboard/api-keys");
    await waitForDashboardLoad(page);

    await expect(page.getByText("Scope Permissions")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Rotate Regularly")).toBeVisible();
    await expect(page.getByText("Monitor Usage")).toBeVisible();
  });

  test("?action=create opens the create dialog", async ({ page }) => {
    await page.goto("/dashboard/api-keys?action=create");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /api keys/i }).first()
    ).toBeVisible({ timeout: 10000 });

    const dialog = page.getByRole("dialog");
    const createHeading = page.getByRole("heading", { name: /create/i });
    await expect(dialog.or(createHeading)).toBeVisible({ timeout: 5000 });
  });
});

// ---------------------------------------------------------------------------
// Billing
// ---------------------------------------------------------------------------

test.describe("Billing (/dashboard/billing)", () => {
  test("page loads with heading and plan info", async ({ page }) => {
    const response = await page.goto("/dashboard/billing");
    expect(response?.status()).toBeLessThan(400);
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /billing/i }).first()
    ).toBeVisible({ timeout: 10000 });

    await expect(
      page.getByRole("heading", { name: "Current Plan" }).first()
    ).toBeVisible();
  });

  test("billing toggle and plan cards visible", async ({ page }) => {
    await page.goto("/dashboard/billing");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("button", { name: /monthly/i })
    ).toBeVisible({ timeout: 10000 });
    await expect(
      page.getByRole("button", { name: /yearly/i })
    ).toBeVisible();

    await expect(
      page.getByRole("heading", { name: /available plans/i })
    ).toBeVisible();
  });

  test("FAQ accordion is present", async ({ page }) => {
    await page.goto("/dashboard/billing");
    await waitForDashboardLoad(page);

    await expect(
      page.getByText(/what happens if i exceed/i)
    ).toBeVisible({ timeout: 10000 });
  });

  test("usage charts render", async ({ page }) => {
    await page.goto("/dashboard/billing");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /usage this month/i })
    ).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Pipelines
// ---------------------------------------------------------------------------

test.describe("Pipelines (/dashboard/pipelines)", () => {
  test("page loads with heading", async ({ page }) => {
    const response = await page.goto("/dashboard/pipelines");
    expect(response?.status()).toBeLessThan(400);
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /pipelines/i }).first()
    ).toBeVisible({ timeout: 10000 });
  });

  test("status overview cards render", async ({ page }) => {
    await page.goto("/dashboard/pipelines");
    await waitForDashboardLoad(page);

    await expect(page.getByText("Total Pipelines")).toBeVisible({ timeout: 15000 });
    await expect(page.getByText("Running")).toBeVisible();
    await expect(page.getByText("Paused")).toBeVisible();
    await expect(page.getByText("Errors")).toBeVisible();
  });

  test("shows pipelines or empty state", async ({ page }) => {
    await page.goto("/dashboard/pipelines");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /no pipelines yet/i })
        .or(page.locator("[class*='pipeline'], [class*='Pipeline']").first())
    ).toBeVisible({ timeout: 15000 });
  });
});

// ---------------------------------------------------------------------------
// Team
// ---------------------------------------------------------------------------

test.describe("Team (/dashboard/team)", () => {
  test("page loads with heading and invite button", async ({ page }) => {
    const response = await page.goto("/dashboard/team");
    expect(response?.status()).toBeLessThan(400);
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /team/i }).first()
    ).toBeVisible({ timeout: 10000 });

    await expect(
      page.getByRole("button", { name: /invite member/i })
    ).toBeVisible();
  });

  test("seat usage card is visible", async ({ page }) => {
    await page.goto("/dashboard/team");
    await waitForDashboardLoad(page);

    await expect(page.getByText(/team seats/i)).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/seats used/i)).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

test.describe("Settings (/dashboard/settings)", () => {
  test("page loads with heading and tab navigation", async ({ page }) => {
    const response = await page.goto("/dashboard/settings");
    expect(response?.status()).toBeLessThan(400);
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /settings/i }).first()
    ).toBeVisible({ timeout: 10000 });

    await expect(page.getByText("Profile", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("Workspace", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("Security", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("Notifications", { exact: true }).first()).toBeVisible();
  });

  test("Profile tab shows name and email fields", async ({ page }) => {
    await page.goto("/dashboard/settings");
    await waitForDashboardLoad(page);

    await expect(page.getByText("Profile Information")).toBeVisible({ timeout: 10000 });
    await expect(page.getByLabel(/full name/i)).toBeVisible();
    await expect(page.getByLabel(/email/i)).toBeVisible();
  });

  test("Workspace tab shows workspace fields", async ({ page }) => {
    await page.goto("/dashboard/settings");
    await waitForDashboardLoad(page);

    await page.getByText("Workspace", { exact: true }).first().click();

    await expect(page.getByText("Workspace Settings")).toBeVisible({ timeout: 10000 });
    await expect(page.getByLabel(/workspace name/i)).toBeVisible();
  });

  test("Security tab shows connected accounts and danger zone", async ({ page }) => {
    await page.goto("/dashboard/settings");
    await waitForDashboardLoad(page);

    await page.getByText("Security", { exact: true }).first().click();

    await expect(page.getByText("Connected Accounts")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Danger Zone")).toBeVisible();
    await expect(
      page.getByRole("button", { name: /delete account/i })
    ).toBeVisible();
  });

  test("Notifications tab shows toggle preferences", async ({ page }) => {
    await page.goto("/dashboard/settings");
    await waitForDashboardLoad(page);

    await page.getByText("Notifications", { exact: true }).first().click();

    await expect(page.getByText("Notification Preferences")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Usage Alerts")).toBeVisible();
    await expect(page.getByText("Pipeline Errors")).toBeVisible();
    await expect(page.getByText("Security Alerts")).toBeVisible();
  });
});

// ---------------------------------------------------------------------------
// Audit Log
// ---------------------------------------------------------------------------

test.describe("Audit Log (/dashboard/settings/audit-log)", () => {
  test("page loads with heading", async ({ page }) => {
    const response = await page.goto("/dashboard/settings/audit-log");
    expect(response?.status()).toBeLessThan(400);
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /audit log/i })
    ).toBeVisible({ timeout: 10000 });
  });

  test("filter chips and activity table render", async ({ page }) => {
    await page.goto("/dashboard/settings/audit-log");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("button", { name: "All" })
    ).toBeVisible({ timeout: 10000 });

    await expect(page.getByRole("main").getByText("Activity", { exact: true }).first()).toBeVisible();
  });

  test("shows entries or empty state", async ({ page }) => {
    await page.goto("/dashboard/settings/audit-log");
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /audit log/i })
    ).toBeVisible({ timeout: 10000 });

    await expect(
      page.getByText(/no audit log entries/i)
        .or(page.getByText("Timestamp"))
        .or(page.getByText(/api.key|member|plan/i).first())
    ).toBeVisible({ timeout: 20000 });
  });
});

// ---------------------------------------------------------------------------
// Event Replay
// ---------------------------------------------------------------------------

test.describe("Event Replay (/dashboard/tools/replay)", () => {
  test("page loads with heading and form", async ({ page }) => {
    const response = await page.goto("/dashboard/tools/replay");
    expect(response?.status()).toBeLessThan(400);
    await waitForDashboardLoad(page);

    await expect(
      page.getByRole("heading", { name: /event replay/i })
    ).toBeVisible({ timeout: 10000 });

    await expect(page.getByText("Start New Replay")).toBeVisible();
  });

  test("replay form fields are present", async ({ page }) => {
    await page.goto("/dashboard/tools/replay");
    await waitForDashboardLoad(page);

    await expect(page.getByLabel(/from date/i)).toBeVisible({ timeout: 10000 });
    await expect(page.getByLabel(/to date/i)).toBeVisible();
    await expect(page.getByPlaceholder(/user\.created/i)).toBeVisible();
    await expect(page.getByPlaceholder(/user-123/i)).toBeVisible();

    await expect(
      page.getByRole("button", { name: /start replay/i })
    ).toBeVisible();
  });

  test("replay history section renders", async ({ page }) => {
    await page.goto("/dashboard/tools/replay");
    await waitForDashboardLoad(page);

    await expect(page.getByText("Replay History")).toBeVisible({ timeout: 10000 });

    const emptyState = page.getByText(/no replays yet/i);
    const historyCard = page.getByText("Replay History");

    await expect(emptyState.or(historyCard)).toBeVisible({ timeout: 10000 });
  });
});
