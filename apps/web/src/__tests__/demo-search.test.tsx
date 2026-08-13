import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VectorQueryPlayground } from "@/components/demo/vector-query-playground";

const events = [
  {
    id: "event-1",
    entity_id: "worker-pool-01",
    event_type: "metric.memory",
    payload: { message: "Heap growth suggests a memory leak" },
    timestamp: "2026-08-14T00:00:00Z",
    version: 1,
  },
  {
    id: "event-2",
    entity_id: "checkout-api-01",
    event_type: "log.error",
    payload: { message: "Payment provider returned HTTP 500" },
    timestamp: "2026-08-14T00:00:01Z",
    version: 1,
  },
];

describe("Demo event search", () => {
  it("returns deterministic lexical matches without vector claims", async () => {
    render(<VectorQueryPlayground initialEvents={events} />);

    expect(screen.getByRole("region", { name: "Event search playground" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "memory leak" }));

    expect(await screen.findByText("Heap growth suggests a memory leak")).toBeVisible();
    expect(screen.getByText("100% match")).toBeVisible();
    expect(screen.queryByText(/cosine similarity/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/semantic similarity/i)).not.toBeInTheDocument();
  });
});
