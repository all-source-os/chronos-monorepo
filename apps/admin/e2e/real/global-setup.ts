/**
 * Playwright global setup for real-stack e2e tests.
 *
 * 1. Waits for Core and Control Plane to be healthy
 * 2. Seeds test data (tenant + events) via Core API
 * 3. Generates an admin JWT and writes storage state (cookie)
 */

import { generateAdminJwt } from "./jwt";
import path from "node:path";
import fs from "node:fs";

const CORE_URL = process.env.E2E_CORE_URL || "http://localhost:3090";
const CP_URL = process.env.E2E_CP_URL || "http://localhost:3091";
const ADMIN_URL = process.env.ADMIN_BASE_URL || "http://localhost:3011";
const STORAGE_STATE_PATH = path.join(__dirname, ".auth-state.json");

export const E2E_TENANT_ID = "e2e-test-tenant";

async function waitForHealth(
  url: string,
  name: string,
  maxAttempts = 30
): Promise<void> {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      const res = await fetch(url);
      if (res.ok) {
        console.log(`  ${name} healthy`);
        return;
      }
    } catch {
      // not ready yet
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  throw new Error(`${name} not healthy after ${maxAttempts}s at ${url}`);
}

async function seedTestData(): Promise<void> {
  // Create a test tenant in Core (dev mode, no auth needed)
  const tenantRes = await fetch(`${CORE_URL}/api/v1/tenants`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      id: E2E_TENANT_ID,
      name: "E2E Test Corp",
      slug: "e2e-test-corp",
      metadata: {
        subscription: { tier: "pro", status: "active" },
        quota: { events_quota: 100000 },
      },
    }),
  });

  if (tenantRes.ok || tenantRes.status === 409) {
    console.log("  Test tenant seeded (or already exists)");
  } else {
    console.warn(
      `  Warning: tenant seed returned ${tenantRes.status}: ${await tenantRes.text()}`
    );
  }

  // Seed a few events so the monitoring/queries pages have data
  const events = [
    {
      event_type: "user.signup",
      entity_id: "e2e-user-001",
      payload: { email: "test@e2e.local", plan: "pro" },
    },
    {
      event_type: "user.login",
      entity_id: "e2e-user-001",
      payload: { method: "oauth" },
    },
    {
      event_type: "order.created",
      entity_id: "e2e-order-001",
      payload: { amount: 99.99, currency: "USD" },
    },
  ];

  for (const event of events) {
    try {
      await fetch(`${CORE_URL}/api/v1/events`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(event),
      });
    } catch {
      // Non-fatal: tests can still verify UI even without events
    }
  }
  console.log("  Seeded test events");
}

function writeStorageState(): void {
  const token = generateAdminJwt({ tenant_id: E2E_TENANT_ID });
  const adminUrl = new URL(ADMIN_URL);

  const state = {
    cookies: [
      {
        name: "admin_token",
        value: token,
        domain: adminUrl.hostname,
        path: "/",
        expires: Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60,
        httpOnly: true,
        secure: false,
        sameSite: "Lax" as const,
      },
    ],
    origins: [],
  };

  fs.writeFileSync(STORAGE_STATE_PATH, JSON.stringify(state, null, 2));
  console.log("  Admin auth state written");
}

export default async function globalSetup(): Promise<void> {
  console.log("\n[e2e:real] Global setup...");

  // 1. Wait for services
  await waitForHealth(`${CORE_URL}/health`, "Core");
  await waitForHealth(`${CP_URL}/health`, "Control Plane");

  // 2. Seed data
  await seedTestData();

  // 3. Write auth cookie for Playwright
  writeStorageState();

  console.log("[e2e:real] Setup complete\n");
}

export { STORAGE_STATE_PATH };
