import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CapabilityWorkbench } from "@/components/demo/capability-workbench";

describe("CapabilityWorkbench", () => {
  it("covers all six product surfaces with one history cursor", () => {
    render(<CapabilityWorkbench />);

    expect(screen.getByRole("heading", { name: "Event Timeline" })).toBeVisible();
    expect(screen.getByRole("heading", { name: /Time travel/ })).toBeVisible();
    expect(screen.getByRole("heading", { name: /Graph visualisation/ })).toBeVisible();
    expect(screen.getByRole("heading", { name: /pipeline/ })).toBeVisible();
    expect(screen.getByRole("heading", { name: /Projection state/ })).toBeVisible();
    expect(screen.getByRole("heading", { name: /MCP data access/ })).toBeVisible();
    expect(screen.getByText("6 / 6 events applied")).toBeVisible();
  });

  it("updates historical state, graph, and projection from the same cursor", () => {
    render(<CapabilityWorkbench />);

    fireEvent.change(screen.getByRole("slider", { name: "Event history position" }), {
      target: { value: "1" },
    });

    expect(screen.getByText("2 / 6 events applied")).toBeVisible();

    const timeTravel = screen.getByRole("heading", { name: /Time travel/ }).closest("section")!;
    expect(within(timeTravel).getByText("pending")).toBeVisible();
    expect(within(timeTravel).getByText("not reserved")).toBeVisible();

    const graph = screen.getByRole("heading", { name: /Graph visualisation/ }).closest("section")!;
    expect(within(graph).getByText("2 nodes · 1 edges")).toBeVisible();

    const projection = screen
      .getByRole("heading", { name: /Projection state/ })
      .closest("section")!;
    expect(
      within(projection).getByText("2 events folded · HTTP, realtime, or analytics read paths")
    ).toBeVisible();
  });

  it("switches between connector-compatible MCP tool examples", () => {
    render(<CapabilityWorkbench />);

    fireEvent.click(screen.getByRole("button", { name: /reconstruct_state/ }));

    expect(screen.getByText(/"name": "reconstruct_state"/)).toBeVisible();
    expect(screen.getAllByText(/"as_of": "2026-08-14T09:08:17Z"/)).toHaveLength(2);
  });
});
