import { defineConfig } from "@playwright/test";
import { readFileSync, existsSync } from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Load .env.production
const envPath = path.resolve(__dirname, ".env.production");
if (existsSync(envPath)) {
  for (const line of readFileSync(envPath, "utf-8").split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx === -1) continue;
    const key = trimmed.slice(0, eqIdx);
    const value = trimmed.slice(eqIdx + 1);
    if (!process.env[key]) process.env[key] = value;
  }
}

const baseURL = process.env.BASE_URL || "https://all-source.xyz";

export default defineConfig({
  globalSetup: "./global-setup.ts",
  testDir: "./tests",
  fullyParallel: false,
  retries: 0,
  workers: 1,
  timeout: 60_000,
  reporter: [
    ["list"],
    ["json", { outputFile: "test-results-lightpanda.json" }],
  ],
  use: {
    baseURL,
    trace: "off",
    screenshot: "only-on-failure",
    video: "off",
    // Connect to Lightpanda's CDP server
    connectOptions: {
      wsEndpoint: "ws://127.0.0.1:9222/",
    },
  },
  projects: [
    {
      name: "ui-lightpanda",
      testDir: "./tests/ui",
    },
  ],
});
