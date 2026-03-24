import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Quant Intelligence — AllSource",
  description:
    "High-performance temporal event store for quantitative trading. 469K events/sec, 11.9μs queries, time-travel across trading sessions.",
  canonical: "/solutions/quant-intelligence",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
