import { NextRequest } from "next/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GET } from "@/app/api/auth/session/route";

function tokenWith(payload: Record<string, unknown>): string {
  const encoded = Buffer.from(JSON.stringify(payload)).toString("base64url");
  return `header.${encoded}.signature`;
}

function sessionRequest(token?: string): NextRequest {
  return new NextRequest("http://localhost/api/auth/session", {
    headers: token ? { cookie: `auth_token=${token}` } : undefined,
  });
}

describe("GET /api/auth/session", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("rejects requests without a session cookie", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    const response = await GET(sessionRequest());

    expect(response.status).toBe(401);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("keeps the cookie when the backend is unavailable", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(new Response("unavailable", { status: 503 }))
        .mockResolvedValueOnce(new Response("unavailable", { status: 503 }))
    );

    const response = await GET(sessionRequest(tokenWith({ sub: "user-1" })));

    expect(response.status).toBe(503);
    expect(response.headers.get("set-cookie")).toBeNull();
    await expect(response.json()).resolves.toMatchObject({
      error: { code: "session_unavailable" },
    });
  });

  it("clears a cookie only when the backend rejects the token", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValueOnce(new Response("unauthorized", { status: 401 }))
        .mockResolvedValueOnce(new Response("unauthorized", { status: 401 }))
    );

    const response = await GET(sessionRequest(tokenWith({ sub: "user-1" })));

    expect(response.status).toBe(401);
    expect(response.headers.get("set-cookie")).toContain("auth_token=");
  });

  it("returns user, tenant, and the JWT sync key", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(Response.json({ data: { user: { id: "user-1", name: "Ada" } } }))
      .mockResolvedValueOnce(Response.json({ data: { id: "tenant-1", name: "Acme" } }));
    vi.stubGlobal("fetch", fetchMock);

    const response = await GET(
      sessionRequest(tokenWith({ sub: "user-1", core_api_key: "ask_sync" }))
    );

    expect(response.status).toBe(200);
    await expect(response.json()).resolves.toEqual({
      data: {
        user: { id: "user-1", name: "Ada" },
        tenant: { id: "tenant-1", name: "Acme" },
        core_api_key: "ask_sync",
      },
    });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
});
