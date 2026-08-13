import { NextRequest } from "next/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { POST } from "@/app/api/v1/demo/seed/route";

function seedRequest(token?: string): NextRequest {
  return new NextRequest("http://localhost/api/v1/demo/seed", {
    method: "POST",
    headers: token ? { cookie: `auth_token=${token}` } : undefined,
  });
}

describe("POST /api/v1/demo/seed", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
  });

  it("targets Query Service instead of the Control Plane gateway", async () => {
    vi.stubEnv("QUERY_SERVICE_URL", "https://query.example.test");
    const fetchMock = vi
      .fn()
      .mockResolvedValue(Response.json({ seeded: true, event_count: 1000 }, { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    const response = await POST(seedRequest("session-token"));

    expect(fetchMock).toHaveBeenCalledWith(
      "https://query.example.test/api/v1/demo/seed",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({ authorization: "Bearer session-token" }),
      })
    );
    expect(response.status).toBe(200);
    await expect(response.json()).resolves.toEqual({ seeded: true, event_count: 1000 });
  });

  it("returns an actionable error when Query Service is unavailable", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("connection refused")));

    const response = await POST(seedRequest());

    expect(response.status).toBe(502);
    await expect(response.json()).resolves.toEqual({
      error: "Demo service is unavailable. Try again shortly.",
    });
  });
});
