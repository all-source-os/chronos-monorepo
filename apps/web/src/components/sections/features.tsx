import { Card, CardContent, Section } from "@allsource/ui";
import { Bot, GitBranch, History, Network, Rows3, Workflow } from "lucide-react";
import Link from "next/link";

const capabilities = [
  {
    title: "Event Timeline",
    content:
      "Read an entity's complete history in order, inspect each payload, and trace which event caused a state change.",
    icon: Rows3,
    href: "/examples#event-timeline",
  },
  {
    title: "Time travel",
    content:
      "Ask what a stream contained at a sequence or timestamp and reconstruct the matching state.",
    icon: History,
    href: "/examples#time-travel",
  },
  {
    title: "Graph visualisation",
    content:
      "Explore Prime nodes and relationships with source-event provenance instead of an opaque knowledge graph.",
    icon: Network,
    href: "/examples#graph-visualisation",
  },
  {
    title: "Stream pipelines",
    content:
      "Filter, map, reduce, window, or branch accepted events inline while preserving source history.",
    icon: Workflow,
    href: "/examples#pipelines",
  },
  {
    title: "Rebuildable projections",
    content:
      "Fold existing events into tenant-scoped read models for HTTP, realtime, and analytics consumers.",
    icon: GitBranch,
    href: "/examples#projections",
  },
  {
    title: "MCP access for agents",
    content:
      "Let MCP clients query timelines, reconstruct state, and inspect events through explicit tenant-scoped tools.",
    icon: Bot,
    href: "/examples#mcp-data-access",
  },
];

export default function Features() {
  return (
    <Section
      title="What you can do with stored history"
      subtitle="Core capabilities"
      description="Each capability uses the same append-only event model, whether AllSource runs hosted or on your infrastructure."
    >
      <div className="grid gap-px border border-border bg-border md:grid-cols-2 lg:grid-cols-3">
        {capabilities.map((capability) => (
          <Link key={capability.title} href={capability.href} className="group bg-background">
            <Card className="h-full rounded-none border-0 bg-card shadow-none transition-colors group-hover:bg-muted/30">
              <CardContent className="flex gap-4 p-6">
                <div className="flex h-11 w-11 shrink-0 items-center justify-center border border-border bg-background transition-colors group-hover:border-primary/50">
                  <capability.icon className="h-5 w-5 text-primary" aria-hidden="true" />
                </div>
                <div>
                  <h3 className="text-lg font-semibold">{capability.title}</h3>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">
                    {capability.content}
                  </p>
                  <span className="mt-4 inline-flex items-center font-mono text-xs font-semibold text-primary">
                    Open demo →
                  </span>
                </div>
              </CardContent>
            </Card>
          </Link>
        ))}
      </div>
    </Section>
  );
}
