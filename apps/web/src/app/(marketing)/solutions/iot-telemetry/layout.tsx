import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "IoT & Telemetry — High-Throughput Ingestion for Sensor Data",
  description:
    "Ingest 469K sensor events per second with WAL durability. Time-series queries, anomaly detection projections, and real-time dashboards for industrial IoT and device fleets.",
  canonical: "/solutions/iot-telemetry",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
