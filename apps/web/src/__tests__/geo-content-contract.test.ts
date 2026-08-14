import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { siteConfig } from "@/lib/config";
import type { Catalog } from "@/lib/pricing-catalog";
import { softwareApplicationSchema } from "@/lib/structured-data";

const WEB_ROOT = path.resolve(__dirname, "../..");

function source(relativePath: string): string {
  return readFileSync(path.join(WEB_ROOT, relativePath), "utf-8");
}

describe("GEO canonical product facts", () => {
  it("keeps visible fallback pricing on the verified GBP catalog snapshot", () => {
    const paid = siteConfig.pricing.filter((tier) => !tier.isSelfHost && !tier.isEnterprise);

    expect(paid.map((tier) => tier.price)).toEqual(["£18.99", "£78.99", "£298.99"]);
    expect(siteConfig.referenceReadLatency).toBe("11.9μs p99");
    expect(siteConfig.stats[2]).toMatchObject({ display: "55", label: "default MCP tools" });
  });

  it("publishes offers only from a live billing catalog", () => {
    const catalog: Catalog = {
      currency: "GBP",
      tiers: [
        { tier: "indie", monthly: { cents: 1899, formatted: "£18.99" } },
        { tier: "studio", monthly: { cents: 7899, formatted: "£78.99" } },
        { tier: "scale", monthly: { cents: 29899, formatted: "£298.99" } },
      ],
    };

    const withoutCatalog = softwareApplicationSchema() as { offers?: unknown[] };
    const withCatalog = softwareApplicationSchema(catalog) as {
      offers?: { price: string; priceCurrency: string }[];
    };

    expect(withoutCatalog.offers).toBeUndefined();
    expect(withCatalog.offers).toHaveLength(3);
    expect(withCatalog.offers?.map((offer) => offer.price)).toEqual(["18.99", "78.99", "298.99"]);
    expect(withCatalog.offers?.every((offer) => offer.priceCurrency === "GBP")).toBe(true);
  });

  it("keeps llms.txt aligned with current prices, tool counts, and benchmark scope", () => {
    const llms = source("public/llms.txt");

    expect(llms).toContain("£18.99/mo; £181.99/year");
    expect(llms).toContain("£78.99/mo; £757.99/year");
    expect(llms).toContain("£298.99/mo; £2,869.99/year");
    expect(llms).toContain("55  default (remote, writes enabled)");
    expect(llms).toContain("11.9us p99 indexed reads");
    expect(llms).not.toMatch(/\b43\s+MCP/i);
    expect(llms).not.toMatch(/\b55\+\s+MCP/i);
    expect(llms).not.toMatch(/\$(?:19|79|299)\/mo/i);
  });

  it("rejects retired claims on canonical public answer surfaces", () => {
    const canonical = [
      "src/app/layout.tsx",
      "src/app/(marketing)/event-sourcing-for-ai-agents/page.tsx",
      "src/app/(marketing)/solutions/agent-memory/page.tsx",
      "src/app/(marketing)/solutions/real-time-analytics/page.tsx",
      "src/app/(marketing)/platform/prime/page.tsx",
      "src/lib/config.ts",
      "src/lib/structured-data.ts",
    ]
      .map(source)
      .join("\n");

    expect(canonical).not.toMatch(/sub-microsecond/i);
    expect(canonical).not.toMatch(/perfect memory/i);
    expect(canonical).not.toMatch(/\b(?:43|55\+)\s+MCP/i);
    expect(canonical).not.toMatch(/\$(?:19|79|299)\/month/i);
  });
});
