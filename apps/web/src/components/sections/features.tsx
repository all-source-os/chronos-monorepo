import { Section } from "@allsource/ui";
import { Bot, GitBranch, History, Zap } from "lucide-react";
import Features from "@/components/features-horizontal";

const data = [
  {
    id: 1,
    title: "Perfect Memory",
    content:
      "Never lose a change. Every event is immutable, versioned, and instantly queryable. Your application remembers everything.",
    video: "/assets/gif-live-stream.mp4",
    icon: <History className="h-6 w-6 text-primary" />,
  },
  {
    id: 2,
    title: "Temporal Queries",
    content:
      "Query any point in history with 11.9μs latency. Reconstruct past states, compare timelines, debug with complete context.",
    video: "/assets/gif-event-explorer.mp4",
    icon: <Zap className="h-6 w-6 text-primary" />,
  },
  {
    id: 3,
    title: "Event Replay",
    content:
      "Replay any sequence of events. Rebuild projections, test what-if scenarios, audit any decision path.",
    video: "/assets/gif-onboarding.mp4",
    icon: <GitBranch className="h-6 w-6 text-primary" />,
  },
  {
    id: 4,
    title: "AI Integration",
    content:
      "73 MCP tools for Claude and GPT. Your AI agents don't just store data—they understand your application's history.",
    video: "/assets/gif-api-key.mp4",
    icon: <Bot className="h-6 w-6 text-primary" />,
  },
];

export default function Component() {
  return (
    <Section
      title="Time-travel your data."
      subtitle="Not just storage—complete memory for your applications"
    >
      <Features collapseDelay={5000} linePosition="bottom" data={data} />
    </Section>
  );
}
