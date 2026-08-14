import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Agent Memory — AllSource Prime",
  description:
    "Give AI agents durable, cross-domain memory with graph, vector, compressed-index, and temporal recall. Run Prime locally or use tenant-scoped hosted Core persistence.",
  canonical: "/solutions/agent-memory",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
