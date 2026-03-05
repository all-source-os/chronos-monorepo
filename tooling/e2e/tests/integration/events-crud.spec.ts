import { test, expect } from "../../fixtures/api-auth";
import { expectOk } from "../../fixtures/api-helpers";

/**
 * Events CRUD — Write event → read back → verify
 *
 * THE critical integration test. If this fails, data is being lost
 * between QS and Core. Uses apiContext fixture for authenticated requests.
 */

test.describe("Events CRUD", () => {
  const uniqueType = `e2e.integration.${Date.now()}`;
  const uniqueEntity = `e2e-entity-${Date.now()}`;

  test("POST /api/events creates an event", async ({ apiContext }) => {
    const resp = await apiContext.post("/api/events", {
      data: {
        event_type: uniqueType,
        entity_id: uniqueEntity,
        data: { test: true, source: "e2e-integration", ts: Date.now() },
      },
    });

    await expectOk(resp, "POST /api/events");
  });

  test("GET /api/events?entity_id=... returns the created event", async ({
    apiContext,
  }) => {
    const resp = await apiContext.get(
      `/api/events?entity_id=${encodeURIComponent(uniqueEntity)}`
    );
    await expectOk(resp, "GET /api/events?entity_id");

    const body = await resp.json();
    const events = body.events || body.data || body;
    expect(Array.isArray(events)).toBeTruthy();
    expect(events.length).toBeGreaterThan(0);

    const found = events.find(
      (e: any) => e.entity_id === uniqueEntity || e.entityId === uniqueEntity
    );
    expect(found).toBeDefined();
  });

  test("GET /api/events?event_type=... filters correctly", async ({
    apiContext,
  }) => {
    const resp = await apiContext.get(
      `/api/events?event_type=${encodeURIComponent(uniqueType)}`
    );
    await expectOk(resp, "GET /api/events?event_type");

    const body = await resp.json();
    const events = body.events || body.data || body;
    expect(Array.isArray(events)).toBeTruthy();

    for (const event of events) {
      const type = event.event_type || event.eventType;
      expect(type).toBe(uniqueType);
    }
  });

  test("POST /api/events/batch creates multiple events", async ({
    apiContext,
  }) => {
    const batchType = `e2e.batch.${Date.now()}`;
    const resp = await apiContext.post("/api/events/batch", {
      data: {
        events: [
          {
            event_type: batchType,
            entity_id: `batch-1-${Date.now()}`,
            data: { index: 1 },
          },
          {
            event_type: batchType,
            entity_id: `batch-2-${Date.now()}`,
            data: { index: 2 },
          },
          {
            event_type: batchType,
            entity_id: `batch-3-${Date.now()}`,
            data: { index: 3 },
          },
        ],
      },
    });

    await expectOk(resp, "POST /api/events/batch");
  });

  test("GET /api/event-types returns list", async ({ apiContext }) => {
    const resp = await apiContext.get("/api/event-types");
    await expectOk(resp, "GET /api/event-types");

    const body = await resp.json();
    const types = body.event_types || body.eventTypes || body.data || body;
    expect(Array.isArray(types)).toBeTruthy();
  });

  test("GET /api/streams returns list", async ({ apiContext }) => {
    const resp = await apiContext.get("/api/streams");
    await expectOk(resp, "GET /api/streams");

    const body = await resp.json();
    const streams = body.streams || body.data || body;
    expect(Array.isArray(streams)).toBeTruthy();
  });
});
