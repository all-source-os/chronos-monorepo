import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Agent Memory — AllSource Prime",
  description:
    "Give your AI agent persistent, cross-domain memory. Knowledge graph + vector search + compressed index in one binary. 12μs queries, $0/month.",
  canonical: "/solutions/agent-memory",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
