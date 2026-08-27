"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  Activity,
  BarChart3,
  ChevronRight,
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
    title: "Tenant-Scoped HTTP Reads",
    description:
      "Serve event queries, stream discovery, schemas, projection state, and replay jobs through authenticated request-response endpoints.",
    icon: Zap,
    color: "from-cyan-500/20 to-cyan-500/5",
  },
  {
    title: "Rebuildable Read Models",
    description:
      "Fold each tenant's Core stream into current-state projections. Rebuild them from durable history when projection logic changes.",
    icon: Layers,
    color: "from-blue-500/20 to-blue-500/5",
  },
  {
    title: "Phoenix Realtime Channels",
    description:
      "Subscribe at `/ws` to tenant-scoped event, entity, event-type, or projection topics. Push live updates without polling.",
    icon: Radio,
    color: "from-indigo-500/20 to-indigo-500/5",
  },
  {
    title: "Analytics Endpoints",
    description:
      "Expose frequency, summary, correlation, percentile, standard-deviation, sliding-window, and session-window reads with query quotas.",
    icon: BarChart3,
    color: "from-teal-500/20 to-teal-500/5",
  },
  {
    title: "One Durable Core",
    description:
      "Query Service does not become another database. Core retains events and metadata; Query Service caches and read models remain disposable.",
    icon: Database,
    color: "from-sky-500/20 to-sky-500/5",
  },
  {
    title: "Read-Plane Observability",
    description:
      "Inspect HTTP performance, WebSocket connections, projection replay lag, analytics cache state, and backend health.",
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
            Query Service read plane
          </span>
          <h1 className="mt-6 text-4xl font-bold tracking-tight sm:text-6xl">
            Four read shapes. One source history.
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            AllSource Query Service separates tenant-scoped HTTP queries, Phoenix realtime channels,
            analytics endpoints, and rebuildable projections over durable Core events.
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
              { value: "HTTP", label: "Request reads" },
              { value: "/ws", label: "Live channels" },
              { value: "7", label: "Analytics routes" },
              { value: "Core", label: "Durable source" },
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
        <h2 className="mb-12 text-center text-3xl font-bold">Match read path to consumer</h2>
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
        <h2 className="mb-4 text-center text-3xl font-bold">Three interfaces, one source</h2>
        <p className="mb-8 text-center text-muted-foreground">
          Query Service scopes every read to a tenant while Core remains source of truth
        </p>
        <div className="mx-auto max-w-3xl">
          <div className="overflow-hidden rounded-xl border">
            <div className="flex items-center gap-2 bg-neutral-900 px-4 py-3">
              <div className="h-3 w-3 rounded-full bg-red-500" />
              <div className="h-3 w-3 rounded-full bg-yellow-500" />
              <div className="h-3 w-3 rounded-full bg-green-500" />
              <span className="ml-4 font-mono text-sm text-neutral-400">
                query-service-paths.txt
              </span>
            </div>
            <pre className="overflow-x-auto bg-neutral-950 p-6 text-sm leading-relaxed text-green-400">
              {`# 1. HTTP — request-response event query
curl -s "https://your-query-service.example.com/api/events/query?event_type=page_view&limit=100" \\
  -H "Authorization: Bearer $API_KEY" \\
  -H "Authorization: Bearer $API_KEY"

# 2. Analytics — cached, quota-aware aggregation
curl -s "https://your-query-service.example.com/api/analytics/summary" \\
  -H "Authorization: Bearer $API_KEY"

# 3. Realtime — Phoenix Channel at /ws
const socket = new Socket("wss://your-query-service.example.com/ws", {
  params: { token: authToken }
});
socket.channel("events:all", {}).join();`}
            </pre>
          </div>
        </div>
      </Section>

      {/* CTA */}
      <Section className="pb-24 text-center">
        <BarChart3 className="mx-auto mb-4 h-12 w-12 text-cyan-400" />
        <h2 className="mb-4 text-3xl font-bold">Choose read path, not another database</h2>
        <p className="mx-auto mb-8 max-w-xl text-muted-foreground">
          Use HTTP for request-response reads, Phoenix Channels for live delivery, analytics routes
          for aggregations, and projections for current state. Every path derives from Core.
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
