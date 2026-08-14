import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ReplayPage from "@/app/dashboard/tools/replay/page";
import { useReplays } from "@/hooks/use-replay";
import { apiClient, type Projection, type ReplayAnalysis } from "@/lib/api/client";

vi.mock("@/hooks/use-replay", () => ({
  useReplays: vi.fn(),
}));

vi.mock("@/lib/api/client", () => ({
  apiClient: {
    listProjections: vi.fn(),
    analyzeReplay: vi.fn(),
  },
}));

const projection: Projection = {
  name: "event-count",
  title: "Event Count",
  description: "Counts tenant events.",
  kind: "counter",
  status: "ready",
};

const analysis: ReplayAnalysis = {
  projection_name: "event-count",
  projection_title: "Event Count",
  projection_kind: "counter",
  projection_status: "ready",
  current_entity_count: 1,
  total_events: 42,
  sampled_events: 42,
  analysis_scope: "full",
  event_type_distribution: [{ event_type: "order.created", count: 42, share: 100 }],
  sampled_entity_count: 10,
  sampled_entities: [{ entity_id: "order-1", event_count: 4 }],
  first_event_at: "2026-08-01T10:00:00Z",
  last_event_at: "2026-08-02T10:00:00Z",
  analyzed_at: "2026-08-14T10:00:00Z",
  ready_to_replay: true,
  checks: [
    {
      key: "tenant_scope",
      label: "Tenant boundary",
      status: "pass",
      detail: "Authenticated tenant only.",
    },
  ],
  warnings: [],
};

describe("Replay Studio", () => {
  const startReplay = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    startReplay.mockResolvedValue(null);
    vi.mocked(useReplays).mockReturnValue({
      replays: [],
      total: 0,
      isLoading: false,
      error: undefined,
      startReplay,
      cancelReplay: vi.fn(),
      deleteReplay: vi.fn(),
      refresh: vi.fn(),
    });
    vi.mocked(apiClient.listProjections).mockResolvedValue({
      data: { projections: [projection], total: 1 },
    });
    vi.mocked(apiClient.analyzeReplay).mockResolvedValue({ data: analysis });
  });

  it("requires impact analysis before starting an atomic replay", async () => {
    render(<ReplayPage />);

    expect(await screen.findByText("Counts tenant events.")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Start safe replay" })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Analyze impact" }));

    await waitFor(() =>
      expect(apiClient.analyzeReplay).toHaveBeenCalledWith({ projection_name: "event-count" })
    );
    const impact = await screen.findByLabelText("Replay impact analysis");
    expect(within(impact).getByText("42")).toBeVisible();
    expect(within(impact).getByText("order.created")).toBeVisible();
    expect(within(impact).getByText("Tenant boundary")).toBeVisible();

    fireEvent.click(within(impact).getByRole("button", { name: "Start safe replay" }));

    await waitFor(() =>
      expect(startReplay).toHaveBeenCalledWith({ projection_name: "event-count" })
    );
  });
});
