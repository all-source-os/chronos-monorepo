import { test as base, type APIRequestContext } from "@playwright/test";
import { demoLoginFull, type DemoAuth } from "./demo-auth";

/**
 * Worker-scoped fixtures for API-level integration tests.
 *
 * Provides an authenticated APIRequestContext against the Query Service
 * without requiring a browser. One demo login per worker for speed.
 *
 * Unlike the browser fixtures, these FAIL LOUDLY when CP/QS is unreachable —
 * no silent test.skip(). If the backend is down, you want to know.
 */

const QS_URL =
  process.env.QS_URL ||
  process.env.NEXT_PUBLIC_API_URL ||
  "https://allsource-query.fly.dev";

export const test = base.extend<
  {},
  { demoAuth: DemoAuth; apiContext: APIRequestContext }
>({
  demoAuth: [
    async ({ playwright }, use) => {
      const ctx = await playwright.request.newContext();
      const auth = await demoLoginFull(ctx);
      await ctx.dispose();
      if (!auth) {
        throw new Error(
          "Demo login failed — Control Plane unreachable. " +
            "Integration tests require a running backend."
        );
      }
      await use(auth);
    },
    { scope: "worker" },
  ],

  apiContext: [
    async ({ playwright, demoAuth }, use) => {
      const ctx = await playwright.request.newContext({
        baseURL: QS_URL,
        extraHTTPHeaders: {
          Authorization: `Bearer ${demoAuth.token}`,
          "Content-Type": "application/json",
        },
      });
      await use(ctx);
      await ctx.dispose();
    },
    { scope: "worker" },
  ],
});

export { expect } from "@playwright/test";
export type { DemoAuth };
