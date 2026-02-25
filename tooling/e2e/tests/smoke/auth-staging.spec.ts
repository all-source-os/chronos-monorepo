import { test, expect } from "@playwright/test";

const CP_URL =
  process.env.CONTROL_PLANE_URL || "http://localhost:3901";

/**
 * Auth E2E Tests — Staging / CI compatible
 *
 * These tests do NOT rely on AUTH_DISABLED or dev-token.
 * They use the demo-start endpoint which requires no credentials,
 * and optionally test real OAuth when E2E_OAUTH_EMAIL is set.
 */
test.describe("Authentication (staging)", () => {
  test("login page renders with OAuth buttons and demo option", async ({
    page,
  }) => {
    await page.goto("/login");
    await expect(
      page.getByRole("heading", { name: /welcome back/i })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /google/i })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /github/i })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /try demo/i })
    ).toBeVisible();
  });

  test("demo flow: start demo -> callback -> dashboard with demo banner", async ({
    page,
    request,
  }) => {
    // 1. Create a demo account via the control plane
    const demoResp = await request.post(`${CP_URL}/api/v1/demo/start`, {
      headers: { "Content-Type": "application/json" },
    });
    expect(demoResp.ok()).toBeTruthy();

    const demoData = await demoResp.json();
    expect(demoData.api_key).toBeTruthy();
    expect(demoData.is_demo).toBe(true);

    // 2. Authenticate via the callback handler (same as login page does)
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(demoData.api_key)}&new_user=true`
    );

    // 3. Should land on dashboard or onboarding
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    // 4. Verify auth cookie was set
    const cookies = await page.context().cookies();
    const authCookie = cookies.find((c) => c.name === "auth_token");
    expect(authCookie).toBeDefined();
    expect(authCookie!.httpOnly).toBe(true);

    // 5. Verify session endpoint works
    const sessionResp = await page.request.get("/api/auth/session");
    expect(sessionResp.ok()).toBeTruthy();
    const session = await sessionResp.json();
    expect(session.data?.user).toBeDefined();
  });

  test("demo flow: logout clears session", async ({ page, request }) => {
    // Authenticate via demo
    const demoResp = await request.post(`${CP_URL}/api/v1/demo/start`);
    const demoData = await demoResp.json();

    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(demoData.api_key)}`
    );
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });

    // Logout
    const logoutResp = await page.request.delete("/api/auth/session");
    expect(logoutResp.ok()).toBeTruthy();

    // Verify cookie cleared
    const cookies = await page.context().cookies();
    const authCookie = cookies.find((c) => c.name === "auth_token");
    expect(authCookie).toBeUndefined();

    // Session should now fail
    const sessionResp = await page.request.get("/api/auth/session");
    expect(sessionResp.status()).toBe(401);
  });

  test("shows error for invalid token callback", async ({ page }) => {
    await page.goto("/api/auth/callback?token=invalid-token-abc");
    await page.waitForURL(/\/login\?error=/);
    await expect(page.locator('[role="alert"]')).toBeVisible();
  });
});

/**
 * Real OAuth flow tests — only run when E2E_OAUTH_EMAIL is set.
 * These use Playwright to automate the full Google/GitHub OAuth flow.
 */
const oauthEmail = process.env.E2E_OAUTH_EMAIL;
const oauthPassword = process.env.E2E_OAUTH_PASSWORD;
const oauthProvider = process.env.E2E_OAUTH_PROVIDER || "google";

test.describe("OAuth flow (real credentials)", () => {
  test.skip(!oauthEmail || !oauthPassword, "E2E_OAUTH_EMAIL/PASSWORD not set");

  test("login via OAuth -> dashboard -> logout", async ({ page }) => {
    await page.goto("/login");

    // Click the OAuth provider button
    const providerButton = page.getByRole("button", {
      name: new RegExp(oauthProvider!, "i"),
    });
    await providerButton.click();

    // Wait for OAuth provider page to load
    await page.waitForURL(/accounts\.google\.com|github\.com\/login/, {
      timeout: 10000,
    });

    // Fill credentials on provider page
    if (oauthProvider === "google") {
      await page.fill('input[type="email"]', oauthEmail!);
      await page.click("#identifierNext");
      await page.waitForSelector('input[type="password"]', {
        state: "visible",
      });
      await page.fill('input[type="password"]', oauthPassword!);
      await page.click("#passwordNext");
    } else {
      // GitHub
      await page.fill("#login_field", oauthEmail!);
      await page.fill("#password", oauthPassword!);
      await page.click('input[name="commit"]');
    }

    // Wait for redirect back to dashboard
    await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 30000 });

    // Verify session
    const sessionResp = await page.request.get("/api/auth/session");
    expect(sessionResp.ok()).toBeTruthy();

    // Logout
    const logoutResp = await page.request.delete("/api/auth/session");
    expect(logoutResp.ok()).toBeTruthy();

    const sessionAfterLogout = await page.request.get("/api/auth/session");
    expect(sessionAfterLogout.status()).toBe(401);
  });
});
