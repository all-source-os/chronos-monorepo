/**
 * Visual-proof capture for the Fleet P2 build. NOT a CI assertion spec — it
 * drives the real authenticated pages and saves screenshots of (1) the fleet
 * overview and (2) a Destructive recovery dialog mid-confirmation (preview shown
 * + typed id matched, Apply enabled). Run with:
 *   bunx playwright test fleet-proof.spec.ts
 * Artifacts land in apps/admin/proofshot-artifacts/playwright/.
 */
import { expect, test } from "@playwright/test";

const TENANT_ID = "tenant-acme";
const OUT = "proofshot-artifacts/playwright";

const mockFleetHealth = {
  summary: { total: 10, healthy: 6, degraded: 2, at_risk: 1, critical: 1 },
  worst: [
    {
      tenant_id: TENANT_ID,
      name: "Acme Corp",
      tier: "critical",
      reasons: [
        { signal: "subscription_state", value: "canceled", tier: "critical" },
        { signal: "events_quota_pct", value: "118%", tier: "at_risk" },
      ],
    },
    {
      tenant_id: "tenant-globex",
      name: "Globex",
      tier: "at_risk",
      reasons: [{ signal: "last_sync_age", value: "31h", tier: "at_risk" }],
    },
    {
      tenant_id: "tenant-umbrella",
      name: "Umbrella",
      tier: "degraded",
      reasons: [{ signal: "subscription_state", value: "past_due", tier: "degraded" }],
    },
  ],
};

const mockTenantHealth = {
  tenant_id: TENANT_ID,
  name: "Acme Corp",
  tier: "critical",
  signals: [
    {
      signal: "subscription_state",
      value: "canceled",
      tier: "critical",
      source: "Core tenant metadata",
    },
    { signal: "events_quota_pct", value: "118%", tier: "at_risk", source: "Core tenant metadata" },
    { signal: "last_event_age", value: "2h", tier: "healthy", source: "Core (desc probe)" },
    { signal: "last_sync_age", value: "31h", tier: "at_risk", source: "Core (sync probe)" },
  ],
  subscription: {
    tier: "pro",
    status: "canceled",
    renews_at: "2026-05-01T00:00:00Z",
    grandfathered: false,
    events_used: 590_000,
    events_quota: 500_000,
  },
};

function adminJwt(): string {
  const b64 = (o: unknown) => Buffer.from(JSON.stringify(o)).toString("base64url");
  const now = Math.floor(Date.now() / 1000);
  return [
    b64({ alg: "HS256", typ: "JWT" }),
    b64({
      sub: "admin-1",
      email: "admin@allsource.co",
      name: "Admin",
      role: "admin",
      iat: now,
      exp: now + 86400,
    }),
    "test-signature",
  ].join(".");
}

test.describe("Fleet visual proof", () => {
  test.beforeEach(async ({ context, page }) => {
    await context.addCookies([
      {
        name: "admin_token",
        value: adminJwt(),
        domain: "localhost",
        path: "/",
        httpOnly: true,
        sameSite: "Lax",
      },
    ]);
    await page.route("**/api/auth/session", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          data: { user: { id: "admin-1", email: "admin@allsource.co", role: "admin" } },
        }),
      })
    );
  });

  test("overview screenshot", async ({ page }) => {
    await page.route("**/api/v1/admin/fleet/health?*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockFleetHealth),
      })
    );
    await page.goto("/fleet");
    await expect(page.getByTestId("fleet-worst-table")).toBeVisible();
    await page.screenshot({ path: `${OUT}/01-fleet-overview.png`, fullPage: true });
  });

  test("destructive dialog mid-confirmation screenshot", async ({ page }) => {
    await page.route(`**/api/v1/admin/fleet/health/${TENANT_ID}`, (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(mockTenantHealth),
      })
    );
    await page.route(`**/api/v1/admin/recovery/${TENANT_ID}/reprovision`, (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          dry_run: true,
          would: {
            metadata_diff: {
              status: "active → reprovisioned",
              quota_events: "500000 (re-applied)",
            },
          },
        }),
      })
    );

    await page.goto(`/fleet/${TENANT_ID}`);
    await expect(page.getByTestId("fleet-tenant-page")).toBeVisible();
    await page.screenshot({ path: `${OUT}/02-fleet-drilldown.png`, fullPage: true });

    // Open the Destructive Reprovision dialog, preview, type the matching id.
    await page.getByTestId("recovery-action-reprovision").click();
    await expect(page.getByTestId("recovery-dialog")).toBeVisible();
    await page.getByTestId("recovery-preview-btn").click();
    await expect(page.getByTestId("recovery-would-payload")).toBeVisible();
    await page.getByTestId("recovery-confirm-input").fill(TENANT_ID);
    await expect(page.getByTestId("recovery-apply-btn")).toBeEnabled();
    await page.screenshot({ path: `${OUT}/03-destructive-dialog-mid-confirmation.png` });
  });
});
