import { afterEach, describe, expect, it, vi } from "vitest";
import { GET } from "@/app/api/v1/config/benchmarks/route";

describe("GET /api/v1/config/benchmarks", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it("targets Query Service instead of the authenticated gateway", async () => {
    vi.stubEnv("QUERY_SERVICE_URL", "https://query.example.test");
    const benchmark = { allsource: { throughput_events_per_sec: 469_000 } };
    const fetchMock = vi.fn().mockResolvedValue(Response.json(benchmark));
    vi.stubGlobal("fetch", fetchMock);

    const response = await GET();

    expect(fetchMock).toHaveBeenCalledWith("https://query.example.test/api/v1/config/benchmarks", {
      cache: "no-store",
    });
    expect(response.status).toBe(200);
    await expect(response.json()).resolves.toEqual(benchmark);
  });

  it("returns an actionable error when Query Service is unavailable", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("connection refused")));

    const response = await GET();

    expect(response.status).toBe(502);
    await expect(response.json()).resolves.toEqual({
      error: "Benchmark source is unavailable. Try again shortly.",
    });
  });
});
