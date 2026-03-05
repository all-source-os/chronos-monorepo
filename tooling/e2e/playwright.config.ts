import { defineConfig, devices } from "@playwright/test";
import { readFileSync, existsSync } from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Load .env.production by default, .env.local for local dev.
 * Explicit env vars always take precedence.
 *
 * Run against production (default):
 *   cd tooling/e2e && bunx playwright test tests/smoke/auth.spec.ts
 *
 * Run locally:
 *   cp .env.local.example .env.local && bunx playwright test
 */
const envFile = process.env.E2E_ENV_FILE || ".env.production";
const envPath = path.resolve(__dirname, envFile);
if (existsSync(envPath)) {
  for (const line of readFileSync(envPath, "utf-8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx === -1) continue;
    const key = trimmed.slice(0, eqIdx);
    const value = trimmed.slice(eqIdx + 1);
    // Explicit env vars take precedence — don't overwrite
    if (!process.env[key]) process.env[key] = value;
  }
}

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
      name: "integration",
      testDir: "./tests/integration",
      // API-only tests — no browser needed
      use: {},
    },
    {
      name: "e2e",
      testDir: "./tests/e2e",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "smoke",
      testDir: "./tests/smoke",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "ui",
      testDir: "./tests/ui",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "ui-components",
      testDir: "./tests/ui-components",
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
