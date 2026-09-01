import { NextRequest } from "next/server";
import { describe, expect, it } from "vitest";
import { proxy } from "@/proxy";

describe("web proxy", () => {
  it("redirects the apex domain to the canonical www origin", () => {
    const request = new NextRequest("https://all-source.xyz/pricing?source=launch", {
      headers: { host: "all-source.xyz" },
    });

    const response = proxy(request);

    expect(response.status).toBe(308);
    expect(response.headers.get("location")).toBe(
      "https://www.all-source.xyz/pricing?source=launch"
    );
  });

  it("serves Fly and www hosts without a canonical redirect", () => {
    for (const host of ["www.all-source.xyz", "allsource-web.fly.dev"]) {
      const request = new NextRequest(`https://${host}/api/healthz`, {
        headers: { host },
      });

      expect(proxy(request).status).toBe(200);
    }
  });
});
