import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Event Sourcing Platform — Immutable Logs with Time-Travel Queries",
  description:
    "Store accepted state changes as immutable events. Reconstruct historical state with WAL + Parquet persistence. Published Core references: 469K events/sec batch ingest and 11.9us p99 indexed reads.",
  canonical: "/platform/event-sourcing",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
