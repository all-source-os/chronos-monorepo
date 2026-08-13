import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import DemoPage from "@/app/dashboard/demo/page";

const push = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push }),
  useSearchParams: () => new URLSearchParams(),
}));

vi.mock("next/dynamic", () => ({
  default: vi.fn(() => () => null),
}));

const storedEvent = {
  id: "event-1",
  entity_id: "checkout-api-01",
  event_type: "log.error",
  payload: { message: "Payment provider returned HTTP 500" },
  timestamp: "2026-08-14T00:00:00Z",
  version: 1,
};

describe("Demo Zone", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("seeds the current workspace then hydrates stored events", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(Response.json({ data: [], count: 0 }))
      .mockResolvedValueOnce(Response.json({ seeded: true, event_count: 60 }))
      .mockResolvedValueOnce(Response.json({ data: [storedEvent], count: 1 }));
    vi.stubGlobal("fetch", fetchMock);

    render(<DemoPage />);

    const seedButton = await screen.findByRole("button", { name: "Add sample events" });
    expect(screen.getByText("Put real events on screen")).toBeVisible();
    fireEvent.click(seedButton);

    expect(await screen.findByText("1 events loaded")).toBeVisible();
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/v1/demo/seed",
      expect.objectContaining({ method: "POST" })
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/events?limit=100",
      expect.objectContaining({ cache: "no-store" })
    );
  });

  it("restores the active demo from workspace events after refresh", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(Response.json({ data: [storedEvent], count: 1 }))
    );

    render(<DemoPage />);

    expect(await screen.findByText("1 events loaded")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Add sample events" })).not.toBeInTheDocument();
    await waitFor(() => expect(screen.getByText(/current workspace/)).toBeVisible());
  });

  it("offers recovery when workspace events cannot load", async () => {
    vi.mocked(global.fetch).mockResolvedValueOnce(
      new Response(JSON.stringify({ error: "upstream unavailable" }), { status: 502 })
    );

    render(<DemoPage />);

    expect(await screen.findByText("Workspace events unavailable")).toBeVisible();
    expect(screen.getByRole("button", { name: "Retry" })).toBeVisible();
  });
});
