import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource vs EventStoreDB — Feature Comparison",
  description:
    "Side-by-side comparison of AllSource and EventStoreDB: self-serve pricing, 55+ tenant MCP tools, x402 payments, and published query benchmarks.",
  canonical: "/compare/eventstoredb",
});

export default function CompareLayout({ children }: { children: React.ReactNode }) {
  return children;
}
