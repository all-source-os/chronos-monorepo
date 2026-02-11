import Features from "@/components/features-horizontal";
import { Section } from "@allsource/ui";
import { Bot, GitBranch, History, Zap } from "lucide-react";

const data = [
  {
    id: 1,
    title: "469K Events/Second",
    content: "Rust-powered core with lock-free data structures. Ingest at scale without breaking a sweat.",
    image: "/dashboard.png",
    icon: <Zap className="h-6 w-6 text-primary" />,
  },
  {
    id: 2,
    title: "Time-Travel Queries",
    content: "Query any point in history. Reconstruct past states. Debug with complete temporal context.",
    image: "/dashboard.png",
    icon: <History className="h-6 w-6 text-primary" />,
  },
  {
    id: 3,
    title: "Stream Processing",
    content: "Filter, map, reduce, window, branch, enrich. Build real-time pipelines with a fluent API.",
    image: "/dashboard.png",
    icon: <GitBranch className="h-6 w-6 text-primary" />,
  },
  {
    id: 4,
    title: "27 MCP Tools",
    content: "AI-native from day one. Claude Desktop integration for autonomous event management.",
    image: "/dashboard.png",
    icon: <Bot className="h-6 w-6 text-primary" />,
  },
];

export default function Component() {
  return (
    <Section title="Features" subtitle="Built for performance. Designed for intelligence.">
      <Features collapseDelay={5000} linePosition="bottom" data={data} />
    </Section>
  );
}
