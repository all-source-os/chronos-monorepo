import { Section } from "@allsource/ui";
import FAQ from "@/components/sections/faq";
import PricingSection from "@/components/sections/pricing";
import { siteConfig } from "@/lib/config";
import { fetchCatalog, indexByTier, resolveMonthly } from "@/lib/pricing-catalog";

// Revalidate the live LemonSqueezy prices hourly (ISR).
export const revalidate = 3600;

// Comparison matrix rows. Each row maps a label to a per-tier cell, keyed by the
// stable public tier id from siteConfig.pricing — Self-Host included as a column.
const matrixRows: { label: string; cells: Record<string, string> }[] = [
  {
    label: "Events / mo",
    cells: {
      "self-host": "Unlimited (your hw)",
      indie: "500K + 50K x402",
      studio: "5M + 500K x402",
      scale: "50M + 5M x402",
      enterprise: "Negotiated",
    },
  },
  {
    label: "Retention",
    cells: {
      "self-host": "Forever",
      indie: "14 days",
      studio: "90 days",
      scale: "365 days",
      enterprise: "Unlimited",
    },
  },
  {
    label: "MCP",
    cells: {
      "self-host": "Full (self-host)",
      indie: "read",
      studio: "read + write",
      scale: "read + write + dedicated",
      enterprise: "Dedicated cluster",
    },
  },
  {
    label: "Streams",
    cells: {
      "self-host": "Unlimited",
      indie: "3",
      studio: "Unlimited",
      scale: "Unlimited",
      enterprise: "Unlimited",
    },
  },
  {
    label: "Support",
    cells: {
      "self-host": "GitHub community",
      indie: "Email 48h",
      studio: "Email 24h + Discord",
      scale: "Priority + Slack",
      enterprise: "24/7 + dedicated SE",
    },
  },
];

export default async function PricingPage() {
  const tiers = siteConfig.pricing;
  const catalog = await fetchCatalog();
  const prices = indexByTier(catalog);

  return (
    <div className="mx-auto w-full max-w-screen-xl px-4 lg:px-8">
      {/* Above the fold: promise, toggle, cards, enterprise strip, x402 lines. */}
      <PricingSection catalog={catalog} />

      {/* Below the fold: comparison matrix with Self-Host as a column. */}
      <Section title="Compare tiers" subtitle="Everything, side by side">
        <div className="overflow-x-auto">
          <table className="w-full min-w-[720px] border-collapse text-sm">
            <thead>
              <tr className="border-b border-border">
                <th className="px-4 py-3 text-left font-medium text-muted-foreground">Feature</th>
                {tiers.map((tier) => (
                  <th key={tier.tier} className={cellHeaderClass(tier.isPopular)} scope="col">
                    {tier.name}
                    {tier.isPopular && (
                      <span className="ml-1 align-middle text-[10px] font-semibold text-primary">
                        Popular
                      </span>
                    )}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {matrixRows.map((row) => (
                <tr key={row.label} className="border-b border-border">
                  <td className="px-4 py-3 text-left font-medium text-foreground">{row.label}</td>
                  {tiers.map((tier) => (
                    <td key={tier.tier} className={cellBodyClass(tier.isPopular)}>
                      {row.cells[tier.tier] ?? "—"}
                    </td>
                  ))}
                </tr>
              ))}
              {/* Price row */}
              <tr>
                <td className="px-4 py-3 text-left font-medium text-foreground">Price</td>
                {tiers.map((tier) => {
                  // Live LemonSqueezy monthly price; paid tiers show a dash
                  // (not a config number) when no live/cached price.
                  const price = resolveMonthly(prices[tier.tier], tier.price);
                  return (
                    <td key={tier.tier} className={cellBodyClass(tier.isPopular)}>
                      {price}
                      {price.startsWith("$") && <span className="text-muted-foreground">/mo</span>}
                    </td>
                  );
                })}
              </tr>
            </tbody>
          </table>
        </div>
      </Section>

      {/* FAQ — includes the "Why no free plan?" entry + FAQPage JSON-LD. */}
      <FAQ />
    </div>
  );
}

function cellHeaderClass(isPopular: boolean) {
  return [
    "px-4 py-3 text-center font-semibold",
    isPopular ? "text-primary" : "text-foreground",
  ].join(" ");
}

function cellBodyClass(isPopular: boolean) {
  return [
    "px-4 py-3 text-center",
    isPopular ? "bg-primary/[0.04] text-foreground" : "text-muted-foreground",
  ].join(" ");
}
