import { test, expect } from "@playwright/test";
import { readFileSync, existsSync, readdirSync, statSync } from "fs";
import path from "path";
import { fileURLToPath } from "url";

/**
 * Static Analysis — CORS & Proxy invariants
 *
 * These tests validate the codebase itself, not running services.
 * They catch regressions like:
 * - Dead code that makes direct cross-origin calls
 * - Module-level env var reads in route handlers
 * - Missing server-only guards on URL helpers
 * - WebSocket URL leaking backend addresses
 *
 * These run without any backend — safe to run in CI or locally.
 *
 * Run:
 *   cd tooling/e2e && bunx playwright test tests/integration/proxy-static-analysis.spec.ts
 */

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WEB_SRC = path.resolve(__dirname, "../../../../apps/web/src");

/** Recursively collect all .ts/.tsx files under a directory. */
function collectFiles(dir: string, ext: string[] = [".ts", ".tsx"]): string[] {
  const results: string[] = [];
  if (!existsSync(dir)) return results;
  for (const entry of readdirSync(dir)) {
    const full = path.join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      results.push(...collectFiles(full, ext));
    } else if (ext.some((e) => full.endsWith(e))) {
      results.push(full);
    }
  }
  return results;
}

// ─── Structural invariants ───────────────────────────────────────────────────

test.describe("Static — no direct backend URLs in client code", () => {
  test("no client component imports getServerApiUrl or getServerControlPlaneUrl", () => {
    // Client components (files with "use client" or in components/) must
    // never import server-only URL helpers.
    const componentFiles = collectFiles(path.join(WEB_SRC, "components"));
    const hookFiles = collectFiles(path.join(WEB_SRC, "hooks"));
    const clientFiles = [...componentFiles, ...hookFiles];

    for (const file of clientFiles) {
      const src = readFileSync(file, "utf-8");
      const rel = path.relative(WEB_SRC, file);

      expect(
        src.includes("getServerApiUrl"),
        `${rel} imports getServerApiUrl — client code must use relative URLs`
      ).toBe(false);

      expect(
        src.includes("getServerControlPlaneUrl"),
        `${rel} imports getServerControlPlaneUrl — client code must use relative URLs`
      ).toBe(false);
    }
  });

  test("no hardcoded backend URLs in client components", () => {
    const componentFiles = collectFiles(path.join(WEB_SRC, "components"));
    const hookFiles = collectFiles(path.join(WEB_SRC, "hooks"));
    const clientFiles = [...componentFiles, ...hookFiles];

    // Patterns that indicate direct backend calls
    const forbidden = [
      /localhost:3900\b/, // Core port
      /localhost:3901\b/, // Control Plane port
      /localhost:3902\b/, // Query Service port
      /allsource-query\.fly\.dev/,
      /allsource-core\.fly\.dev/,
    ];

    // use-phoenix-channel.ts has localhost:3902 as an SSR-only fallback (never
    // runs in browser — guarded by `typeof window !== "undefined"` check).
    const allowedFallbacks: Record<string, RegExp[]> = {
      "hooks/use-phoenix-channel.ts": [/localhost:3902\b/],
    };

    for (const file of clientFiles) {
      const src = readFileSync(file, "utf-8");
      const rel = path.relative(WEB_SRC, file);
      const allowed = allowedFallbacks[rel] || [];

      for (const pattern of forbidden) {
        if (allowed.some((a) => a.source === pattern.source)) continue;
        expect(
          pattern.test(src),
          `${rel} contains hardcoded backend URL matching ${pattern}`
        ).toBe(false);
      }
    }
  });

  test("dead event-store client directory does not exist", () => {
    const eventStoreDir = path.join(WEB_SRC, "lib", "event-store");
    expect(
      existsSync(eventStoreDir),
      "lib/event-store/ still exists — it makes direct cross-origin calls and must be deleted"
    ).toBe(false);
  });
});

// ─── Route handler invariants ────────────────────────────────────────────────

test.describe("Static — route handler correctness", () => {
  test("auth/session uses per-request env read, not module-level const", () => {
    const file = path.join(WEB_SRC, "app", "api", "auth", "session", "route.ts");
    expect(existsSync(file), "auth/session/route.ts missing").toBe(true);

    const src = readFileSync(file, "utf-8");

    // Must use a function for env var access
    expect(src).toMatch(/function\s+getApiUrl\(\)/);
    // Must NOT have a module-level const for the URL
    expect(src).not.toMatch(/^(?:export\s+)?const\s+API_URL\s*=/m);
  });

  test("v1 catch-all proxy forwards set-cookie headers", () => {
    const file = path.join(WEB_SRC, "app", "api", "v1", "[...path]", "route.ts");
    expect(existsSync(file), "v1/[...path]/route.ts missing").toBe(true);

    const src = readFileSync(file, "utf-8");

    expect(
      src.includes('response.headers.get("set-cookie")'),
      "Proxy must read set-cookie from backend response"
    ).toBe(true);
  });

  test("v1 catch-all proxy forwards body for DELETE requests", () => {
    const file = path.join(WEB_SRC, "app", "api", "v1", "[...path]", "route.ts");
    const src = readFileSync(file, "utf-8");

    // DELETE must be in the method list for body forwarding
    expect(src).toMatch(/"DELETE"/);
    // And it must be in the same condition that forwards the body
    const bodyForwardBlock = src.match(
      /if\s*\(\[.*?"DELETE".*?\]\.includes\(request\.method\).*?request\.body/s
    );
    expect(
      bodyForwardBlock,
      "DELETE must be included in body-forwarding methods"
    ).toBeTruthy();
  });

  test("auth callback imports getServerApiUrl from client lib", () => {
    const file = path.join(WEB_SRC, "app", "api", "auth", "callback", "route.ts");
    expect(existsSync(file), "auth/callback/route.ts missing").toBe(true);

    const src = readFileSync(file, "utf-8");

    expect(src).toContain("getServerApiUrl");
    expect(src).toContain("@/lib/api/client");
  });
});

// ─── WebSocket invariants ────────────────────────────────────────────────────

test.describe("Static — Phoenix Channel safety", () => {
  test("use-phoenix-channel.ts imports from phoenix package", () => {
    const file = path.join(WEB_SRC, "hooks", "use-phoenix-channel.ts");
    expect(existsSync(file), "use-phoenix-channel.ts missing").toBe(true);

    const src = readFileSync(file, "utf-8");

    expect(src).toContain('from "phoenix"');
    expect(src).toContain("NEXT_PUBLIC_WS_URL");
    expect(src).not.toContain("NEXT_PUBLIC_API_URL");
  });

  test("no remaining imports of use-websocket", () => {
    const allFiles = [
      ...collectFiles(path.join(WEB_SRC, "components")),
      ...collectFiles(path.join(WEB_SRC, "hooks")),
    ];

    for (const file of allFiles) {
      const src = readFileSync(file, "utf-8");
      const rel = path.relative(WEB_SRC, file);
      expect(
        src.includes("use-websocket"),
        `${rel} still imports use-websocket — migrate to use-phoenix-channel`
      ).toBe(false);
    }
  });
});

// ─── Server-only guard invariants ────────────────────────────────────────────

test.describe("Static — server-only guards", () => {
  test("client.ts has browser detection guard on getServerApiUrl", () => {
    const file = path.join(WEB_SRC, "lib", "api", "client.ts");
    const src = readFileSync(file, "utf-8");

    // Find the getServerApiUrl function and verify it checks for window
    const fnMatch = src.match(
      /function\s+getServerApiUrl[\s\S]*?^}/m
    );
    expect(fnMatch, "getServerApiUrl function not found").toBeTruthy();

    const fnBody = fnMatch![0];
    expect(
      fnBody.includes("window"),
      "getServerApiUrl must check for window to prevent client-side usage"
    ).toBe(true);
    expect(
      fnBody.includes("throw"),
      "getServerApiUrl must throw when called from browser"
    ).toBe(true);
  });

  test("ApiClient singleton uses empty baseUrl (relative URLs)", () => {
    const file = path.join(WEB_SRC, "lib", "api", "client.ts");
    const src = readFileSync(file, "utf-8");

    // The singleton must use the default constructor (empty baseUrl)
    expect(src).toMatch(/export\s+const\s+apiClient\s*=\s*new\s+ApiClient\(\)/);
  });
});
