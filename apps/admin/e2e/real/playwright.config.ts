import { defineConfig, devices } from "@playwright/test";
import path from "node:path";

const STORAGE_STATE = path.join(__dirname, ".auth-state.json");

export default defineConfig({
  testDir: ".",
  testMatch: "*.spec.ts",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["html", { outputFolder: "../../playwright-report-real" }]],

  globalSetup: "./global-setup.ts",

  use: {
    baseURL: process.env.ADMIN_BASE_URL || "http://localhost:3011",
    storageState: STORAGE_STATE,
    trace: "on-first-retry",
    // Longer timeouts — real services may be slower than mocks
    actionTimeout: 10_000,
    navigationTimeout: 15_000,
  },

  expect: {
    timeout: 10_000,
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  // Start the admin app dev server if not already running
  webServer: process.env.CI
    ? undefined
    : {
        command: "NEXT_PUBLIC_API_URL=http://localhost:3091 bun run dev",
        port: 3011,
        reuseExistingServer: true,
        cwd: path.resolve(__dirname, "../.."),
        env: {
          NEXT_PUBLIC_API_URL: "http://localhost:3091",
          PORT: "3011",
        },
      },
});
