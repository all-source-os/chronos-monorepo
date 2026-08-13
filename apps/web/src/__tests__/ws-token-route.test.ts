import { NextRequest } from "next/server";
import { describe, expect, it } from "vitest";
import { GET } from "@/app/api/auth/ws-token/route";

function tokenWith(payload: Record<string, unknown>): string {
  const encoded = Buffer.from(JSON.stringify(payload)).toString("base64url");
  return `header.${encoded}.signature`;
}

function requestWith(token?: string): NextRequest {
  return new NextRequest("http://localhost/api/auth/ws-token", {
    headers: token ? { cookie: `auth_token=${token}` } : undefined,
  });
}

describe("GET /api/auth/ws-token", () => {
  it("returns a credential-free session token", async () => {
    const token = tokenWith({ sub: "user-1", tenant_id: "tenant-1" });

    const response = await GET(requestWith(token));

    expect(response.status).toBe(200);
    await expect(response.json()).resolves.toEqual({ token });
  });

  it("rejects a legacy session containing a long-lived API key", async () => {
    const response = await GET(
      requestWith(tokenWith({ sub: "user-1", core_api_key: "legacy-secret" }))
    );

    expect(response.status).toBe(401);
    expect(response.headers.get("set-cookie")).toContain("auth_token=");
    await expect(response.json()).resolves.toMatchObject({
      error: { code: "session_refresh_required" },
    });
  });
});
