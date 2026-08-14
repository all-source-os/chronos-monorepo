import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource Prime — Knowledge Graphs, Vector Search, and Agent Memory",
  description:
    "AllSource Prime adds knowledge graphs, HNSW vector search, compressed-index context, temporal recall, and provenance over durable Core events.",
  canonical: "/platform/prime",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
