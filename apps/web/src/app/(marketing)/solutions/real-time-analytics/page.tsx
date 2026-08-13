"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  Activity,
  BarChart3,
  ChevronRight,
  Cpu,
  Database,
  Gauge,
  Layers,
  Radio,
  Zap,
} from "lucide-react";
import Link from "next/link";
import { staticMotion as motion } from "@/components/ui/static-motion";

const features = [
  {
    title: "11.9us p99 Query Latency",
    description:
      "DashMap concurrent reads deliver sub-microsecond indexed lookups. No query planner overhead, no connection pooling — just O(1) access to your event data.",
    icon: Zap,
    color: "from-cyan-500/20 to-cyan-500/5",
  },
  {
    title: "Projections & Materialized Views",
    description:
      "Define projections that automatically aggregate events into queryable views. Running totals, counters, session metrics — all updated in real-time as events arrive.",
    icon: Layers,
    color: "from-blue-500/20 to-blue-500/5",
  },
  {
    title: "WebSocket Live Streaming",
    description:
      "Subscribe to event streams over WebSocket. Dashboards update in real-time without polling. Filter by event type, entity, or custom predicates server-side.",
    icon: Radio,
    color: "from-indigo-500/20 to-indigo-500/5",
  },
  {
    title: "DashMap Concurrent Reads",
    description:
      "Lock-free concurrent hash map serves 40K+ queries per second. Multiple dashboard users querying simultaneously with no contention or degradation.",
    icon: Cpu,
    color: "from-teal-500/20 to-teal-500/5",
  },
  {
    title: "Parquet Columnar Storage",
    description:
      "Historical data compresses into Parquet files with Snappy compression. Columnar layout means analytical queries scan only the columns they need.",
    icon: Database,
    color: "from-sky-500/20 to-sky-500/5",
  },
  {
    title: "Prometheus Metrics Built-In",
    description:
      "Export event throughput, query latency percentiles, projection lag, and storage metrics directly to Prometheus. Grafana dashboards out of the box.",
    icon: Gauge,
    color: "from-violet-500/20 to-violet-500/5",
  },
];

export default function RealTimeAnalyticsPage() {
  return (
    <div className="relative overflow-hidden">
      {/* Hero */}
      <Section className="relative pt-24 pb-16 text-center">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6 }}
        >
          <span className="inline-flex items-center gap-2 rounded-full border bg-background/50 px-4 py-1.5 text-sm backdrop-blur-sm">
            <Activity className="h-4 w-4 text-cyan-400" />
            Real-Time Analytics
          </span>
          <h1 className="mt-6 text-4xl font-bold tracking-tight sm:text-6xl">
            Query live event streams and materialized views
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            Build projections and materialized views from ordered streams, then push new events to
            dashboards over WebSocket without a separate analytics copy.
          </p>
          <div className="mt-8 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
            <Link href="/signup" className={cn(buttonVariants({ size: "lg" }))}>
              Start 14-day trial
              <ChevronRight className="ml-1 h-4 w-4" />
            </Link>
            <Link
              href="/docs/api"
              className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
            >
              API reference
            </Link>
          </div>

          {/* Key Metrics */}
          <motion.div
            className="mx-auto mt-12 grid max-w-3xl grid-cols-2 gap-4 md:grid-cols-4"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.3 }}
          >
            {[
              { value: "11.9us", label: "p99 Latency" },
              { value: "469K/s", label: "Ingestion" },
              { value: "40K+", label: "Queries/sec" },
              { value: "0ms", label: "ETL Needed" },
            ].map((metric) => (
              <div
                key={metric.label}
                className="rounded-xl border bg-background/50 p-4 backdrop-blur-sm"
              >
                <div className="text-2xl font-bold text-cyan-400">{metric.value}</div>
                <div className="text-sm text-muted-foreground">{metric.label}</div>
              </div>
            ))}
          </motion.div>
        </motion.div>
      </Section>

      {/* Features */}
      <Section className="pb-16">
        <h2 className="mb-12 text-center text-3xl font-bold">
          From ingestion to insight in microseconds
        </h2>
        <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {features.map((feature, i) => (
            <motion.div
              key={feature.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.08 }}
              viewport={{ once: true }}
              className="rounded-xl border p-6"
            >
              <div
                className={cn(
                  "mb-4 flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br",
                  feature.color
                )}
              >
                <feature.icon className="h-5 w-5" />
              </div>
              <h3 className="mb-2 font-semibold">{feature.title}</h3>
              <p className="text-sm text-muted-foreground">{feature.description}</p>
            </motion.div>
          ))}
        </div>
      </Section>

      {/* Code Example */}
      <Section className="pb-16">
        <h2 className="mb-4 text-center text-3xl font-bold">Query events in real-time</h2>
        <p className="mb-8 text-center text-muted-foreground">
          Time-range queries, projections, and live streaming — all from one API
        </p>
        <div className="mx-auto max-w-3xl">
          <div className="overflow-hidden rounded-xl border">
            <div className="flex items-center gap-2 bg-neutral-900 px-4 py-3">
              <div className="h-3 w-3 rounded-full bg-red-500" />
              <div className="h-3 w-3 rounded-full bg-yellow-500" />
              <div className="h-3 w-3 rounded-full bg-green-500" />
              <span className="ml-4 font-mono text-sm text-neutral-400">analytics-query.sh</span>
            </div>
            <pre className="overflow-x-auto bg-neutral-950 p-6 text-sm leading-relaxed text-green-400">
              {`# Query events in a time range — returns in microseconds
curl -s https://api.all-source.xyz/api/v1/events/query \\
  -H "Authorization: Bearer $API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "event_type": "page_view",
    "start_time": "2026-04-16T00:00:00Z",
    "end_time": "2026-04-16T23:59:59Z",
    "limit": 1000
  }'

# {"events": [...], "count": 847}
# Response time: 0.012ms (11.9us p99)

# Get a live projection — materialized view updated on every event
curl -s https://api.all-source.xyz/api/v1/projections/daily-active-users \\
  -H "Authorization: Bearer $API_KEY"

# {"projection": {"name": "daily-active-users", "value": 12847, ...}}

# Stream events in real-time via WebSocket
wscat -c "wss://api.all-source.xyz/api/v1/events/stream" \\
  -H "Authorization: Bearer $API_KEY"
# > {"subscribe": {"event_type": "page_view"}}
# < {"event": {"type": "page_view", "data": {...}, "timestamp": "..."}}`}
            </pre>
          </div>
        </div>
      </Section>

      {/* CTA */}
      <Section className="pb-24 text-center">
        <BarChart3 className="mx-auto mb-4 h-12 w-12 text-cyan-400" />
        <h2 className="mb-4 text-3xl font-bold">Kill your ETL pipeline</h2>
        <p className="mx-auto mb-8 max-w-xl text-muted-foreground">
          Stop waiting for batch jobs. Query live event streams with sub-microsecond latency and
          stream updates to dashboards in real-time.
        </p>
        <div className="flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
          <Link href="/signup" className={cn(buttonVariants({ size: "lg" }))}>
            Start 14-day trial
            <ChevronRight className="ml-1 h-4 w-4" />
          </Link>
          <Link
            href="/compare/eventstoredb"
            className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
          >
            Compare to EventStoreDB
          </Link>
        </div>
        <div className="mt-6 flex items-center justify-center gap-6 text-sm text-muted-foreground">
          <Link href="/docs" className="underline">
            Documentation
          </Link>
          <Link href="/docs/api" className="underline">
            API Reference
          </Link>
          <Link href="https://github.com/all-source-os/all-source" className="underline">
            GitHub
          </Link>
        </div>
      </Section>
    </div>
  );
}
