import { test, expect } from "@playwright/test";
import {
  demoLoginFull,
  authenticateAndGoToDashboard,
} from "../../fixtures/demo-auth";

/**
 * Login → Dashboard — Full browser journey
 *
 * Demo login → lands on dashboard → real data visible.
 * Intercepts network responses to verify real user data.
 */

test.describe("Login to Dashboard", () => {
  let token: string;
  let email: string;

  test.beforeAll(async ({ request }) => {
    const auth = await demoLoginFull(request);
    if (!auth) {
      throw new Error(
        "Demo login failed — Control Plane unreachable. " +
          "E2E tests require a running backend."
      );
    }
    token = auth.token;
    email = auth.email;
  });

  test("demo login lands on dashboard with greeting visible", async ({
    page,
  }) => {
    await authenticateAndGoToDashboard(page, token);

    await expect(
      page.getByRole("heading", {
        name: /good (morning|afternoon|evening)/i,
      })
    ).toBeVisible({ timeout: 10000 });
  });

  test("session endpoint returns real user data", async ({ page }) => {
    await authenticateAndGoToDashboard(page, token);

    // Intercept the session call the dashboard makes
    const sessionResp = await page.request.get("/api/auth/session");
    expect(sessionResp.ok()).toBeTruthy();

    const session = await sessionResp.json();
    expect(session.data).toBeDefined();
    expect(session.data.user).toBeDefined();
    // Real user data — not a mock
    expect(session.data.user.email).toBe(email);
  });

  test("stats banner shows non-error state", async ({ page }) => {
    await authenticateAndGoToDashboard(page, token);

    // The instance stats banner should NOT show "failed to load" or error states
    const banner = page.getByText(/Your Event Store|Instance Stats/i).first();
    await expect(banner).toBeVisible({ timeout: 15000 });

    // Should not have error indicators
    const errorText = page.getByText(/failed to load|error loading/i).first();
    await expect(errorText).not.toBeVisible({ timeout: 3000 }).catch(() => {
      // If error IS visible, fail with a clear message
      throw new Error(
        "Dashboard stats banner shows an error state — backend data fetch failed"
      );
    });
  });
});
