import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Real-Time Analytics — Sub-Microsecond Queries on Live Event Streams",
  description:
    "Query event streams with 11.9us p99 latency. Projections, materialized views, and WebSocket streaming for dashboards that update in real-time. No ETL pipeline needed.",
  canonical: "/solutions/real-time-analytics",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
