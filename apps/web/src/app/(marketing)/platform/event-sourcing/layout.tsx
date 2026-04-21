import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Event Sourcing Platform — Immutable Logs with Time-Travel Queries",
  description:
    "Store every state change as an immutable event. Query any point in history with 11.9us latency. WAL + Parquet durability, 469K events/sec throughput. Open source.",
  canonical: "/platform/event-sourcing",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
