import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Stream Processing — Real-Time Pipelines with Filter, Map, and Reduce",
  description:
    "Build real-time event pipelines that filter, transform, and route events at 469K/sec. Projections, materialized views, and WebSocket streaming. Rust-powered, zero external deps.",
  canonical: "/platform/stream-processing",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
