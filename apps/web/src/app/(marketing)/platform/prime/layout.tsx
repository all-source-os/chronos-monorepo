import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource Prime — Knowledge Graphs, Vector Search, and Agent Memory",
  description: "Add-on module for AllSource. Knowledge graphs, HNSW vector embeddings, compressed index, and 12us recall. Give your AI agents durable, cross-domain memory.",
  canonical: "/platform/prime",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
