import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Stream Processing — Real-Time Pipelines with Filter, Map, and Reduce",
  description:
    "Filter, map, reduce, enrich, route, and aggregate event streams inside AllSource. Build projections and publish live updates over WebSocket.",
  canonical: "/platform/stream-processing",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
