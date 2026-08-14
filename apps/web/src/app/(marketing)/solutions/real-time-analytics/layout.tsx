import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Real-Time Analytics — Live Event Streams and Materialized Views",
  description:
    "Build event-stream dashboards with projections, materialized views, and WebSocket delivery. Published Core indexed-read reference: 11.9us p99; end-to-end latency varies by path.",
  canonical: "/solutions/real-time-analytics",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
