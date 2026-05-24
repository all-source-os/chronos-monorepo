import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Agent Memory: 5 Approaches Compared",
  description:
    "An honest comparison of five approaches to AI agent memory — platform memory, RAG / retrieval, file-based, database, and event-sourced. Where each wins, where each loses, and how to pick.",
  canonical: "/compare/agent-memory",
});

export default function CompareLayout({ children }: { children: React.ReactNode }) {
  return children;
}
