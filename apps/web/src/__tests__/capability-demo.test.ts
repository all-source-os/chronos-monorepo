import { describe, expect, it } from "vitest";
import {
  CAPABILITY_EVENTS,
  graphAt,
  mcpExchange,
  projectionAt,
  reconstructOrderState,
} from "@/lib/capability-demo";

describe("capability demo model", () => {
  it("reconstructs state only through the selected event", () => {
    expect(reconstructOrderState(1)).toMatchObject({
      status: "confirmed",
      payment: "pending",
      inventory: "not reserved",
      postcode: "E1 6AA",
    });

    expect(reconstructOrderState(CAPABILITY_EVENTS.length - 1)).toMatchObject({
      status: "dispatched",
      payment: "authorized",
      inventory: "reserved",
      postcode: "E1 6AN",
      shipment: "ship_48 · Northline Express",
    });
  });

  it("reveals graph relationships only when source events exist", () => {
    expect(graphAt(1)).toMatchObject({
      nodes: expect.arrayContaining([]),
      edges: [{ label: "PLACED" }],
    });
    expect(graphAt(1).nodes).toHaveLength(2);
    expect(graphAt(5).nodes).toHaveLength(5);
    expect(graphAt(5).edges).toHaveLength(4);
  });

  it("builds a versioned Query Service projection from visible history", () => {
    expect(projectionAt(3)).toMatchObject({
      projection: "order-summary",
      kind: "entity_table",
      version: 4,
      applied_events: 4,
      state: { payment: "authorized", inventory: "reserved", shipment: "not assigned" },
    });
  });

  it("uses real MCP event-store tool shapes at the history cursor", () => {
    const exchange = mcpExchange("reconstruct_state", 2);
    expect(exchange.request).toEqual({
      entity_id: "order-1042",
      as_of: "2026-08-14T09:02:41Z",
    });
    expect(exchange.response).toMatchObject({ version: 3, state: { payment: "authorized" } });
  });
});
