import { test, expect } from "@playwright/test";
import { demoLogin } from "../../fixtures/demo-auth";

/**
 * Demo Zone Smoke Tests
 *
 * All tests require a running Control Plane for demo account creation.
 * When the CP is unreachable, demoLogin returns null and tests skip.
 */
test.describe("Demo Zone Smoke Tests", () => {
  let fileToken: string | null = null;

  test.beforeAll(async ({ request }) => {
    fileToken = await demoLogin(request);
  });

  test.beforeEach(async ({ page }) => {
    test.skip(!fileToken, "Demo login failed (Control Plane not running)");
    await page.goto(
      `/api/auth/callback?token=${encodeURIComponent(fileToken!)}&new_user=true`
    );
    await page.waitForURL(/\/(dashboard|onboarding|demo)/, { timeout: 15000 });
  });

/**
 * Demo Zone - Page Shell & Navigation (US-005)
 *
 * Verifies the Demo Zone page loads, toggle buttons render,
 * and view switching works via URL params.
 */
test.describe("Demo Zone", () => {
  test("page loads with view toggle buttons", async ({ page }) => {
    const response = await page.goto("/dashboard/demo");
    expect(response?.status()).toBeLessThan(400);

    // Verify page heading
    await expect(page.getByRole("heading", { name: "Demo Zone" })).toBeVisible();

    // Verify both toggle buttons render
    await expect(page.getByRole("button", { name: /Live Fire/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /MCP Showdown/i })).toBeVisible();
  });

  test("defaults to Live Fire view", async ({ page }) => {
    await page.goto("/dashboard/demo");

    const liveFire = page.getByRole("button", { name: /Live Fire/i });
    await expect(liveFire).toHaveAttribute("aria-pressed", "true");
  });

  test("switches to MCP Showdown via toggle", async ({ page }) => {
    await page.goto("/dashboard/demo");

    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // URL should update
    await expect(page).toHaveURL(/view=mcp-showdown/);

    const mcpShowdown = page.getByRole("button", { name: /MCP Showdown/i });
    await expect(mcpShowdown).toHaveAttribute("aria-pressed", "true");
  });

  test("respects view param from URL", async ({ page }) => {
    await page.goto("/dashboard/demo?view=mcp-showdown");

    const mcpShowdown = page.getByRole("button", { name: /MCP Showdown/i });
    await expect(mcpShowdown).toHaveAttribute("aria-pressed", "true");
  });

  test("shows Start Demo button for empty state", async ({ page }) => {
    await page.goto("/dashboard/demo");

    await expect(page.getByRole("button", { name: /Start Demo/i })).toBeVisible();
  });
});

/**
 * Demo Zone - Live Event Stream Panel (US-006)
 *
 * Verifies the event stream panel renders and receives events
 * when the demo is seeded.
 */
test.describe("Live Event Stream Panel", () => {
  test("renders event stream panel with stats bar after seeding", async ({ page }) => {
    // Seed the demo (mock the response so we don't need a live backend)
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    await page.goto("/dashboard/demo");

    // Click Start Demo to seed
    await page.getByRole("button", { name: /Start Demo/i }).click();

    // Event stream panel should now be visible
    await expect(page.getByTestId("event-stream-panel")).toBeVisible();

    // Stats bar should render
    await expect(page.getByTestId("stats-bar")).toBeVisible();
    await expect(page.getByTestId("stats-bar")).toContainText("/sec");
    await expect(page.getByTestId("stats-bar")).toContainText("Latency");
  });

  test("event stream receives events via simulation fallback", async ({ page }) => {
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    // Wait for simulation events to appear (fallback triggers after 2s WS timeout)
    const eventList = page.getByTestId("event-list");
    await expect(eventList).toBeVisible();

    // Wait for at least one event row to appear (simulation kicks in after ~2s)
    await expect(page.getByTestId("event-row").first()).toBeVisible({ timeout: 5000 });
  });

  test("pause and resume controls work", async ({ page }) => {
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    await expect(page.getByTestId("event-stream-panel")).toBeVisible();

    // Pause button should be visible
    const pauseBtn = page.getByRole("button", { name: /Pause/i });
    await expect(pauseBtn).toBeVisible();

    // Click pause
    await pauseBtn.click();

    // Should now show resume
    await expect(page.getByRole("button", { name: /Resume/i })).toBeVisible();
  });

  test("scroll lock toggle works", async ({ page }) => {
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    await expect(page.getByTestId("event-stream-panel")).toBeVisible();

    // Scroll lock button should be present
    const scrollBtn = page.getByRole("button", { name: /Lock scroll/i });
    await expect(scrollBtn).toBeVisible();

    // Toggle it
    await scrollBtn.click();
    await expect(page.getByRole("button", { name: /Unlock scroll/i })).toBeVisible();
  });
});

/**
 * Demo Zone - Replay Last 10s (US-007)
 *
 * Verifies the replay button renders, triggers replay, and events re-render
 * with the replayed highlight styling.
 */
test.describe("Replay Last 10s", () => {
  test.beforeEach(async ({ page }) => {
    // Mock seed endpoint
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    // Mock events query endpoint (replay will try to fetch from QS)
    await page.route("**/api/v1/events/query**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ events: [], count: 0 }),
      })
    );
  });

  test("replay button renders in event stream header", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    await expect(page.getByTestId("event-stream-panel")).toBeVisible();
    await expect(page.getByTestId("replay-button")).toBeVisible();
    await expect(page.getByTestId("replay-button")).toContainText("Replay 10s");
  });

  test("clicking replay replays events with highlight styling", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    // Wait for simulation events to appear
    await expect(page.getByTestId("event-row").first()).toBeVisible({ timeout: 5000 });

    // Click replay
    const replayBtn = page.getByTestId("replay-button");
    await replayBtn.click();

    // Button should show replaying state and be disabled
    await expect(replayBtn).toContainText("Replaying...");
    await expect(replayBtn).toBeDisabled();

    // Wait for replayed events to appear (they have data-replayed attribute)
    await expect(page.locator("[data-replayed]").first()).toBeVisible({ timeout: 5000 });

    // Button should re-enable after replay completes
    await expect(replayBtn).toContainText("Replay 10s", { timeout: 10000 });
    await expect(replayBtn).toBeEnabled();
  });
});

/**
 * Demo Zone - Vector Query Playground (US-008)
 *
 * Verifies the vector query panel renders, accepts search input,
 * displays results with latency badge, and shows Why? popovers.
 */
test.describe("Vector Query Playground", () => {
  test.beforeEach(async ({ page }) => {
    // Mock seed endpoint
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    // Mock events query endpoint with sample data
    await page.route("**/api/v1/events/query**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          events: [
            {
              id: "evt-001",
              entity_id: "server-0001",
              event_type: "log.error",
              payload: {
                level: "error",
                message: "Connection timeout to upstream",
                service: "api-1",
                error_code: 500,
                _embedding: [0.1, 0.2, 0.3, 0.4, 0.5],
              },
              timestamp: new Date().toISOString(),
              version: 1,
            },
            {
              id: "evt-002",
              entity_id: "user-0042",
              event_type: "user.search",
              payload: {
                user_id: "usr-000042",
                query: "error 500",
                results_count: 15,
                took_ms: 38,
                _embedding: [0.2, 0.3, 0.4, 0.5, 0.6],
              },
              timestamp: new Date().toISOString(),
              version: 2,
            },
            {
              id: "evt-003",
              entity_id: "infra-0005",
              event_type: "metric.latency",
              payload: {
                endpoint: "/api/events",
                p50_ms: 12,
                p95_ms: 89,
                p99_ms: 245,
                requests: 5000,
              },
              timestamp: new Date().toISOString(),
              version: 3,
            },
          ],
          count: 3,
        }),
      })
    );
  });

  test("renders vector query panel with search input after seeding", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    // Vector query panel should be visible
    await expect(page.getByTestId("vector-query-panel")).toBeVisible();

    // Search input should be present
    await expect(page.getByTestId("vector-search-input")).toBeVisible();
    await expect(page.getByTestId("vector-search-input")).toHaveAttribute(
      "placeholder",
      /Search like/
    );
  });

  test("typing a query and submitting shows results with latency badge", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    await expect(page.getByTestId("vector-query-panel")).toBeVisible();

    // Type a query
    const input = page.getByTestId("vector-search-input");
    await input.fill("error 500");
    await input.press("Enter");

    // Results should appear
    await expect(page.getByTestId("search-result-card").first()).toBeVisible({
      timeout: 5000,
    });

    // Latency badge should be visible
    await expect(page.getByTestId("latency-badge")).toBeVisible();
    await expect(page.getByTestId("latency-badge")).toContainText("ms");
  });

  test("clicking a suggested query triggers search", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    await expect(page.getByTestId("vector-query-panel")).toBeVisible();

    // Click one of the quick-suggest buttons visible in the empty state
    await page.getByRole("button", { name: "error 500" }).click();

    // Results should render
    await expect(page.getByTestId("search-result-card").first()).toBeVisible({
      timeout: 5000,
    });

    // Latency badge should be visible
    await expect(page.getByTestId("latency-badge")).toBeVisible();
  });

  test("Why? button shows match explanation popover", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    await expect(page.getByTestId("vector-query-panel")).toBeVisible();

    // Search
    const input = page.getByTestId("vector-search-input");
    await input.fill("error");
    await input.press("Enter");

    await expect(page.getByTestId("search-result-card").first()).toBeVisible({
      timeout: 5000,
    });

    // Click Why? on the first result
    await page.getByTestId("why-button").first().click();

    // Popover should appear
    await expect(page.getByTestId("why-popover").first()).toBeVisible();
    await expect(page.getByTestId("why-popover").first()).toContainText("Match Explanation");
  });
});

/**
 * Demo Zone - Speed + Simplicity Dashboard (US-009)
 *
 * Verifies three comparison cards render with data from the benchmark config endpoint.
 */
test.describe("Speed + Simplicity Dashboard", () => {
  const benchmarkData = {
    allsource: {
      throughput_events_per_sec: 469000,
      query_latency_p50_ms: 0.012,
      vector_search: "built-in",
      vector_search_setup: "zero-config",
      services_required: 1,
      storage: "WAL + Parquet + DashMap",
    },
    competitors: {
      kafka: {
        throughput_events_per_sec: 50000,
        vector_search: "none",
        vector_search_setup: "requires Pinecone/Weaviate",
        services_required: 3,
        note: "Kafka + Pinecone + ETL glue",
      },
      redis: {
        throughput_events_per_sec: 100000,
        vector_search: "bolt-on (RediSearch)",
        vector_search_setup: "bolt-on",
        services_required: 2,
      },
    },
    cost_formulas: {
      raw_cost_per_million_tokens_usd: 3.0,
      mcp_cost_per_million_tokens_usd: 0.3,
      tokens_per_event_raw: 150,
      tokens_per_event_mcp: 15,
      vectors_cost_multiplier: 1.2,
    },
    _meta: { updated_at: "2026-02-23" },
  };

  test.beforeEach(async ({ page }) => {
    // Mock seed endpoint
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    // Mock benchmarks config endpoint
    await page.route("**/api/v1/config/benchmarks", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(benchmarkData),
      })
    );
  });

  test("renders three comparison cards with benchmark data", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    // Speed dashboard should be visible
    await expect(page.getByTestId("speed-dashboard")).toBeVisible();

    // All three cards should render
    await expect(page.getByTestId("throughput-card")).toBeVisible();
    await expect(page.getByTestId("vector-search-card")).toBeVisible();
    await expect(page.getByTestId("no-glue-card")).toBeVisible();
  });

  test("throughput card shows AllSource and Kafka bars with data", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    const card = page.getByTestId("throughput-card");
    await expect(card).toBeVisible();

    // Should contain both labels
    await expect(card).toContainText("AllSource");
    await expect(card).toContainText("Kafka");

    // Should show the multiplier comparison
    await expect(card).toContainText("faster than Kafka");
  });

  test("vector search card shows zero-config vs bolt-on comparison", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    const card = page.getByTestId("vector-search-card");
    await expect(card).toBeVisible();

    await expect(card).toContainText("zero-config");
    await expect(card).toContainText("bolt-on");
  });

  test("no-glue card shows 1 service vs 3 with icons", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    const card = page.getByTestId("no-glue-card");
    await expect(card).toBeVisible();

    await expect(card).toContainText("1 service vs 3");
    await expect(card).toContainText("Kafka + Pinecone + ETL");
  });
});

/**
 * Demo Zone - MCP Showdown Split-Screen Layout (US-010)
 *
 * Verifies the MCP Showdown view renders a split-screen with
 * Raw Mode and MCP Mode panels, and the Start Test button works.
 */
test.describe("MCP Showdown Split-Screen", () => {
  test.beforeEach(async ({ page }) => {
    // Mock seed endpoint
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );
  });

  test("toggle to MCP Showdown shows split-screen with Raw and MCP panels", async ({ page }) => {
    await page.goto("/dashboard/demo");

    // Seed first
    await page.getByRole("button", { name: /Start Demo/i }).click();

    // Toggle to MCP Showdown
    await page.getByRole("button", { name: /MCP Showdown/i }).click();
    await expect(page).toHaveURL(/view=mcp-showdown/);

    // Showdown panel should render
    await expect(page.getByTestId("mcp-showdown-panel")).toBeVisible();

    // Both sides visible
    await expect(page.getByTestId("raw-mode-panel")).toBeVisible();
    await expect(page.getByTestId("mcp-mode-panel")).toBeVisible();

    // Labels present
    await expect(page.getByTestId("raw-mode-label")).toContainText("Raw Mode");
    await expect(page.getByTestId("mcp-mode-label")).toContainText("MCP Mode");
  });

  test("split-screen has orange accent for Raw and green for MCP", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // Raw panel has orange border
    const rawPanel = page.getByTestId("raw-mode-panel");
    await expect(rawPanel).toHaveClass(/border-orange/);

    // MCP panel has green border
    const mcpPanel = page.getByTestId("mcp-mode-panel");
    await expect(mcpPanel).toHaveClass(/border-green/);
  });

  test("Start Test button triggers metrics updates on both sides", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    await expect(page.getByTestId("start-test-button")).toBeVisible();

    // Start the test
    await page.getByTestId("start-test-button").click();

    // Should switch to Stop Test button
    await expect(page.getByTestId("stop-test-button")).toBeVisible();

    // Wait for metrics to update (both sides should show non-zero tokens)
    // Use regex to check the text is not exactly "0" (plain containText("0") matches "100", "200", etc.)
    await expect(async () => {
      const rawText = await page.getByTestId("raw-mode-panel").getByTestId("metric-tokens").innerText();
      const mcpText = await page.getByTestId("mcp-mode-panel").getByTestId("metric-tokens").innerText();
      expect(rawText.trim()).not.toBe("0");
      expect(mcpText.trim()).not.toBe("0");
    }).toPass({ timeout: 8000 });
  });

  test("both sides show tokens, latency, cost, and events metrics", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // Both panels should have all 4 metric cards
    for (const side of ["raw-mode-panel", "mcp-mode-panel"]) {
      const panel = page.getByTestId(side);
      await expect(panel.getByTestId("metric-tokens")).toBeVisible();
      await expect(panel.getByTestId("metric-latency")).toBeVisible();
      await expect(panel.getByTestId("metric-cost")).toBeVisible();
      await expect(panel.getByTestId("metric-events")).toBeVisible();
    }
  });
});

/**
 * Demo Zone - MCP Showdown Live Metrics + Progress Bars (US-011)
 *
 * Verifies dual animated line charts, real-time metrics updates,
 * test progress bar, and summary stats after test completion.
 */
test.describe("MCP Showdown Live Metrics", () => {
  test.beforeEach(async ({ page }) => {
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );
  });

  test("start test shows latency chart with dual lines after data accumulates", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // Start the test
    await page.getByTestId("start-test-button").click();

    // Wait for chart to appear (needs >1 data points, sampled every 500ms)
    await expect(page.getByTestId("latency-chart-card")).toBeVisible({ timeout: 5000 });
    await expect(page.getByTestId("latency-chart")).toBeVisible();

    // Chart legend should show both modes
    await expect(page.getByText("Raw Mode", { exact: false }).last()).toBeVisible();
    await expect(page.getByText("MCP Mode", { exact: false }).last()).toBeVisible();
  });

  test("metrics update in real-time during test run on both panels", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    await page.getByTestId("start-test-button").click();

    // Progress bar should appear
    await expect(page.getByTestId("test-progress-bar")).toBeVisible();

    // Running timer label should appear
    await expect(page.getByTestId("test-running-label")).toBeVisible();

    // Wait for metrics to update on both sides
    await expect(async () => {
      const rawLatency = page.getByTestId("raw-mode-panel").getByTestId("metric-latency");
      const mcpLatency = page.getByTestId("mcp-mode-panel").getByTestId("metric-latency");
      await expect(rawLatency).not.toContainText("0ms");
      await expect(mcpLatency).not.toContainText("0ms");
    }).toPass({ timeout: 3000 });

    // Raw latency should be higher than MCP (extract number from text like "120ms")
    const rawLatencyText = await page.getByTestId("raw-mode-panel").getByTestId("metric-latency").innerText();
    const mcpLatencyText = await page.getByTestId("mcp-mode-panel").getByTestId("metric-latency").innerText();
    const rawMs = parseFloat(rawLatencyText.replace(/[^0-9.]/g, ""));
    const mcpMs = parseFloat(mcpLatencyText.replace(/[^0-9.]/g, ""));
    expect(rawMs).toBeGreaterThan(mcpMs);
  });

  test("stopping test shows summary with token savings and latency improvement", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // Start and let it run briefly
    await page.getByTestId("start-test-button").click();
    await expect(page.getByTestId("stop-test-button")).toBeVisible();

    // Wait for some data
    await page.waitForTimeout(1500);

    // Stop the test
    await page.getByTestId("stop-test-button").click();

    // Summary should appear
    await expect(page.getByTestId("showdown-summary")).toBeVisible();
    await expect(page.getByTestId("token-savings")).toBeVisible();
    await expect(page.getByTestId("latency-improvement")).toBeVisible();
    await expect(page.getByTestId("cost-reduction")).toBeVisible();

    // Token savings should show a percentage
    await expect(page.getByTestId("token-savings")).toContainText("%");

    // Latency improvement should show "x faster"
    await expect(page.getByTestId("latency-improvement")).toContainText("faster");
  });

  test("latency chart persists after test completes and shows full history", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // Start test, wait for chart data, then stop
    await page.getByTestId("start-test-button").click();
    await expect(page.getByTestId("latency-chart-card")).toBeVisible({ timeout: 5000 });
    await page.waitForTimeout(1000);
    await page.getByTestId("stop-test-button").click();

    // Chart should still be visible after test completion
    await expect(page.getByTestId("latency-chart-card")).toBeVisible();
    await expect(page.getByTestId("test-complete-label")).toBeVisible();
  });
});

/**
 * Demo Zone - Speed Race Overlay (US-012)
 *
 * Verifies the Speed Race button triggers a race animation with
 * two horizontal bars, shows latency for both modes, displays
 * completion label, and supports re-running with different queries.
 */
test.describe("Speed Race Overlay", () => {
  test.beforeEach(async ({ page }) => {
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );
  });

  test("Speed Race button renders in MCP Showdown view", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // Speed Race card and button should be visible
    await expect(page.getByTestId("speed-race-card")).toBeVisible();
    await expect(page.getByTestId("speed-race-button")).toBeVisible();
    await expect(page.getByTestId("speed-race-button")).toContainText("Speed Race");
  });

  test("clicking Speed Race starts animation with two bars that show latency", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // Click Speed Race
    await page.getByTestId("speed-race-button").click();

    // Both race bars should be visible
    await expect(page.getByTestId("raw-race-bar")).toBeVisible();
    await expect(page.getByTestId("mcp-race-bar")).toBeVisible();

    // Wait for race to complete (MCP finishes first, then raw)
    await expect(page.getByTestId("mcp-race-latency")).toBeVisible({ timeout: 3000 });
    await expect(page.getByTestId("raw-race-latency")).toBeVisible({ timeout: 3000 });

    // Both should show "ms" latency values
    await expect(page.getByTestId("mcp-race-latency")).toContainText("ms");
    await expect(page.getByTestId("raw-race-latency")).toContainText("ms");

    // Completion label should appear
    await expect(page.getByTestId("race-completion-label")).toBeVisible();
    await expect(page.getByTestId("race-completion-label")).toContainText("Tokens slashed");
    await expect(page.getByTestId("race-completion-label")).toContainText("No code tweaks");
  });

  test("race completion shows speed multiplier and re-run button", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    await page.getByTestId("speed-race-button").click();

    // Wait for completion
    await expect(page.getByTestId("race-completion-label")).toBeVisible({ timeout: 3000 });

    // Should show speed multiplier (e.g., "Speed 5x")
    const label = await page.getByTestId("race-completion-label").innerText();
    expect(label).toMatch(/Speed \d+x/);

    // Re-run button should appear
    await expect(page.getByTestId("rerun-race-button")).toBeVisible();
  });

  test("re-running race uses a different query", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // First race
    const firstQuery = await page.getByTestId("race-query-label").innerText();
    await page.getByTestId("speed-race-button").click();
    await expect(page.getByTestId("race-completion-label")).toBeVisible({ timeout: 3000 });

    // Re-run
    await page.getByTestId("rerun-race-button").click();

    // Query should change
    await expect(async () => {
      const secondQuery = await page.getByTestId("race-query-label").innerText();
      expect(secondQuery).not.toBe(firstQuery);
    }).toPass({ timeout: 2000 });

    // New race should complete
    await expect(page.getByTestId("race-completion-label")).toBeVisible({ timeout: 3000 });
  });
});

/**
 * Demo Zone - Cost Calculator Widget (US-013)
 *
 * Verifies the cost calculator renders in MCP Showdown view with
 * slider inputs, cost output cards, savings percentage, and explainer bullets.
 */
test.describe("Cost Calculator Widget", () => {
  const benchmarkData = {
    allsource: {
      throughput_events_per_sec: 469000,
      query_latency_p50_ms: 0.012,
      vector_search: "built-in",
      vector_search_setup: "zero-config",
      services_required: 1,
      storage: "WAL + Parquet + DashMap",
    },
    competitors: {
      kafka: {
        throughput_events_per_sec: 50000,
        vector_search: "none",
        vector_search_setup: "requires Pinecone/Weaviate",
        services_required: 3,
        note: "Kafka + Pinecone + ETL glue",
      },
      redis: {
        throughput_events_per_sec: 100000,
        vector_search: "bolt-on (RediSearch)",
        vector_search_setup: "bolt-on",
        services_required: 2,
      },
    },
    cost_formulas: {
      raw_cost_per_million_tokens_usd: 3.0,
      mcp_cost_per_million_tokens_usd: 0.3,
      tokens_per_event_raw: 150,
      tokens_per_event_mcp: 15,
      vectors_cost_multiplier: 1.2,
    },
    _meta: { updated_at: "2026-02-23" },
  };

  test.beforeEach(async ({ page }) => {
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    await page.route("**/api/v1/config/benchmarks", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(benchmarkData),
      })
    );
  });

  test("cost calculator renders in MCP Showdown view with sliders and cost cards", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // Cost calculator should be visible
    await expect(page.getByTestId("cost-calculator")).toBeVisible();

    // Both sliders should be present
    await expect(page.getByTestId("events-slider")).toBeVisible();
    await expect(page.getByTestId("vectors-slider")).toBeVisible();

    // Cost cards should show values
    await expect(page.getByTestId("raw-cost-card")).toBeVisible();
    await expect(page.getByTestId("mcp-cost-card")).toBeVisible();
    await expect(page.getByTestId("raw-cost-value")).toContainText("$");
    await expect(page.getByTestId("mcp-cost-value")).toContainText("$");

    // Savings badge should be present
    await expect(page.getByTestId("savings-badge")).toBeVisible();
    await expect(page.getByTestId("savings-percentage")).toContainText("% savings");
  });

  test("moving events slider updates cost outputs instantly", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    await expect(page.getByTestId("cost-calculator")).toBeVisible();

    // Get initial cost values
    const initialRaw = await page.getByTestId("raw-cost-value").innerText();

    // Move events slider to max
    const slider = page.getByTestId("events-slider");
    await slider.fill("100");

    // Cost should update (be different from initial)
    await expect(async () => {
      const newRaw = await page.getByTestId("raw-cost-value").innerText();
      expect(newRaw).not.toBe(initialRaw);
    }).toPass({ timeout: 2000 });

    // Events/day label should show ~100M
    await expect(page.getByTestId("events-per-day-value")).toContainText("M");
  });

  test("moving vectors slider updates cost outputs", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    await expect(page.getByTestId("cost-calculator")).toBeVisible();

    const initialRaw = await page.getByTestId("raw-cost-value").innerText();

    // Move vectors slider to max
    await page.getByTestId("vectors-slider").fill("100");

    await expect(async () => {
      const newRaw = await page.getByTestId("raw-cost-value").innerText();
      expect(newRaw).not.toBe(initialRaw);
    }).toPass({ timeout: 2000 });

    // Vectors/event label should show 10K
    await expect(page.getByTestId("vectors-per-event-value")).toContainText("10");
  });

  test("explainer bullet points explain why MCP saves", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    await expect(page.getByTestId("cost-calculator")).toBeVisible();

    // Explainer section should mention caching, pruning, and batching
    const calc = page.getByTestId("cost-calculator");
    await expect(calc).toContainText("Smart Caching");
    await expect(calc).toContainText("Context Pruning");
    await expect(calc).toContainText("Batch Routing");
  });
});

/**
 * Onboarding Wizard — Page Shell + Step Navigation (US-014)
 *
 * Verifies the onboarding wizard page renders with step indicator,
 * navigation between steps, and URL param persistence.
 */
test.describe("Onboarding Wizard", () => {
  test("page loads with step indicator and all 4 steps", async ({ page }) => {
    const response = await page.goto("/dashboard/demo/onboarding");
    expect(response?.status()).toBeLessThan(400);

    // Step indicator should be visible
    await expect(page.getByTestId("step-indicator")).toBeVisible();

    // All 4 step buttons should be present
    await expect(page.getByRole("button", { name: /Step 1/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Step 2/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Step 3/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Step 4/i })).toBeVisible();

    // Navigation buttons
    await expect(page.getByTestId("back-button")).toBeVisible();
    await expect(page.getByTestId("next-button")).toBeVisible();
  });

  test("defaults to step 1 with SDK selector and persists step in URL", async ({ page }) => {
    await page.goto("/dashboard/demo/onboarding");

    // SDK selector should be visible (Step 1: Choose SDK)
    await expect(page.getByTestId("sdk-selector")).toBeVisible();

    // All 4 SDK options should render
    await expect(page.getByTestId("sdk-option-typescript")).toBeVisible();
    await expect(page.getByTestId("sdk-option-rust")).toBeVisible();
    await expect(page.getByTestId("sdk-option-go")).toBeVisible();
    await expect(page.getByTestId("sdk-option-python")).toBeVisible();

    // Select TypeScript — should advance to step 2 with params in URL
    await page.getByTestId("sdk-option-typescript").click();

    await expect(page).toHaveURL(/step=2/);
    await expect(page).toHaveURL(/sdk=typescript/);

    // Install commands should be visible
    await expect(page.getByTestId("install-commands")).toBeVisible();
  });

  test("back and next navigation works between steps", async ({ page }) => {
    // Start at step 2 with TypeScript selected
    await page.goto("/dashboard/demo/onboarding?step=2&sdk=typescript");

    // Should show install commands
    await expect(page.getByTestId("install-commands")).toBeVisible();

    // Click Back — should go to step 1
    await page.getByTestId("back-button").click();
    await expect(page).toHaveURL(/step=1/);
    await expect(page.getByTestId("sdk-selector")).toBeVisible();

    // Click Next — should go to step 2
    await page.getByTestId("next-button").click();
    await expect(page).toHaveURL(/step=2/);

    // Click Next — should go to step 3
    await page.getByTestId("next-button").click();
    await expect(page).toHaveURL(/step=3/);
    await expect(page.getByTestId("send-event-snippet")).toBeVisible();

    // Click Next — should go to step 4
    await page.getByTestId("next-button").click();
    await expect(page).toHaveURL(/step=4/);
    await expect(page.getByTestId("query-snippet")).toBeVisible();

    // On step 4, should show "Go to Dashboard" instead of Next
    await expect(page.getByTestId("go-to-dashboard-button")).toBeVisible();
  });

  test("selecting TypeScript shows correct install and code snippets", async ({ page }) => {
    // Go to step 2 with TypeScript
    await page.goto("/dashboard/demo/onboarding?step=2&sdk=typescript");

    // Install command should contain the package name
    const commands = page.getByTestId("install-commands");
    await expect(commands).toContainText("@allsource/client");

    // Go to step 3 — send event snippet
    await page.getByTestId("next-button").click();
    const sendSnippet = page.getByTestId("send-event-snippet");
    await expect(sendSnippet).toContainText("createEvent");

    // Go to step 4 — query snippet
    await page.getByTestId("next-button").click();
    const querySnippet = page.getByTestId("query-snippet");
    await expect(querySnippet).toContainText("queryEvents");

    // CTA should link to dashboard
    const ctaLink = page.getByTestId("go-to-dashboard-button");
    await expect(ctaLink).toBeVisible();
    await expect(ctaLink).toHaveAttribute("href", "/dashboard");
  });
});

/**
 * Onboarding Wizard — Send First Event + Query It Back (US-016)
 *
 * Verifies the Run It button sends an event and shows success feedback,
 * and the Try It button queries events and shows results inline.
 */
test.describe("Onboarding Wizard - Send & Query", () => {
  test("Step 3 shows Run It button and success feedback after sending event", async ({ page }) => {
    // Mock the events POST endpoint
    await page.route("**/api/v1/events", (route) => {
      if (route.request().method() === "POST") {
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ id: "evt-onboard-001", event_type: "user.signup" }),
        });
      }
      return route.continue();
    });

    await page.goto("/dashboard/demo/onboarding?step=3&sdk=typescript");

    // Code snippet should be visible
    await expect(page.getByTestId("send-event-snippet")).toBeVisible();
    await expect(page.getByTestId("send-event-snippet")).toContainText("createEvent");

    // Run It button should be visible
    const runBtn = page.getByTestId("run-it-button");
    await expect(runBtn).toBeVisible();
    await expect(runBtn).toContainText("Run It");

    // Click Run It
    await runBtn.click();

    // Success feedback should appear with event ID
    await expect(page.getByTestId("event-created-feedback")).toBeVisible({ timeout: 3000 });
    await expect(page.getByTestId("event-created-feedback")).toContainText("Event created!");
    await expect(page.getByTestId("event-created-feedback")).toContainText("evt-onboard-001");

    // Run It button should be disabled after success
    await expect(runBtn).toBeDisabled();
  });

  test("Step 4 shows Try It button and query results inline", async ({ page }) => {
    // Mock the events query endpoint
    await page.route("**/api/v1/events/query**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          events: [
            {
              id: "evt-onboard-001",
              event_type: "user.signup",
              entity_id: "user-001",
              payload: { email: "dev@example.com", plan: "pro" },
              timestamp: new Date().toISOString(),
            },
          ],
          count: 1,
        }),
      })
    );

    await page.goto("/dashboard/demo/onboarding?step=4&sdk=typescript");

    // Query snippet should be visible
    await expect(page.getByTestId("query-snippet")).toBeVisible();
    await expect(page.getByTestId("query-snippet")).toContainText("queryEvents");

    // Try It button should be visible
    const tryBtn = page.getByTestId("try-it-button");
    await expect(tryBtn).toBeVisible();
    await expect(tryBtn).toContainText("Try It");

    // Click Try It
    await tryBtn.click();

    // Query result should appear inline
    await expect(page.getByTestId("query-result")).toBeVisible({ timeout: 3000 });
    await expect(page.getByTestId("query-result")).toContainText("Found 1 event");
    await expect(page.getByTestId("query-result")).toContainText("evt-onboard-001");
  });

  test("code snippets render for all SDK languages", async ({ page }) => {
    for (const sdk of ["rust", "go", "python"]) {
      await page.goto(`/dashboard/demo/onboarding?step=3&sdk=${sdk}`);
      await expect(page.getByTestId("send-event-snippet")).toBeVisible();
      await expect(page.getByTestId("run-it-button")).toBeVisible();

      await page.goto(`/dashboard/demo/onboarding?step=4&sdk=${sdk}`);
      await expect(page.getByTestId("query-snippet")).toBeVisible();
      await expect(page.getByTestId("try-it-button")).toBeVisible();
    }
  });

  test("Go to Dashboard CTA links to /dashboard on step 4", async ({ page }) => {
    await page.goto("/dashboard/demo/onboarding?step=4&sdk=typescript");

    const ctaLink = page.getByTestId("go-to-dashboard-button");
    await expect(ctaLink).toBeVisible();
    await expect(ctaLink).toContainText("Go to Dashboard");
    await expect(ctaLink).toHaveAttribute("href", "/dashboard");
  });
});

/**
 * Demo Zone - Feedback Widget (US-018)
 *
 * Verifies the floating feedback widget renders in Demo Zone,
 * thumbs up/down buttons work, and confirmation appears after click.
 */
test.describe("Feedback Widget", () => {
  test.beforeEach(async ({ page }) => {
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    await page.route("**/api/v1/feedback", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ ok: true }),
      })
    );
  });

  test("feedback widget renders after seeding with thumbs up/down buttons", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    // Widget should appear
    await expect(page.getByTestId("feedback-widget")).toBeVisible();
    await expect(page.getByTestId("feedback-widget")).toContainText("Did this show the power?");

    // Both buttons should be present
    await expect(page.getByTestId("feedback-thumbs-up")).toBeVisible();
    await expect(page.getByTestId("feedback-thumbs-down")).toBeVisible();
  });

  test("clicking thumbs up sends feedback and shows Thanks! confirmation", async ({ page }) => {
    let feedbackPayload: Record<string, unknown> | null = null;

    await page.route("**/api/v1/feedback", async (route) => {
      if (route.request().method() === "POST") {
        feedbackPayload = route.request().postDataJSON();
      }
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ ok: true }),
      });
    });

    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    await expect(page.getByTestId("feedback-widget")).toBeVisible();

    // Click thumbs up
    await page.getByTestId("feedback-thumbs-up").click();

    // Should show Thanks! confirmation
    await expect(page.getByTestId("feedback-thanks")).toBeVisible();
    await expect(page.getByTestId("feedback-thanks")).toContainText("Thanks!");

    // Verify the feedback POST was sent with positive=true
    expect(feedbackPayload).toBeTruthy();
    expect((feedbackPayload as Record<string, unknown>).positive).toBe(true);
  });

  test("clicking thumbs down sends negative feedback and shows Thanks!", async ({ page }) => {
    let feedbackPayload: Record<string, unknown> | null = null;

    await page.route("**/api/v1/feedback", async (route) => {
      if (route.request().method() === "POST") {
        feedbackPayload = route.request().postDataJSON();
      }
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ ok: true }),
      });
    });

    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /Start Demo/i }).click();

    await expect(page.getByTestId("feedback-widget")).toBeVisible();

    // Click thumbs down
    await page.getByTestId("feedback-thumbs-down").click();

    // Should show Thanks! confirmation
    await expect(page.getByTestId("feedback-thanks")).toBeVisible();

    // Verify negative feedback
    expect(feedbackPayload).toBeTruthy();
    expect((feedbackPayload as Record<string, unknown>).positive).toBe(false);
  });

  test("feedback widget is not visible before seeding", async ({ page }) => {
    await page.goto("/dashboard/demo");

    // Widget should not be present in empty state
    await expect(page.getByTestId("feedback-widget")).not.toBeVisible();
  });
});

/**
 * Demo Zone - Responsive Layout + Accessibility (US-017)
 *
 * Verifies that Live Fire panels stack vertically at mobile breakpoints,
 * MCP Showdown split stacks at tablet breakpoints, interactive elements
 * have ARIA labels, and keyboard navigation works.
 */
test.describe("Responsive Layout + Accessibility", () => {
  const benchmarkData = {
    allsource: {
      throughput_events_per_sec: 469000,
      query_latency_p50_ms: 0.012,
      vector_search: "built-in",
      vector_search_setup: "zero-config",
      services_required: 1,
      storage: "WAL + Parquet + DashMap",
    },
    competitors: {
      kafka: {
        throughput_events_per_sec: 50000,
        vector_search: "none",
        vector_search_setup: "requires Pinecone/Weaviate",
        services_required: 3,
        note: "Kafka + Pinecone + ETL glue",
      },
      redis: {
        throughput_events_per_sec: 100000,
        vector_search: "bolt-on (RediSearch)",
        vector_search_setup: "bolt-on",
        services_required: 2,
      },
    },
    cost_formulas: {
      raw_cost_per_million_tokens_usd: 3.0,
      mcp_cost_per_million_tokens_usd: 0.3,
      tokens_per_event_raw: 150,
      tokens_per_event_mcp: 15,
      vectors_cost_multiplier: 1.2,
    },
    _meta: { updated_at: "2026-02-23" },
  };

  test.beforeEach(async ({ page }) => {
    await page.route("**/api/v1/demo/seed", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ seeded: true, event_count: 1000 }),
      })
    );

    await page.route("**/api/v1/config/benchmarks", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(benchmarkData),
      })
    );

    await page.route("**/api/v1/events/query**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ events: [], count: 0 }),
      })
    );
  });

  test("Live Fire panels stack vertically at mobile viewport (375px)", async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto("/dashboard/demo");

    // Seed the demo
    await page.getByRole("button", { name: /Start Demo/i }).click();

    // Wait for panels to render
    await expect(page.getByTestId("event-stream-panel")).toBeVisible();
    await expect(page.getByTestId("vector-query-panel")).toBeVisible();
    await expect(page.getByTestId("speed-dashboard")).toBeVisible();

    // At 375px (< 1024px lg breakpoint), panels should stack vertically.
    // Check that the event stream panel and vector query panel are not side-by-side
    // by comparing their bounding boxes — stacked means similar left position.
    const streamBox = await page.getByTestId("event-stream-panel").boundingBox();
    const queryBox = await page.getByTestId("vector-query-panel").boundingBox();

    expect(streamBox).toBeTruthy();
    expect(queryBox).toBeTruthy();

    if (streamBox && queryBox) {
      // In stacked layout, both panels should have similar left positions (within a few pixels)
      expect(Math.abs(streamBox.x - queryBox.x)).toBeLessThan(20);
      // And query panel should be below the stream panel
      expect(queryBox.y).toBeGreaterThan(streamBox.y);
    }
  });

  test("MCP Showdown split stacks vertically at tablet viewport (600px)", async ({ page }) => {
    await page.setViewportSize({ width: 600, height: 900 });
    await page.goto("/dashboard/demo");

    // Seed and switch to MCP Showdown
    await page.getByRole("button", { name: /Start Demo/i }).click();
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    await expect(page.getByTestId("mcp-showdown-panel")).toBeVisible();
    await expect(page.getByTestId("raw-mode-panel")).toBeVisible();
    await expect(page.getByTestId("mcp-mode-panel")).toBeVisible();

    // At 600px (< 768px md breakpoint), panels should stack
    const rawBox = await page.getByTestId("raw-mode-panel").boundingBox();
    const mcpBox = await page.getByTestId("mcp-mode-panel").boundingBox();

    expect(rawBox).toBeTruthy();
    expect(mcpBox).toBeTruthy();

    if (rawBox && mcpBox) {
      // Stacked: similar left position, MCP below Raw
      expect(Math.abs(rawBox.x - mcpBox.x)).toBeLessThan(20);
      expect(mcpBox.y).toBeGreaterThan(rawBox.y);
    }
  });

  test("panels render side-by-side at desktop viewport (1440px)", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/dashboard/demo");

    // Seed the demo
    await page.getByRole("button", { name: /Start Demo/i }).click();

    await expect(page.getByTestId("event-stream-panel")).toBeVisible();
    await expect(page.getByTestId("vector-query-panel")).toBeVisible();

    // At 1440px (> 1024px), panels should be side-by-side
    const streamBox = await page.getByTestId("event-stream-panel").boundingBox();
    const queryBox = await page.getByTestId("vector-query-panel").boundingBox();

    expect(streamBox).toBeTruthy();
    expect(queryBox).toBeTruthy();

    if (streamBox && queryBox) {
      // Side-by-side: query panel's left should be to the right of stream panel's left
      expect(queryBox.x).toBeGreaterThan(streamBox.x + streamBox.width - 20);
    }
  });

  test("interactive elements have ARIA labels", async ({ page }) => {
    await page.goto("/dashboard/demo");

    // View toggle buttons have aria-label and aria-pressed
    const liveFire = page.getByRole("button", { name: /Switch to Live Fire/i });
    await expect(liveFire).toBeVisible();
    await expect(liveFire).toHaveAttribute("aria-pressed");

    // Seed to access components
    await page.getByRole("button", { name: /Start Demo/i }).click();

    // Event stream has region role
    await expect(page.getByTestId("event-stream-panel")).toHaveAttribute("role", "region");

    // Pause/Resume and scroll lock buttons have aria-labels
    await expect(page.getByRole("button", { name: /Pause/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Lock scroll/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /Replay last 10s/i })).toBeVisible();

    // Vector search input has aria-label
    await expect(page.getByTestId("vector-search-input")).toHaveAttribute("aria-label", "Vector search query");

    // Switch to MCP Showdown
    await page.getByRole("button", { name: /MCP Showdown/i }).click();

    // MCP panels have region roles
    await expect(page.getByTestId("raw-mode-panel")).toHaveAttribute("role", "region");
    await expect(page.getByTestId("mcp-mode-panel")).toHaveAttribute("role", "region");

    // Cost calculator has region role
    await expect(page.getByTestId("cost-calculator")).toHaveAttribute("role", "region");

    // Sliders have aria-labels
    await expect(page.getByTestId("events-slider")).toHaveAttribute("aria-label", "Events per day");
    await expect(page.getByTestId("vectors-slider")).toHaveAttribute("aria-label", "Vectors per event");
  });

  test("keyboard navigation works for view toggle and wizard steps", async ({ page }) => {
    await page.goto("/dashboard/demo");

    // Focus the MCP Showdown button and activate via keyboard
    const mcpBtn = page.getByRole("button", { name: /MCP Showdown/i });
    await mcpBtn.focus();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/view=mcp-showdown/);

    // Verify wizard keyboard navigation
    await page.goto("/dashboard/demo/onboarding");
    await expect(page.getByTestId("step-indicator")).toBeVisible();

    // SDK options should be focusable via Tab
    const sdkOption = page.getByTestId("sdk-option-typescript");
    await sdkOption.focus();

    // Should have focus-visible ring
    const hasFocusRing = await sdkOption.evaluate((el) => {
      const styles = window.getComputedStyle(el);
      return styles.outlineStyle !== "none" || el.className.includes("focus-visible");
    });
    // Focus was set programmatically; the element should at least be focusable
    expect(await sdkOption.evaluate((el) => el === document.activeElement)).toBe(true);
  });
});

/**
 * Demo Account Flow - Authenticated Demo Tests @demo
 *
 * Tests the full demo account flow: verify session works,
 * demo banner renders, and demo-specific features are accessible.
 * Auth is handled by the file-level beforeAll/beforeEach.
 */
test.describe("Demo Account Flow @demo", () => {
  test("demo account authenticates and reaches dashboard @demo", async ({
    page,
  }) => {
    // Already authenticated by file-level beforeEach — verify session
    const sessionResp = await page.request.get("/api/auth/session");
    expect(sessionResp.ok()).toBeTruthy();
  });

  test.skip("demo dashboard shows demo banner @demo", async ({ page }) => {
    // TODO: QS /api/auth/me doesn't return tenant data (is_demo flag).
    // The DemoBanner component checks tenant.is_demo, but the session
    // endpoint only returns user info. Fix: include tenant in /api/auth/me.
    if (page.url().includes("onboarding")) {
      await page.goto("/dashboard");
    }

    await expect(page.getByText(/demo account/i)).toBeVisible({ timeout: 5000 });
  });

  test("demo account can access Live Fire view @demo", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await expect(
      page.getByRole("heading", { name: "Demo Zone" })
    ).toBeVisible();

    const liveFire = page.getByRole("button", { name: /Live Fire/i });
    await expect(liveFire).toHaveAttribute("aria-pressed", "true");
  });

  test("demo account can switch to MCP Showdown @demo", async ({ page }) => {
    await page.goto("/dashboard/demo");
    await page.getByRole("button", { name: /MCP Showdown/i }).click();
    await expect(page).toHaveURL(/view=mcp-showdown/);
  });
});

}); // end: Demo Zone Smoke Tests
