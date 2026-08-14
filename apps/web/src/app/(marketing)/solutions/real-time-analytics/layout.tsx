import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource Query Service: HTTP, Realtime, and Analytics Reads",
  description:
    "See how AllSource Query Service separates tenant-scoped HTTP queries, Phoenix realtime channels, analytics endpoints, and rebuildable projections over Core.",
  canonical: "/solutions/real-time-analytics",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
