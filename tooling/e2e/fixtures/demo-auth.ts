import { expect, type Page, type APIRequestContext } from "@playwright/test";

const CP_URL =
  process.env.CONTROL_PLANE_URL || "http://localhost:3901";

export interface DemoAuth {
  token: string;
  email: string;
}

/**
 * Starts a demo session via the Control Plane and logs in.
 * Returns the JWT token string, or null if the CP is unreachable.
 *
 * Wraps the HTTP calls in try-catch so tests can `test.skip(!token)`
 * instead of crashing with ECONNREFUSED when the stack isn't running.
 */
export async function demoLogin(
  request: APIRequestContext | any
): Promise<string | null> {
  try {
    const demoResp = await request.post(`${CP_URL}/api/v1/demo/start`, {
      headers: { "Content-Type": "application/json" },
    });
    if (!demoResp.ok()) return null;
    const demoData = await demoResp.json();

    const loginResp = await request.post(`${CP_URL}/api/v1/auth/login`, {
      headers: { "Content-Type": "application/json" },
      data: { email: demoData.email, password: demoData.password },
    });
    if (!loginResp.ok()) return null;
    const loginData = await loginResp.json();
    return loginData.token as string;
  } catch {
    // ECONNREFUSED, timeout, etc. — return null so tests skip gracefully
    return null;
  }
}

/**
 * Like demoLogin but returns { token, email } for tests that need the email.
 */
export async function demoLoginFull(
  request: APIRequestContext
): Promise<DemoAuth | null> {
  try {
    const demoResp = await request.post(`${CP_URL}/api/v1/demo/start`, {
      headers: { "Content-Type": "application/json" },
    });
    if (!demoResp.ok()) return null;
    const demoData = await demoResp.json();

    const loginResp = await request.post(`${CP_URL}/api/v1/auth/login`, {
      headers: { "Content-Type": "application/json" },
      data: { email: demoData.email, password: demoData.password },
    });
    if (!loginResp.ok()) return null;
    const loginData = await loginResp.json();
    return { token: loginData.token, email: demoData.email };
  } catch {
    return null;
  }
}

/**
 * Authenticates via the token callback and navigates to /dashboard.
 */
export async function authenticateAndGoToDashboard(
  page: Page,
  token: string
): Promise<void> {
  await page.goto(
    `/api/auth/callback?token=${encodeURIComponent(token)}&new_user=false`
  );
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15000 });
  if (!page.url().includes("/dashboard")) {
    await page.goto("/dashboard");
  }
  await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });
}
