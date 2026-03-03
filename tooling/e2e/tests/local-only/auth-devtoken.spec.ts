import { test, expect } from "@playwright/test";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
const baseURL = process.env.BASE_URL || "https://all-source.xyz";
const isLocal = baseURL.includes("localhost");

/**
 * Authentication E2E Tests
 *
 * Tests the full auth flow using the Query Service dev-token endpoint
 * (requires AUTH_DISABLED=true on the Query Service).
 *
 * NOTE: These tests require a local QS with AUTH_DISABLED=true.
 * Automatically skipped when running against production.
 */
test.describe("Authentication", () => {
  test.beforeEach(() => {
    test.skip(!isLocal, "Local-only tests — requires local QS with AUTH_DISABLED=true");
  });
  test("login page renders correctly", async ({ page }) => {
    await page.goto("/login");
    await expect(page.getByRole("heading", { name: /sign in/i })).toBeVisible();
    await expect(
      page.getByRole("button", { name: /google/i })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /github/i })
    ).toBeVisible();
  });

  test("shows error for invalid token callback", async ({ page }) => {
    await page.goto("/api/auth/callback?token=invalid-token-abc");
    // Should redirect to login with error
    await page.waitForURL(/\/login\?error=/);
    await expect(page.locator('[role="alert"]')).toBeVisible();
  });

  test("dev-token authenticates and redirects to dashboard", async ({
    page,
    request,
  }) => {
    // 1. Get a dev token from the Query Service
    const tokenResponse = await request.get(`${API_URL}/api/auth/dev-token`);
    expect(tokenResponse.ok()).toBeTruthy();

    const body = await tokenResponse.json();
    const token = body.data?.token;
    expect(token).toBeTruthy();

    // 2. Mock the /api/auth/me call so the callback handler accepts the token.
    //    The callback route calls getApiUrl()/api/auth/me server-side;
    //    we intercept the browser-level navigation and let the Next.js route
    //    handler talk to the real Query Service.
    //    Just navigate to the callback with the real token.
    await page.goto(`/api/auth/callback?token=${token}`);

    // 3. Should redirect to /dashboard (existing user) or /onboarding (new user)
    await page.waitForURL(/\/(dashboard|onboarding)/);

    // 4. Verify auth cookie was set
    const cookies = await page.context().cookies();
    const authCookie = cookies.find((c) => c.name === "auth_token");
    expect(authCookie).toBeDefined();
    expect(authCookie!.httpOnly).toBe(true);
    expect(authCookie!.path).toBe("/");
  });

  test("authenticated session returns user data", async ({
    page,
    request,
  }) => {
    // Get a dev token
    const tokenResponse = await request.get(`${API_URL}/api/auth/dev-token`);
    const body = await tokenResponse.json();
    const token = body.data?.token;

    // Authenticate via callback
    await page.goto(`/api/auth/callback?token=${token}`);
    await page.waitForURL(/\/(dashboard|onboarding)/);

    // Check session endpoint returns user data
    const sessionResponse = await page.request.get("/api/auth/session");
    expect(sessionResponse.ok()).toBeTruthy();

    const session = await sessionResponse.json();
    expect(session.data?.user).toBeDefined();
    expect(session.data?.user?.id).toBeTruthy();
  });

  test("logout clears session", async ({ page, request }) => {
    // Authenticate first
    const tokenResponse = await request.get(`${API_URL}/api/auth/dev-token`);
    const body = await tokenResponse.json();
    const token = body.data?.token;

    await page.goto(`/api/auth/callback?token=${token}`);
    await page.waitForURL(/\/(dashboard|onboarding)/);

    // Logout via session endpoint
    const logoutResponse = await page.request.delete("/api/auth/session");
    expect(logoutResponse.ok()).toBeTruthy();

    // Verify cookie is cleared
    const cookies = await page.context().cookies();
    const authCookie = cookies.find((c) => c.name === "auth_token");
    expect(authCookie).toBeUndefined();

    // Session should now return 401
    const sessionResponse = await page.request.get("/api/auth/session");
    expect(sessionResponse.status()).toBe(401);
  });
});
