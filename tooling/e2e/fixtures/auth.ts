import { test as base, expect } from "@playwright/test";
import path from "path";
import fs from "fs";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const STORAGE_STATE_PATH = path.join(
  __dirname,
  "..",
  "test-results",
  ".auth-storage-state.json"
);

/**
 * Logs in via the email/password form at /login and saves the browser
 * storage state so subsequent tests can reuse the session without
 * logging in again.
 */
async function loginAndSaveState(
  page: import("@playwright/test").Page,
  email: string,
  password: string
) {
  await page.goto("/login");

  // Expand the email login form
  await page.getByRole("button", { name: /email/i }).click();

  // Fill credentials
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', password);

  // Submit
  await page.getByRole("button", { name: /sign in/i }).click();

  // Wait for redirect to dashboard or onboarding
  await page.waitForURL(/\/(dashboard|onboarding)/, { timeout: 15_000 });

  // Ensure test-results directory exists for storage state file
  const dir = path.dirname(STORAGE_STATE_PATH);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }

  // Persist the authenticated state
  await page.context().storageState({ path: STORAGE_STATE_PATH });
}

type AuthFixtures = {
  authenticatedPage: import("@playwright/test").Page;
};

/**
 * Reusable fixture that provides an already-authenticated Page.
 *
 * On first use it logs in via email/password and persists the storage
 * state. Subsequent tests in the same worker reuse the saved state,
 * skipping the login form entirely.
 *
 * Requires TEST_EMAIL and TEST_PASSWORD environment variables (or .env.test).
 */
export const test = base.extend<AuthFixtures>({
  authenticatedPage: async ({ browser }, use) => {
    const email = process.env.TEST_EMAIL;
    const password = process.env.TEST_PASSWORD;

    if (!email || !password) {
      throw new Error(
        "TEST_EMAIL and TEST_PASSWORD must be set. " +
          "Copy .env.test and fill in valid credentials."
      );
    }

    // Reuse existing storage state if available
    if (fs.existsSync(STORAGE_STATE_PATH)) {
      const context = await browser.newContext({
        storageState: STORAGE_STATE_PATH,
      });
      const page = await context.newPage();

      // Quick check: is the session still valid?
      const resp = await page.request.get("/api/auth/session");
      if (resp.ok()) {
        await use(page);
        await context.close();
        return;
      }

      // Session expired — fall through and re-login
      await context.close();
    }

    // Fresh login
    const context = await browser.newContext();
    const page = await context.newPage();
    await loginAndSaveState(page, email, password);
    await use(page);
    await context.close();
  },
});

export { expect } from "@playwright/test";
