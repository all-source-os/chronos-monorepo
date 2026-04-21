import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource vs EventStoreDB — Feature Comparison",
  description:
    "Side-by-side comparison of AllSource and EventStoreDB. Self-serve pricing, 43 MCP tools, x402 agent payments, and sub-microsecond queries vs cluster-priced infrastructure.",
  canonical: "/compare/eventstoredb",
});

export default function CompareLayout({ children }: { children: React.ReactNode }) {
  return children;
}
