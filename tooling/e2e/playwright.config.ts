import { defineConfig, devices } from "@playwright/test";

/**
 * See https://playwright.dev/docs/test-configuration.
 *
 * Run against production/staging:
 *   BASE_URL=https://app.all-source.xyz CONTROL_PLANE_URL=https://cp.all-source.xyz \
 *     bunx playwright test tests/smoke/auth-staging.spec.ts
 *
 * Run locally (starts Next.js dev server automatically):
 *   bunx playwright test
 */

const isRemote = !!process.env.BASE_URL;

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [
    ["html", { outputFolder: "playwright-report" }],
    ["list"],
    ["json", { outputFile: "test-results.json" }],
  ],
  use: {
    baseURL: process.env.BASE_URL || "http://localhost:3000",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  // Only start the local dev server when running against localhost.
  // When BASE_URL points at a remote environment, skip this entirely.
  ...(isRemote
    ? {}
    : {
        webServer: {
          command: "cd ../../apps/web && bun run dev",
          url: "http://localhost:3000",
          reuseExistingServer: true,
          timeout: 60_000,
        },
      }),
});
