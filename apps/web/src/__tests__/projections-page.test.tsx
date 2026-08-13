import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PipelinesPage from "@/app/dashboard/pipelines/page";
import { apiClient, type Projection, type ProjectionTemplate } from "@/lib/api/client";

vi.mock("next/dynamic", () => ({
  default: vi.fn(() => () => null),
}));

vi.mock("@/lib/api/client", () => ({
  apiClient: {
    listProjections: vi.fn(),
    listProjectionTemplates: vi.fn(),
    enableProjection: vi.fn(),
    disableProjection: vi.fn(),
  },
}));

const templates: ProjectionTemplate[] = [
  {
    name: "event-count",
    title: "Event Count",
    description: "Counts all tenant events.",
    kind: "counter",
  },
  {
    name: "events-per-day",
    title: "Events Per Day",
    description: "Daily event volume.",
    kind: "timeseries",
  },
];

const eventCountProjection: Projection = {
  name: "event-count",
  title: "Event Count",
  description: "Counts all tenant events.",
  kind: "counter",
  status: "ready",
};

describe("Projections page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(apiClient.listProjectionTemplates).mockResolvedValue({
      data: { templates, total: templates.length },
    });
    vi.mocked(apiClient.enableProjection).mockResolvedValue({
      data: { projection: eventCountProjection },
    });
    vi.mocked(apiClient.disableProjection).mockResolvedValue({
      data: { deleted: "event-count" },
    });
  });

  it("turns the empty state into an actionable projection catalog", async () => {
    vi.mocked(apiClient.listProjections)
      .mockResolvedValueOnce({ data: { projections: [], total: 0 } })
      .mockResolvedValueOnce({
        data: { projections: [eventCountProjection], total: 1 },
      });

    render(<PipelinesPage />);

    expect(
      await screen.findByRole("heading", { name: "Choose your first read model" })
    ).toBeVisible();
    expect(screen.queryByText("No projections enabled")).not.toBeInTheDocument();
    expect(screen.queryByText("0 Enabled")).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "View event history" })).toHaveAttribute(
      "href",
      "/dashboard/events"
    );

    fireEvent.click(screen.getByRole("button", { name: "Enable Event Count projection" }));

    await waitFor(() => expect(apiClient.enableProjection).toHaveBeenCalledWith("event-count"));
    expect(await screen.findByText("Enabled read models")).toBeVisible();
  });

  it("shows compact status and only remaining templates for active tenants", async () => {
    vi.mocked(apiClient.listProjections).mockResolvedValue({
      data: { projections: [eventCountProjection], total: 1 },
    });

    render(<PipelinesPage />);

    const status = await screen.findByRole("region", { name: "Projection status" });
    expect(within(status).getByText("Enabled")).toBeVisible();
    expect(within(status).getByText("Ready")).toBeVisible();
    expect(screen.getByRole("button", { name: "Actions for Event Count" })).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Add read model" }));

    const dialog = screen.getByRole("dialog", { name: "Add a read model" });
    expect(
      within(dialog).getByRole("button", { name: "Enable Events Per Day projection" })
    ).toBeVisible();
    expect(
      within(dialog).queryByRole("button", { name: "Enable Event Count projection" })
    ).not.toBeInTheDocument();
  });
});
