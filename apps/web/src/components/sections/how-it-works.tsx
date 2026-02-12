import { Section } from "@allsource/ui";
import { Database, Play, Sparkles } from "lucide-react";
import Features from "@/components/features-vertical";

const data = [
  {
    id: 1,
    title: "1. Capture Everything",
    content:
      "Send events via REST, WebSocket, or SDK. Every event is immutably stored with automatic schema validation. Nothing is ever lost or overwritten.",
    image: "/dashboard.png",
    icon: <Database className="w-6 h-6 text-primary" />,
  },
  {
    id: 2,
    title: "2. Process in Real-Time",
    content:
      "Build pipelines that filter, transform, and route events as they arrive. Create projections and materialized views that stay in sync automatically.",
    image: "/dashboard.png",
    icon: <Play className="w-6 h-6 text-primary" />,
  },
  {
    id: 3,
    title: "3. Query Any Point in Time",
    content:
      "Reconstruct state at any timestamp. Replay sequences for debugging. Let AI agents analyze patterns across your entire history with 27 MCP tools.",
    image: "/dashboard.png",
    icon: <Sparkles className="w-6 h-6 text-primary" />,
  },
];

export default function Component() {
  return (
    <Section title="How It Works" subtitle="From events to perfect memory in three steps">
      <Features data={data} />
    </Section>
  );
}
