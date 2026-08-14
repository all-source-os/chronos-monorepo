import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Quant Intelligence — AllSource",
  description:
    "Store and replay ordered market events, query point-in-time state, and build probability models over durable history. Includes qualified Core reference benchmarks.",
  canonical: "/solutions/quant-intelligence",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
