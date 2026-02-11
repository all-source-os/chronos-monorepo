import Features from "@/components/features-vertical";
import { Section } from "@allsource/ui";
import { Database, Play, Sparkles } from "lucide-react";

const data = [
  {
    id: 1,
    title: "1. Connect Your Sources",
    content:
      "Point AllSource at your event streams. REST API, WebSocket, or direct SDK integration. Schema validation ensures data quality from the start.",
    image: "/dashboard.png",
    icon: <Database className="w-6 h-6 text-primary" />,
  },
  {
    id: 2,
    title: "2. Build Your Pipelines",
    content:
      "Define stream processing pipelines with our fluent API. Filter, transform, aggregate, and route events in real-time. No complex infrastructure to manage.",
    image: "/dashboard.png",
    icon: <Play className="w-6 h-6 text-primary" />,
  },
  {
    id: 3,
    title: "3. Query Through Time",
    content:
      "Access any point in your event history with sub-microsecond latency. Use our MCP tools to let AI agents analyze patterns, detect anomalies, and automate responses.",
    image: "/dashboard.png",
    icon: <Sparkles className="w-6 h-6 text-primary" />,
  },
];

export default function Component() {
  return (
    <Section title="How It Works" subtitle="From events to intelligence in three steps">
      <Features data={data} />
    </Section>
  );
}
