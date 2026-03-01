import { defineConfig, devices } from "@playwright/test";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * See https://playwright.dev/docs/test-configuration.
 *
 * Default target: https://all-source.xyz (production).
 * Override with BASE_URL env var for staging or local dev.
 *
 * Run against production (default):
 *   cd tooling/e2e && bunx playwright test tests/smoke/auth.spec.ts
 *
 * Run locally:
 *   BASE_URL=http://localhost:3000 bunx playwright test
 */

const baseURL = process.env.BASE_URL || "https://all-source.xyz";
const isLocal = baseURL.includes("localhost");

export default defineConfig({
  globalSetup: "./global-setup.ts",
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
    baseURL,
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
  ...(isLocal
    ? {
        webServer: {
          command: "cd ../../apps/web && bun run dev",
          url: baseURL,
          reuseExistingServer: true,
          timeout: 60_000,
        },
      }
    : {}),
});
