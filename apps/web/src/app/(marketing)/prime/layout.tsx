import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource Prime — Memory for Claude, where your agents already work",
  description:
    "Install in 30 seconds. AllSource Prime is the AI-native memory layer for Claude Desktop, Claude Code, and any MCP client. Compressed-index auto-injection, hybrid recall, in-process embeddings — no separate CMS to babysit.",
  canonical: "/prime",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
