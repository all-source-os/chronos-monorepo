"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  Activity,
  ArrowRight,
  ChevronRight,
  Filter,
  GitMerge,
  Layers,
  Radio,
  Repeat,
  SplitSquareVertical,
  Workflow,
  Zap,
} from "lucide-react";
import Link from "next/link";
import { staticMotion as motion } from "@/components/ui/static-motion";

const operators = [
  {
    name: "Filter",
    icon: Filter,
    desc: "Drop events that don't match your criteria before they enter the pipeline",
  },
  {
    name: "Map",
    icon: ArrowRight,
    desc: "Transform event payloads — rename fields, compute derived values, enrich with context",
  },
  {
    name: "Reduce",
    icon: GitMerge,
    desc: "Aggregate events into running totals, counts, or custom accumulations over time windows",
  },
  {
    name: "Window",
    icon: SplitSquareVertical,
    desc: "Group events by time window (tumbling, sliding, session) for batch-style processing on a stream",
  },
  {
    name: "Branch",
    icon: Workflow,
    desc: "Route events to different downstream pipelines based on type, content, or custom predicates",
  },
  {
    name: "Enrich",
    icon: Layers,
    desc: "Join event data with external sources — add user profiles, geo data, or lookup tables in-flight",
  },
];

const features = [
  {
    title: "Projections",
    description:
      "Materialized views that stay in sync with your event stream. Define a projection as a fold over events — AllSource keeps it current as new events arrive.",
    icon: Activity,
    color: "from-blue-500/20 to-blue-500/5",
  },
  {
    title: "WebSocket Streaming",
    description:
      "Subscribe to live event feeds via Phoenix Channels. The Query Service pushes new events to connected clients in real-time — no polling.",
    icon: Radio,
    color: "from-green-500/20 to-green-500/5",
  },
  {
    title: "Event Replay",
    description:
      "Replay any sequence of events through a pipeline. Rebuild projections from scratch, test new pipeline logic against historical data, or debug by replaying the last hour.",
    icon: Repeat,
    color: "from-purple-500/20 to-purple-500/5",
  },
  {
    title: "469K Events/Sec",
    description:
      "The published Core batch-ingest reference reaches 469K events/sec on its stated hardware. Pipeline stages and durability settings change application throughput.",
    icon: Zap,
    color: "from-yellow-500/20 to-yellow-500/5",
  },
];

const pipelineExample = `// Define a pipeline: order events → compute daily revenue
{
  "name": "daily-revenue",
  "source": "order.*",
  "stages": [
    { "filter": { "event_type": "order.placed" } },
    { "map": { "extract": ["payload.total", "payload.currency"] } },
    { "window": { "type": "tumbling", "size": "1d" } },
    { "reduce": { "sum": "total", "count": "*", "group_by": "currency" } }
  ],
  "sink": "projection://daily-revenue-by-currency"
}`;

export default function StreamProcessingPage() {
  return (
    <div className="pt-24">
      {/* Hero */}
      <Section className="pb-8">
        <motion.div
          className="mx-auto max-w-3xl text-center"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6 }}
        >
          <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-primary/30 bg-primary/10 px-4 py-1.5 text-sm text-primary">
            <Activity className="h-4 w-4" />
            Platform
          </div>
          <h1 className="text-4xl font-bold tracking-tight sm:text-6xl">
            Process event streams inside AllSource
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            AllSource processes event streams inside the event store itself — filtering, mapping,
            reducing, windowing, branching, and enriching accepted events. The 469K events/sec
            figure is a Core batch-ingest reference, not a universal rate for every pipeline. No
            separate stream processor is required for these built-in stages.
          </p>
          <div className="mt-8 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
            <Link href="/signup" className={cn(buttonVariants({ variant: "default" }))}>
              Start 14-day trial <ChevronRight className="ml-1 h-4 w-4" />
            </Link>
            <Link href="/docs/api" className={cn(buttonVariants({ variant: "outline" }))}>
              Pipeline API docs
            </Link>
          </div>
        </motion.div>
      </Section>

      {/* Operators */}
      <Section
        title="Six operators for event pipelines"
        subtitle="Filter, map, reduce, enrich, route, and aggregate events as they arrive"
        className="py-16"
      >
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {operators.map((op, i) => (
            <motion.div
              key={op.name}
              initial={{ opacity: 0, y: 15 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.05 }}
              viewport={{ once: true }}
              className="rounded-xl border p-5"
            >
              <div className="mb-3 flex items-center gap-3">
                <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10">
                  <op.icon className="h-4 w-4 text-primary" />
                </div>
                <h3 className="font-semibold">{op.name}</h3>
              </div>
              <p className="text-sm text-muted-foreground">{op.desc}</p>
            </motion.div>
          ))}
        </div>
      </Section>

      {/* Pipeline JSON example */}
      <Section
        title="Define pipelines as JSON"
        subtitle="Declarative pipeline definitions that version-control alongside your code"
        className="py-16"
      >
        <motion.div
          className="mx-auto max-w-3xl overflow-hidden rounded-xl border bg-[#0c0c14]"
          initial={{ opacity: 0, scale: 0.98 }}
          whileInView={{ opacity: 1, scale: 1 }}
          viewport={{ once: true }}
        >
          <div className="flex items-center gap-2 border-b px-4 py-3">
            <div className="h-3 w-3 rounded-full bg-red-500" />
            <div className="h-3 w-3 rounded-full bg-yellow-500" />
            <div className="h-3 w-3 rounded-full bg-green-500" />
            <span className="ml-2 text-xs text-muted-foreground">daily-revenue.json</span>
          </div>
          <pre className="overflow-x-auto p-6 text-sm leading-relaxed text-gray-300">
            <code>{pipelineExample}</code>
          </pre>
        </motion.div>
      </Section>

      {/* Features */}
      <Section className="py-16">
        <div className="grid gap-6 sm:grid-cols-2">
          {features.map((feat, i) => (
            <motion.div
              key={feat.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.05 }}
              viewport={{ once: true }}
              className="rounded-xl border p-6"
            >
              <div
                className={cn(
                  "mb-4 flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br",
                  feat.color
                )}
              >
                <feat.icon className="h-5 w-5" />
              </div>
              <h3 className="mb-2 text-lg font-semibold">{feat.title}</h3>
              <p className="text-sm text-muted-foreground">{feat.description}</p>
            </motion.div>
          ))}
        </div>
      </Section>

      {/* CTA */}
      <Section className="py-16 text-center">
        <h2 className="text-3xl font-bold">Build your first event pipeline</h2>
        <p className="mx-auto mt-4 max-w-lg text-muted-foreground">
          Start with a 14-day trial — pipelines included. Pick a plan when it ends.
        </p>
        <div className="mt-8 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
          <Link href="/signup" className={cn(buttonVariants({ variant: "default", size: "lg" }))}>
            Start 14-day trial
          </Link>
          <Link
            href="/platform/event-sourcing"
            className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
          >
            Event sourcing platform
          </Link>
        </div>
      </Section>
    </div>
  );
}
