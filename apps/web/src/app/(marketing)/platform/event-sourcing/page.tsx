"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  ChevronRight,
  Clock,
  Database,
  FileCheck,
  GitBranch,
  History,
  Lock,
  Search,
  Shield,
  Zap,
} from "lucide-react";
import { motion } from "motion/react";
import Link from "next/link";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

const features = [
  {
    title: "Immutable Event Log",
    description:
      "Every state change is an append-only event with CRC32 checksums. No silent updates, no lost writes. The complete provenance of every record is preserved forever.",
    icon: Lock,
    color: "from-blue-500/20 to-blue-500/5",
  },
  {
    title: "Time-Travel Queries",
    description:
      "Reconstruct the state of any entity at any historical timestamp with 11.9us p99 latency. Debug production issues by replaying exactly what happened.",
    icon: History,
    color: "from-purple-500/20 to-purple-500/5",
  },
  {
    title: "WAL + Parquet Durability",
    description:
      "Write-Ahead Log with configurable fsync ensures no event is ever lost. Parquet columnar storage with Snappy compression gives you fast analytical reads and compact storage.",
    icon: Shield,
    color: "from-green-500/20 to-green-500/5",
  },
  {
    title: "DashMap Concurrent Reads",
    description:
      "In-memory concurrent hash map delivers 469K events/sec ingestion and sub-microsecond reads. Lock-free architecture means reads never block writes.",
    icon: Zap,
    color: "from-yellow-500/20 to-yellow-500/5",
  },
  {
    title: "Projections & Snapshots",
    description:
      "Build materialized views that stay in sync with your event stream. Checkpoint snapshots periodically to keep recovery fast as your event volume grows.",
    icon: GitBranch,
    color: "from-cyan-500/20 to-cyan-500/5",
  },
  {
    title: "Schema Governance",
    description:
      "Register and validate event schemas before they enter the store. Catch breaking changes at ingest time, not in production at 3 AM.",
    icon: FileCheck,
    color: "from-orange-500/20 to-orange-500/5",
  },
];

const codeExample = `# Ingest an event
curl -X POST https://api.all-source.xyz/api/v1/events \\
  -H "Authorization: Bearer $API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "event_type": "order.placed",
    "entity_id": "order-123",
    "payload": { "total": 149.99, "items": 3 }
  }'

# Time-travel query: what was this order's state yesterday?
curl "https://api.all-source.xyz/api/v1/events/query?\\
  entity_id=order-123&before=2026-04-19T00:00:00Z" \\
  -H "Authorization: Bearer $API_KEY"`;

export default function EventSourcingPage() {
  return (
    <>
      <Header />
      <main className="pt-24">
        {/* Hero */}
        <Section className="pb-8">
          <motion.div
            className="mx-auto max-w-3xl text-center"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
          >
            <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-primary/30 bg-primary/10 px-4 py-1.5 text-sm text-primary">
              <Database className="h-4 w-4" />
              Platform
            </div>
            <h1 className="text-4xl font-bold tracking-tight sm:text-6xl">
              Event sourcing that
              <br />
              <span className="bg-gradient-to-r from-blue-400 to-cyan-400 bg-clip-text text-transparent">
                remembers everything
              </span>
            </h1>
            <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
              AllSource stores every state change as an immutable event. Query any point in
              history with sub-microsecond latency. No event is ever lost, overwritten, or
              silently mutated.
            </p>
            <div className="mt-8 flex items-center justify-center gap-4">
              <Link href="/signup" className={cn(buttonVariants({ variant: "default" }))}>
                Start free <ChevronRight className="ml-1 h-4 w-4" />
              </Link>
              <Link href="/docs/api" className={cn(buttonVariants({ variant: "outline" }))}>
                API docs
              </Link>
            </div>
          </motion.div>
        </Section>

        {/* Key metrics */}
        <Section className="py-12">
          <div className="mx-auto grid max-w-4xl grid-cols-2 gap-8 sm:grid-cols-4 text-center">
            {[
              { value: "469K", label: "events/sec", sub: "ingestion throughput" },
              { value: "11.9μs", label: "p99 latency", sub: "query response" },
              { value: "0", label: "events lost", sub: "WAL + Parquet durability" },
              { value: "43", label: "MCP tools", sub: "AI agent integration" },
            ].map((stat) => (
              <motion.div
                key={stat.label}
                initial={{ opacity: 0, y: 10 }}
                whileInView={{ opacity: 1, y: 0 }}
                viewport={{ once: true }}
              >
                <div className="text-3xl font-bold text-primary">{stat.value}</div>
                <div className="text-sm font-medium text-foreground">{stat.label}</div>
                <div className="text-xs text-muted-foreground">{stat.sub}</div>
              </motion.div>
            ))}
          </div>
        </Section>

        {/* Features grid */}
        <Section
          title="Built for systems that can't afford to forget"
          subtitle="Every feature exists to ensure your events are durable, queryable, and correct"
          className="py-16"
        >
          <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
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

        {/* Code example */}
        <Section title="Two API calls. That's it." subtitle="Ingest events and query history with plain HTTP" className="py-16">
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
              <span className="ml-2 text-xs text-muted-foreground">Terminal</span>
            </div>
            <pre className="overflow-x-auto p-6 text-sm leading-relaxed text-gray-300">
              <code>{codeExample}</code>
            </pre>
          </motion.div>
        </Section>

        {/* CTA */}
        <Section className="py-16 text-center">
          <h2 className="text-3xl font-bold">Start capturing events in 60 seconds</h2>
          <p className="mx-auto mt-4 max-w-lg text-muted-foreground">
            Free tier: 100K events/month. No credit card required. Open source, self-host or cloud.
          </p>
          <div className="mt-8 flex items-center justify-center gap-4">
            <Link href="/signup" className={cn(buttonVariants({ variant: "default", size: "lg" }))}>
              Get started free
            </Link>
            <Link href="/compare/eventstoredb" className={cn(buttonVariants({ variant: "outline", size: "lg" }))}>
              Compare with EventStoreDB
            </Link>
          </div>
        </Section>
      </main>
      <Footer />
    </>
  );
}
