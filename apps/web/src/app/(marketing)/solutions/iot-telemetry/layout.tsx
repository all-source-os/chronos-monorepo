import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "IoT & Telemetry — High-Throughput Ingestion for Sensor Data",
  description:
    "Durable sensor-event ingestion, time-range queries, projections, and dashboards for industrial IoT. Published Core batch-ingest reference: 469K events/sec.",
  canonical: "/solutions/iot-telemetry",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
