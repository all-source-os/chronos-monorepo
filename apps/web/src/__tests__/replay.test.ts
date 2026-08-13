import { describe, expect, it } from "vitest";
import { normalizeApiError } from "@/lib/api/client";
import { normalizeReplayStatus } from "@/lib/replay";

describe("normalizeReplayStatus", () => {
  it.each([
    ["running", "running"],
    ["Running", "running"],
    ["COMPLETED", "completed"],
    ["cancelled", "cancelled"],
  ])("normalizes %s", (input, expected) => {
    expect(normalizeReplayStatus(input)).toBe(expected);
  });

  it("keeps an unexpected backend status from crashing replay history", () => {
    expect(normalizeReplayStatus("paused")).toBe("unknown");
    expect(normalizeReplayStatus(undefined)).toBe("unknown");
  });
});

describe("normalizeApiError", () => {
  it("preserves plain backend error messages", () => {
    expect(normalizeApiError("Projection is not enabled", "Request failed")).toEqual({
      code: "request_failed",
      message: "Projection is not enabled",
    });
  });
});
