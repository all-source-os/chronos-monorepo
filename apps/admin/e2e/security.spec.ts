import { test, expect } from "@playwright/test";

/**
 * Security dashboard e2e tests.
 *
 * These tests mock the Control Plane API endpoints so the page can render
 * without a running backend.
 */

const mockIpRules = [
  {
    id: "rule-1",
    cidr: "10.0.0.0/8",
    rule_type: "allow",
    description: "Internal network",
    created_at: "2025-12-01T10:00:00Z",
  },
  {
    id: "rule-2",
    cidr: "192.168.1.0/24",
    rule_type: "deny",
    description: "Blocked subnet",
    created_at: "2025-12-05T14:30:00Z",
  },
];

const mockAuditEntries = [
  {
    id: "audit-1",
    tenant_id: "t-1",
    tenant_name: "Acme Corp",
    token_prefix: "sk-abc1",
    action: "events.ingest",
    ip_address: "10.0.1.50",
    timestamp: "2026-03-07T12:00:00Z",
  },
  {
    id: "audit-2",
    tenant_id: "t-2",
    tenant_name: "Globex Inc",
    token_prefix: "sk-def2",
    action: "events.query",
    ip_address: "172.16.0.10",
    timestamp: "2026-03-07T13:00:00Z",
  },
  {
    id: "audit-3",
    tenant_id: "t-1",
    tenant_name: "Acme Corp",
    token_prefix: "sk-abc1",
    action: "events.ingest",
    ip_address: "10.0.1.51",
    timestamp: "2026-03-07T14:00:00Z",
  },
];

const mockAuditSummary = [
  { tenant_id: "t-1", tenant_name: "Acme Corp", api_calls: 1420 },
  { tenant_id: "t-2", tenant_name: "Globex Inc", api_calls: 890 },
  { tenant_id: "t-3", tenant_name: "Initech", api_calls: 340 },
];

const mockPolicies = [
  {
    id: "policy-1",
    name: "Admin Full Access",
    description: "Full access to all resources",
    permissions: ["read", "write", "delete", "admin"],
    created_at: "2025-06-01T00:00:00Z",
  },
  {
    id: "policy-2",
    name: "Read Only",
    description: "Read-only access to events and projections",
    permissions: ["read"],
    created_at: "2025-06-15T00:00:00Z",
  },
];

test.describe("Security dashboard", () => {
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

    // Mock IP rules GET
    await page.route("**/api/v1/admin/security/ip-rules", (route) => {
      if (route.request().method() === "GET") {
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ rules: mockIpRules }),
        });
      }
      // POST: create a new rule
      if (route.request().method() === "POST") {
        const body = route.request().postDataJSON();
        const newRule = {
          id: `rule-${Date.now()}`,
          cidr: body.cidr,
          rule_type: body.rule_type,
          description: body.description,
          created_at: new Date().toISOString(),
        };
        mockIpRules.push(newRule);
        return route.fulfill({
          status: 201,
          contentType: "application/json",
          body: JSON.stringify(newRule),
        });
      }
      return route.continue();
    });

    // Mock IP rule DELETE
    await page.route("**/api/v1/admin/security/ip-rules/*", (route) => {
      if (route.request().method() === "DELETE") {
        const url = route.request().url();
        const ruleId = url.split("/").pop();
        const idx = mockIpRules.findIndex((r) => r.id === ruleId);
        if (idx !== -1) mockIpRules.splice(idx, 1);
        return route.fulfill({ status: 204 });
      }
      return route.continue();
    });

    // Mock token audit
    await page.route("**/api/v1/admin/security/token-audit?*", (route) => {
      const url = new URL(route.request().url());
      const tenantId = url.searchParams.get("tenant_id");
      let filtered = mockAuditEntries;
      if (tenantId) {
        filtered = mockAuditEntries.filter((e) => e.tenant_id === tenantId);
      }
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          entries: filtered,
          total: filtered.length,
          page: 1,
          per_page: 50,
        }),
      });
    });

    // Mock token audit summary
    await page.route("**/api/v1/admin/security/token-audit/summary", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ summary: mockAuditSummary }),
      })
    );

    // Mock policies
    await page.route("**/api/v1/policies", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ policies: mockPolicies }),
      })
    );
  });

  test("renders security page with tabs", async ({ page }) => {
    await page.goto("/security");
    await expect(page.getByTestId("security-page")).toBeVisible();
    await expect(page.getByTestId("security-tabs")).toBeVisible();

    // All three tab triggers should be visible
    await expect(page.getByTestId("tab-ip-rules")).toBeVisible();
    await expect(page.getByTestId("tab-token-audit")).toBeVisible();
    await expect(page.getByTestId("tab-rbac-policies")).toBeVisible();
  });

  test("IP Rules tab: displays rules in table", async ({ page }) => {
    await page.goto("/security");
    await expect(page.getByTestId("security-page")).toBeVisible();

    // IP Rules tab is active by default
    await expect(page.getByTestId("ip-rules-tab")).toBeVisible();
    await expect(page.getByTestId("ip-rules-table")).toBeVisible();

    // Verify rules render
    await expect(page.getByTestId("ip-rule-rule-1")).toBeVisible();
    await expect(page.getByTestId("ip-rule-rule-1")).toContainText("10.0.0.0/8");
    await expect(page.getByTestId("ip-rule-rule-1")).toContainText("allow");

    await expect(page.getByTestId("ip-rule-rule-2")).toBeVisible();
    await expect(page.getByTestId("ip-rule-rule-2")).toContainText("192.168.1.0/24");
    await expect(page.getByTestId("ip-rule-rule-2")).toContainText("deny");
  });

  test("IP Rules tab: add rule, verify in list, then delete", async ({
    page,
  }) => {
    await page.goto("/security");
    await expect(page.getByTestId("ip-rules-table")).toBeVisible();

    // Click Add Rule
    await page.getByTestId("add-ip-rule-btn").click();
    await expect(page.getByTestId("add-ip-rule-dialog")).toBeVisible();

    // Fill in the form
    await page.getByTestId("ip-rule-cidr-input").fill("203.0.113.0/24");
    await page.getByTestId("ip-rule-description-input").fill("Test block");

    // Toggle to deny (switch is on "allow" by default, click to toggle)
    await page.getByTestId("ip-rule-type-toggle").click();

    // Submit
    await page.getByTestId("ip-rule-submit-btn").click();

    // Dialog should close and the new rule should appear in the refreshed table
    await expect(page.getByTestId("add-ip-rule-dialog")).not.toBeVisible();

    // After reload, the mock returns updated mockIpRules which includes the new entry
    await expect(page.getByTestId("ip-rules-table")).toBeVisible();
    await expect(
      page.getByText("203.0.113.0/24")
    ).toBeVisible();

    // Now delete the first rule
    await page.getByTestId("delete-ip-rule-rule-1").click();
    await expect(page.getByTestId("delete-ip-rule-dialog")).toBeVisible();
    await expect(page.getByTestId("delete-ip-rule-dialog")).toContainText(
      "10.0.0.0/8"
    );

    // Confirm deletion
    await page.getByTestId("confirm-delete-btn").click();
    await expect(page.getByTestId("delete-ip-rule-dialog")).not.toBeVisible();

    // rule-1 should be gone after reload
    await expect(page.getByTestId("ip-rule-rule-1")).not.toBeVisible();
  });

  test("Token Audit tab: table loads and filters by tenant", async ({
    page,
  }) => {
    await page.goto("/security");
    await expect(page.getByTestId("security-page")).toBeVisible();

    // Switch to Token Audit tab
    await page.getByTestId("tab-token-audit").click();
    await expect(page.getByTestId("token-audit-tab")).toBeVisible();
    await expect(page.getByTestId("audit-table")).toBeVisible();

    // Verify entries render
    await expect(page.getByTestId("audit-entry-audit-1")).toBeVisible();
    await expect(page.getByTestId("audit-entry-audit-1")).toContainText(
      "Acme Corp"
    );
    await expect(page.getByTestId("audit-entry-audit-2")).toBeVisible();
    await expect(page.getByTestId("audit-entry-audit-2")).toContainText(
      "Globex Inc"
    );

    // Filter by tenant ID
    await page.getByTestId("audit-tenant-filter").fill("t-1");

    // Wait for reload
    await page.waitForTimeout(500);

    // After filtering, only Acme Corp entries should remain
    await expect(page.getByTestId("audit-entry-audit-1")).toBeVisible();
    await expect(page.getByTestId("audit-entry-audit-3")).toBeVisible();
    await expect(page.getByTestId("audit-entry-audit-2")).not.toBeVisible();
  });

  test("Token Audit tab: switches to chart view", async ({ page }) => {
    await page.goto("/security");
    await page.getByTestId("tab-token-audit").click();
    await expect(page.getByTestId("token-audit-tab")).toBeVisible();

    // Switch to chart view
    await page.getByTestId("audit-view-chart").click();
    await expect(page.getByTestId("audit-chart")).toBeVisible();
    await expect(page.getByTestId("audit-chart")).toContainText(
      "API Calls per Tenant"
    );

    // Recharts should render an SVG
    await expect(
      page.getByTestId("audit-chart").locator("svg.recharts-surface")
    ).toBeVisible();
  });

  test("RBAC Policies tab: lists policies read-only", async ({ page }) => {
    await page.goto("/security");
    await page.getByTestId("tab-rbac-policies").click();
    await expect(page.getByTestId("rbac-policies-tab")).toBeVisible();

    // Read-only badge
    await expect(page.getByTestId("rbac-policies-tab")).toContainText(
      "Read-only"
    );

    // Policies table
    await expect(page.getByTestId("policies-table")).toBeVisible();
    await expect(page.getByTestId("policy-policy-1")).toBeVisible();
    await expect(page.getByTestId("policy-policy-1")).toContainText(
      "Admin Full Access"
    );
    await expect(page.getByTestId("policy-policy-2")).toBeVisible();
    await expect(page.getByTestId("policy-policy-2")).toContainText(
      "Read Only"
    );

    // Permissions badges
    await expect(page.getByTestId("policy-policy-1")).toContainText("admin");
    await expect(page.getByTestId("policy-policy-2")).toContainText("read");
  });
});
