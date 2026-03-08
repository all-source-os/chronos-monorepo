import { test, expect } from "@playwright/test";

test.describe("Navigation — real stack", () => {
  test("sidebar links are navigable", async ({ page }) => {
    await page.goto("/tenants");
    await expect(page.getByTestId("tenants-page")).toBeVisible({
      timeout: 15_000,
    });

    // Navigate to monitoring via sidebar
    const monitoringLink = page.getByRole("link", { name: /monitoring/i });
    if (await monitoringLink.isVisible()) {
      await monitoringLink.click();
      await expect(page.getByTestId("monitoring-page")).toBeVisible({
        timeout: 15_000,
      });
    }

    // Navigate to billing
    const billingLink = page.getByRole("link", { name: /billing/i });
    if (await billingLink.isVisible()) {
      await billingLink.click();
      await expect(page.getByText(/revenue|billing/i).first()).toBeVisible({
        timeout: 15_000,
      });
    }

    // Navigate to security
    const securityLink = page.getByRole("link", { name: /security/i });
    if (await securityLink.isVisible()) {
      await securityLink.click();
      await expect(
        page.getByText(/security|ip rules/i).first()
      ).toBeVisible({ timeout: 15_000 });
    }
  });

  test("all pages are reachable without errors", async ({ page }) => {
    const routes = [
      "/tenants",
      "/monitoring",
      "/monitoring/alerts",
      "/monitoring/slos",
      "/billing",
      "/security",
      "/security/alerts",
    ];

    for (const route of routes) {
      const response = await page.goto(route);

      // Should not redirect to login (we have auth cookie)
      expect(page.url()).not.toContain("/login");

      // Should not get a server error
      expect(response?.status()).toBeLessThan(500);
    }
  });
});
