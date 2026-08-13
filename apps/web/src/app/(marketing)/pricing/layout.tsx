import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Pricing — pay for the events your agents write",
  description:
    "Self-host AllSource free (Apache-2.0), or go hosted from $19/mo. Indie, Studio, and Scale tiers ship metered x402 micropayment credits and explicit MCP read/write access. Enterprise is custom.",
  canonical: "/pricing",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
