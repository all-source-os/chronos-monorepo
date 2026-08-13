import { describe, expect, it } from "vitest";
import type { Event } from "@/lib/api/client";
import { eventMatchesLocalFilters } from "@/lib/event-filters";

const event: Event = {
  id: "evt-1",
  entity_id: "customer-42",
  event_type: "invoice.paid",
  payload: {},
  timestamp: "2026-08-13T12:00:00.000Z",
  version: 1,
};

describe("eventMatchesLocalFilters", () => {
  it("matches event type and entity searches without case sensitivity", () => {
    expect(eventMatchesLocalFilters(event, "INVOICE", "")).toBe(true);
    expect(eventMatchesLocalFilters(event, "Customer-42", "")).toBe(true);
    expect(eventMatchesLocalFilters(event, "signup", "")).toBe(false);
  });

  it("filters events before the selected local date", () => {
    expect(eventMatchesLocalFilters(event, "", "2026-08-13")).toBe(true);
    expect(eventMatchesLocalFilters(event, "", "2026-08-14")).toBe(false);
  });
});
