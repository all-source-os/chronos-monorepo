import { defineConfig, devices } from "@playwright/test";

/**
 * Read environment variables from file.
 * https://github.com/motdotla/dotenv
 */
// require('dotenv').config();

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  testDir: "./tests",
  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: [
    ["html", { outputFolder: "playwright-report" }],
    ["list"],
    ["json", { outputFile: "test-results.json" }],
  ],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: process.env.BASE_URL || "http://localhost:3000",

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: "on-first-retry",

    /* Screenshot on failure */
    screenshot: "only-on-failure",

    /* Video on failure */
    video: "retain-on-failure",
  },

  /* Configure projects for Chromium only (fast, focused test execution) */
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  /* Run Core API and web app before starting tests */
  webServer: [
    {
      // Core API service (Rust) - must start first
      command: "cd ../../apps/core && cargo build --release && ./target/release/allsource-core",
      url: "http://localhost:3900/health",
      reuseExistingServer: true,
      timeout: 180_000, // Rust build + startup
      stdout: "pipe",
      stderr: "pipe",
    },
    {
      // Web app (Next.js) - use dev server for faster iteration
      command: "cd ../../apps/web && bun run dev",
      url: "http://localhost:3000",
      reuseExistingServer: true,
      timeout: 60_000, // Dev server starts faster than build+start
    },
  ],
});
