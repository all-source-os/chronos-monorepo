"use client";

import { buttonVariants, cn, FlickeringGrid, Section } from "@allsource/ui";
import {
  Brain,
  Check,
  ChevronRight,
  Clock,
  Code2,
  Database,
  GitBranch,
  Layers,
  RefreshCw,
  Server,
  Users,
  Zap,
} from "lucide-react";
import Link from "next/link";
import { staticMotion as motion } from "@/components/ui/static-motion";
import { faqPageSchema } from "@/lib/structured-data";

const capabilities = [
  {
    question: "Where does the data live?",
    answer:
      "Accepted events are recoverable from a CRC32-checked write-ahead log and Snappy-compressed Parquet files. Entity IDs map to 32 partitions by default; the partition count is configurable.",
    icon: Database,
    details: [
      "By Symbol: entity_id = 'NQ' or 'BTC'",
      "By Session: event_type tags for RTH/ETH",
      "By Date: Microsecond timestamp precision",
    ],
    metric: "32 partitions",
    color: "from-blue-500/20 to-blue-500/5",
  },
  {
    question: "How fast is time slicing?",
    answer:
      "The published Core reference benchmark measured 11.9μs p99 indexed reads. Bar generation and range slicing scan or aggregate more data, so benchmark them with your event shape and hardware.",
    icon: Zap,
    details: [
      "Indexed-read reference: 11.9μs p99",
      "Batch-ingestion reference: 469K events/sec",
      "No published end-to-end bar benchmark",
    ],
    metric: "11.9μs p99",
    color: "from-yellow-500/20 to-yellow-500/5",
  },
  {
    question: "How are corrections handled?",
    answer:
      "Immutable append-only event sourcing with correction events. Full audit trails with as_of temporal queries.",
    icon: RefreshCw,
    details: [
      "Original events are never modified",
      "Corrections appended with references",
      "as_of queries show pre-correction state",
    ],
    metric: "Append-only corrections",
    color: "from-green-500/20 to-green-500/5",
  },
  {
    question: "Can past analysis be reproduced?",
    answer:
      "AllSource preserves the ordered inputs needed for replay and point-in-time reconstruction. Reproducible analysis also requires deterministic projection code, versioned models, and captured configuration.",
    icon: Clock,
    details: [
      "Snapshots: Every 100 events or 1 hour",
      "as_of queries: Point-in-time state",
      "Event Replay: Full deterministic replay",
    ],
    metric: "Replayable inputs",
    color: "from-purple-500/20 to-purple-500/5",
  },
  {
    question: "How easy is Python integration?",
    answer:
      "Use the HTTP API from any Python client or the Python SDK in the AllSource repository. JSON responses can be loaded into pandas; WebSocket streaming supports live updates.",
    icon: Code2,
    details: [
      "REST API: GET /api/v1/events/query",
      "WebSocket: WS /api/v1/events/stream",
      "Python client source in sdks/python-client",
    ],
    metric: "Multi-platform",
    color: "from-orange-500/20 to-orange-500/5",
  },
  {
    question: "Does it support concurrent users?",
    answer:
      "Core uses a sharded concurrent map for hot reads. Capacity depends on filters, payload size, API hops, cache state, and hardware; load-test your end-to-end route before setting an SLO.",
    icon: Users,
    details: [
      "Concurrent in-memory read path",
      "Prometheus latency and throughput metrics",
      "No universal hosted QPS claim",
    ],
    metric: "Measure your route",
    color: "from-pink-500/20 to-pink-500/5",
  },
];

const apiEndpoints = [
  {
    method: "GET",
    endpoint: "/api/v1/analytics/frequency",
    description: "Event frequency bucketed by time window",
  },
  {
    method: "GET",
    endpoint: "/api/v1/analytics/summary",
    description: "Statistical summary of events",
  },
  {
    method: "GET",
    endpoint: "/api/v1/analytics/correlation",
    description: "Event correlation analysis",
  },
];

const architectureLayers = [
  {
    name: "Client Layer",
    icon: Layers,
    items: ["Web App (React)", "Python SDK", "AI Query Engine"],
    color: "bg-blue-500",
  },
  {
    name: "API Layer",
    icon: Server,
    items: ["REST API", "WebSocket Streaming", "Analytics Endpoints"],
    color: "bg-green-500",
  },
  {
    name: "Processing Layer",
    icon: GitBranch,
    items: ["Rust Core (469K/s)", "Elixir Query Service", "Go Control Plane"],
    color: "bg-orange-500",
  },
  {
    name: "Storage Layer",
    icon: Database,
    items: ["Parquet Columnar", "WAL Durability", "Point-in-Time Snapshots"],
    color: "bg-purple-500",
  },
];

export default function QuantIntelligencePage() {
  // Built from the SAME `capabilities` array rendered below, so the schema can
  // never claim an answer the page does not show.
  const faqJsonLd = faqPageSchema(
    capabilities.map((cap) => ({ question: cap.question, answer: cap.answer }))
  );

  return (
    <div className="min-h-screen">
      <script
        type="application/ld+json"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD structured data requires dangerouslySetInnerHTML
        dangerouslySetInnerHTML={{ __html: JSON.stringify(faqJsonLd) }}
      />

      {/* Hero */}
      <section className="relative overflow-hidden border-b bg-gradient-to-b from-background to-neutral-100 px-4 pb-20 pt-32 dark:to-neutral-900">
        <FlickeringGrid
          className="absolute inset-0 z-0 [mask:radial-gradient(ellipse_at_top,#fff_400px,transparent_100%)]"
          squareSize={4}
          gridGap={6}
          color="#f97316"
          maxOpacity={0.1}
          flickerChance={0.1}
          height={600}
          width={2000}
        />
        <div className="container relative z-10 mx-auto max-w-5xl text-center">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
          >
            <span className="inline-flex items-center gap-2 rounded-full border bg-background/50 px-4 py-1.5 text-sm backdrop-blur-sm">
              <Brain className="h-4 w-4 text-primary" />
              Market event analysis
            </span>
          </motion.div>

          <motion.h1
            className="mt-6 text-4xl font-bold tracking-tight md:text-6xl"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.1 }}
          >
            Probability estimates from market event data
          </motion.h1>

          <motion.p
            className="mx-auto mt-4 max-w-2xl text-lg text-muted-foreground"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.2 }}
          >
            Store price and signal events, compute calibrated probability views, and query those
            results through dashboards, APIs, or an AI client.
          </motion.p>

          <motion.div
            className="mt-8 flex flex-wrap items-center justify-center gap-4"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.3 }}
          >
            <Link href="/signup" className={buttonVariants({ variant: "default", size: "lg" })}>
              Get Early Access
              <ChevronRight className="ml-1 h-4 w-4" />
            </Link>
            <Link
              href="#capabilities"
              className={buttonVariants({ variant: "outline", size: "lg" })}
            >
              View Capabilities
            </Link>
          </motion.div>

          {/* Key Metrics */}
          <motion.div
            className="mt-12 grid grid-cols-2 gap-4 md:grid-cols-4"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.4 }}
          >
            {[
              { value: "11.9μs", label: "Core indexed read p99" },
              { value: "469K/s", label: "Batch ingest reference" },
              { value: "32", label: "Default partitions" },
              { value: "3", label: "Analytics endpoints" },
            ].map((metric) => (
              <div
                key={metric.label}
                className="rounded-xl border bg-background/50 p-4 backdrop-blur-sm"
              >
                <div className="text-2xl font-bold text-primary">{metric.value}</div>
                <div className="text-sm text-muted-foreground">{metric.label}</div>
              </div>
            ))}
          </motion.div>
        </div>
      </section>

      {/* Architecture Overview */}
      <Section
        title="Architecture"
        subtitle="Built for Performance & Scale"
        className="bg-background"
      >
        <div className="mx-auto mt-12 max-w-4xl">
          <div className="grid gap-4 md:grid-cols-2">
            {architectureLayers.map((layer, index) => (
              <motion.div
                key={layer.name}
                className="rounded-2xl border bg-background p-6"
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.4, delay: index * 0.1 }}
                viewport={{ once: true }}
              >
                <div className="flex items-center gap-3">
                  <div className={cn("rounded-lg p-2", layer.color)}>
                    <layer.icon className="h-5 w-5 text-white" />
                  </div>
                  <h3 className="font-semibold">{layer.name}</h3>
                </div>
                <ul className="mt-4 space-y-2">
                  {layer.items.map((item) => (
                    <li
                      key={item}
                      className="flex items-center gap-2 text-sm text-muted-foreground"
                    >
                      <Check className="h-4 w-4 text-primary" />
                      {item}
                    </li>
                  ))}
                </ul>
              </motion.div>
            ))}
          </div>
        </div>
      </Section>

      {/* Capabilities FAQ */}
      <Section
        id="capabilities"
        title="Technical capabilities"
        subtitle="What Core provides, and where your analysis begins"
        className="bg-neutral-100 dark:bg-neutral-900"
      >
        <div className="mx-auto mt-12 max-w-5xl space-y-6">
          {capabilities.map((cap, index) => (
            <motion.div
              key={cap.question}
              className={cn("overflow-hidden rounded-2xl border bg-gradient-to-r p-6", cap.color)}
              initial={{ opacity: 0, x: index % 2 === 0 ? -20 : 20 }}
              whileInView={{ opacity: 1, x: 0 }}
              transition={{ duration: 0.4, delay: index * 0.1 }}
              viewport={{ once: true }}
            >
              <div className="flex flex-col gap-6 md:flex-row md:items-start">
                <div className="flex-shrink-0">
                  <div className="rounded-xl bg-background p-3">
                    <cap.icon className="h-6 w-6 text-primary" />
                  </div>
                </div>
                <div className="flex-grow">
                  <div className="flex items-center justify-between">
                    <h3 className="text-lg font-semibold">{cap.question}</h3>
                    <span className="rounded-full bg-primary/10 px-3 py-1 text-xs font-medium text-primary">
                      {cap.metric}
                    </span>
                  </div>
                  <p className="mt-2 text-muted-foreground">{cap.answer}</p>
                  <ul className="mt-4 grid gap-2 md:grid-cols-3">
                    {cap.details.map((detail) => (
                      <li key={detail} className="flex items-center gap-2 text-sm">
                        <Check className="h-4 w-4 flex-shrink-0 text-primary" />
                        <span className="text-foreground">{detail}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      </Section>

      {/* API Endpoints */}
      <Section
        title="Available analytics API"
        subtitle="Current endpoints in Core"
        className="bg-background"
      >
        <div className="mx-auto mt-12 max-w-4xl">
          <div className="overflow-hidden rounded-2xl border">
            <div className="bg-neutral-900 p-4">
              <div className="flex items-center gap-2">
                <div className="h-3 w-3 rounded-full bg-red-500" />
                <div className="h-3 w-3 rounded-full bg-yellow-500" />
                <div className="h-3 w-3 rounded-full bg-green-500" />
                <span className="ml-4 text-sm text-neutral-400">api-reference.ts</span>
              </div>
            </div>
            <div className="bg-neutral-950 p-6">
              <div className="space-y-3 font-mono text-sm">
                {apiEndpoints.map((endpoint, index) => (
                  <motion.div
                    key={endpoint.endpoint}
                    className="flex flex-wrap items-center gap-2"
                    initial={{ opacity: 0, x: -10 }}
                    whileInView={{ opacity: 1, x: 0 }}
                    transition={{ duration: 0.3, delay: index * 0.05 }}
                    viewport={{ once: true }}
                  >
                    <span
                      className={cn(
                        "rounded px-2 py-0.5 text-xs font-bold",
                        endpoint.method === "GET"
                          ? "bg-green-500/20 text-green-400"
                          : "bg-blue-500/20 text-blue-400"
                      )}
                    >
                      {endpoint.method}
                    </span>
                    <span className="text-white">{endpoint.endpoint}</span>
                    <span className="text-neutral-500">{`// ${endpoint.description}`}</span>
                  </motion.div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </Section>

      <Section
        title="Product boundary"
        subtitle="AllSource stores evidence; your model produces probabilities"
        className="bg-neutral-100 dark:bg-neutral-900"
      >
        <div className="mx-auto mt-12 grid max-w-4xl gap-6 md:grid-cols-3">
          {[
            {
              title: "Available now",
              body: "Event ingestion, time-range queries, replay, snapshots, WebSocket streams, and three analytics endpoints.",
            },
            {
              title: "You provide",
              body: "Market-data licensing, bar construction, feature definitions, probability models, calibration, and validation.",
            },
            {
              title: "Not claimed here",
              body: "No built-in trading strategy, return guarantee, or shipped natural-language quant endpoint is implied.",
            },
          ].map((item) => (
            <div key={item.title} className="rounded-2xl border bg-background p-6">
              <h3 className="font-semibold">{item.title}</h3>
              <p className="mt-2 text-sm text-muted-foreground">{item.body}</p>
            </div>
          ))}
        </div>
      </Section>

      {/* CTA */}
      <section className="border-t bg-gradient-to-b from-background to-neutral-100 px-4 py-20 dark:to-neutral-900">
        <div className="container mx-auto max-w-3xl text-center">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
            viewport={{ once: true }}
          >
            <h2 className="text-3xl font-bold md:text-4xl">Build on durable market events</h2>
            <p className="mx-auto mt-4 max-w-xl text-muted-foreground">
              Use AllSource for ordered history, replay, and current analytics endpoints. Keep your
              probability model explicit and independently validated.
            </p>
            <div className="mt-8 flex flex-wrap items-center justify-center gap-4">
              <Link href="/signup" className={buttonVariants({ variant: "default", size: "lg" })}>
                Start 14-day trial
                <ChevronRight className="ml-1 h-4 w-4" />
              </Link>
              <Link href="/#pricing" className={buttonVariants({ variant: "outline", size: "lg" })}>
                View Pricing
              </Link>
            </div>
          </motion.div>
        </div>
      </section>
    </div>
  );
}
