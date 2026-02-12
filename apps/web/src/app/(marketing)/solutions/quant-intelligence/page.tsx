"use client";

import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";
import { buttonVariants, cn, FlickeringGrid, Ripple, Section } from "@allsource/ui";
import { motion } from "motion/react";
import Link from "next/link";
import {
  BarChart3,
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

const capabilities = [
  {
    question: "Where does the data live?",
    answer: "Apache Parquet files with SNAPPY compression, organized into 32 fixed partitions using consistent hashing.",
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
    answer: "11.9μs indexed lookups with O(1) complexity. Generate 1-minute bars for a full trading session in under 10ms.",
    icon: Zap,
    details: [
      "Symbol lookup: 11.9μs (indexed)",
      "1-minute bars (6.5 hrs): < 10ms",
      "Time range slice (1M events): < 5ms",
    ],
    metric: "11.9μs p99",
    color: "from-yellow-500/20 to-yellow-500/5",
  },
  {
    question: "How are corrections handled?",
    answer: "Immutable append-only event sourcing with correction events. Full audit trails with as_of temporal queries.",
    icon: RefreshCw,
    details: [
      "Original events are never modified",
      "Corrections appended with references",
      "as_of queries show pre-correction state",
    ],
    metric: "100% traceable",
    color: "from-green-500/20 to-green-500/5",
  },
  {
    question: "Can past analysis be reproduced?",
    answer: "Yes — automatic snapshots, as_of queries, and event replay engine ensure exact reproducibility.",
    icon: Clock,
    details: [
      "Snapshots: Every 100 events or 1 hour",
      "as_of queries: Point-in-time state",
      "Event Replay: Full deterministic replay",
    ],
    metric: "Exact replay",
    color: "from-purple-500/20 to-purple-500/5",
  },
  {
    question: "How easy is Python integration?",
    answer: "REST API ready with pandas-compatible responses. WebSocket streaming for real-time updates. SDK coming soon.",
    icon: Code2,
    details: [
      "REST API: GET /api/v1/events/query",
      "WebSocket: WS /api/v1/events/stream",
      "Returns JSON convertible to DataFrame",
    ],
    metric: "Multi-platform",
    color: "from-orange-500/20 to-orange-500/5",
  },
  {
    question: "Does it support concurrent users?",
    answer: "Lock-free DashMap architecture handles 40K+ queries/sec with graceful latency degradation under load.",
    icon: Users,
    details: [
      "10 users: 12μs latency, ~83K qps",
      "100 users: 15μs latency, ~66K qps",
      "1000 users: 25μs latency, ~40K qps",
    ],
    metric: "40K+ qps",
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
  {
    method: "GET",
    endpoint: "/api/v1/quant/distributions/{symbol}",
    description: "Probability distributions (Phase 1)",
    upcoming: true,
  },
  {
    method: "POST",
    endpoint: "/api/v1/quant/analyze",
    description: "Custom analysis requests (Phase 2)",
    upcoming: true,
  },
  {
    method: "POST",
    endpoint: "/api/v1/quant/ask",
    description: "Natural language queries (Phase 3)",
    upcoming: true,
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
  return (
    <main className="min-h-screen">
      <Header />

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
              Premium Analytics Layer
            </span>
          </motion.div>

          <motion.h1
            className="mt-6 text-4xl font-bold tracking-tight md:text-6xl"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.1 }}
          >
            Quant Intelligence
          </motion.h1>

          <motion.p
            className="mx-auto mt-4 max-w-2xl text-lg text-muted-foreground"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.2 }}
          >
            Turn raw market data into probability-based insights. From precomputed
            analytics to AI-powered natural language queries — the data layer that
            powers smart trading decisions.
          </motion.p>

          <motion.div
            className="mt-8 flex flex-wrap items-center justify-center gap-4"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.3 }}
          >
            <Link
              href="/signup"
              className={buttonVariants({ variant: "default", size: "lg" })}
            >
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
              { value: "11.9μs", label: "Query Latency" },
              { value: "469K/s", label: "Events Throughput" },
              { value: "< 10ms", label: "Bar Generation" },
              { value: "40K+", label: "Queries/Second" },
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
        title="Technical Capabilities"
        subtitle="Answers to Your Data Layer Questions"
        className="bg-neutral-100 dark:bg-neutral-900"
      >
        <div className="mx-auto mt-12 max-w-5xl space-y-6">
          {capabilities.map((cap, index) => (
            <motion.div
              key={cap.question}
              className={cn(
                "overflow-hidden rounded-2xl border bg-gradient-to-r p-6",
                cap.color
              )}
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
                      <li
                        key={detail}
                        className="flex items-center gap-2 text-sm"
                      >
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
        title="API Design"
        subtitle="Analytics & AI Query Endpoints"
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
                    {endpoint.upcoming && (
                      <span className="rounded bg-primary/20 px-2 py-0.5 text-xs text-primary">
                        Coming Soon
                      </span>
                    )}
                    <span className="text-neutral-500">
                      // {endpoint.description}
                    </span>
                  </motion.div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </Section>

      {/* AI Query Preview */}
      <Section
        title="AI Query Interface"
        subtitle="Natural Language to Insights"
        description="Ask questions about market behavior in plain English. The AI translates your query into optimized analytics and returns probability-based answers."
        className="bg-neutral-100 dark:bg-neutral-900"
      >
        <motion.div
          className="mx-auto mt-12 max-w-4xl"
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          viewport={{ once: true }}
        >
          <div className="relative overflow-hidden rounded-2xl border bg-neutral-900 text-white">
            <Ripple className="absolute -bottom-1/2 opacity-20" />
            <div className="relative z-10 p-8">
              <div className="flex items-center gap-2 text-neutral-400">
                <Brain className="h-5 w-5" />
                <span className="font-medium">AI Query Interface</span>
                <span className="rounded bg-primary/20 px-2 py-0.5 text-xs text-primary">
                  Phase 3
                </span>
              </div>

              <div className="mt-6 space-y-6">
                <div>
                  <div className="text-sm text-neutral-500">Your question:</div>
                  <div className="mt-2 rounded-lg bg-neutral-800/50 p-4 text-green-400">
                    &quot;What&apos;s the probability of NQ making new session highs after a
                    gap up greater than 0.5% on Mondays?&quot;
                  </div>
                </div>

                <div>
                  <div className="text-sm text-neutral-500">Generated SQL:</div>
                  <pre className="mt-2 overflow-x-auto rounded-lg bg-neutral-800/50 p-4 text-xs text-neutral-300">
{`WITH gap_days AS (
  SELECT date, open, high, prev_close,
         (open - prev_close) / prev_close * 100 AS gap_pct
  FROM daily_bars
  WHERE symbol = 'NQ'
    AND EXTRACT(DOW FROM date) = 1
    AND (open - prev_close) / prev_close * 100 > 0.5
)
SELECT
  COUNT(*) AS total_samples,
  SUM(CASE WHEN high > open THEN 1 ELSE 0 END) AS made_new_high,
  AVG(CASE WHEN high > open THEN 1.0 ELSE 0.0 END) AS probability
FROM gap_days`}
                  </pre>
                </div>

                <div>
                  <div className="text-sm text-neutral-500">Answer:</div>
                  <div className="mt-2 rounded-lg bg-neutral-800/50 p-4">
                    <p className="text-neutral-300">
                      Based on <span className="font-bold text-primary">52 samples</span> over
                      5 years of Monday gap-up sessions for NQ:
                    </p>
                    <div className="mt-4 grid gap-4 md:grid-cols-3">
                      <div className="rounded-lg bg-neutral-700/50 p-3">
                        <div className="text-2xl font-bold text-primary">65.4%</div>
                        <div className="text-xs text-neutral-400">Probability</div>
                      </div>
                      <div className="rounded-lg bg-neutral-700/50 p-3">
                        <div className="text-lg font-bold text-white">51.2% - 79.6%</div>
                        <div className="text-xs text-neutral-400">95% Confidence</div>
                      </div>
                      <div className="rounded-lg bg-neutral-700/50 p-3">
                        <div className="text-lg font-bold text-white">52</div>
                        <div className="text-xs text-neutral-400">Sample Size</div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </motion.div>
      </Section>

      {/* Roadmap */}
      <Section
        title="Roadmap"
        subtitle="From Analytics to AI"
        className="bg-background"
      >
        <div className="mx-auto mt-12 max-w-4xl">
          <div className="grid gap-6 md:grid-cols-3">
            {[
              {
                phase: "Phase 1",
                title: "Precomputed Analytics",
                status: "current",
                items: [
                  "NQ/BTC probability distributions",
                  "Basic regime classification",
                  "Canned analytics queries",
                  "< 10ms response times",
                ],
              },
              {
                phase: "Phase 2",
                title: "Dynamic Query Engine",
                status: "upcoming",
                items: [
                  "Custom filter conditions",
                  "Multi-symbol analysis",
                  "Strategy backtesting",
                  "Risk metrics calculation",
                ],
              },
              {
                phase: "Phase 3",
                title: "AI-Powered Queries",
                status: "planned",
                items: [
                  "Natural language interface",
                  "Auto-generated insights",
                  "Strategy recommendations",
                  "Anomaly detection alerts",
                ],
              },
            ].map((phase, index) => (
              <motion.div
                key={phase.phase}
                className={cn(
                  "rounded-2xl border p-6",
                  phase.status === "current"
                    ? "border-primary bg-primary/5"
                    : "bg-background"
                )}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.4, delay: index * 0.1 }}
                viewport={{ once: true }}
              >
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-muted-foreground">
                    {phase.phase}
                  </span>
                  {phase.status === "current" && (
                    <span className="rounded-full bg-primary px-3 py-1 text-xs font-medium text-primary-foreground">
                      Now
                    </span>
                  )}
                  {phase.status === "upcoming" && (
                    <span className="rounded-full bg-blue-500/10 px-3 py-1 text-xs font-medium text-blue-500">
                      Q2 2025
                    </span>
                  )}
                  {phase.status === "planned" && (
                    <span className="rounded-full bg-neutral-500/10 px-3 py-1 text-xs font-medium text-neutral-500">
                      Q3 2025
                    </span>
                  )}
                </div>
                <h3 className="mt-3 text-xl font-semibold">{phase.title}</h3>
                <ul className="mt-4 space-y-2">
                  {phase.items.map((item) => (
                    <li
                      key={item}
                      className="flex items-center gap-2 text-sm text-muted-foreground"
                    >
                      <BarChart3 className="h-4 w-4 text-primary" />
                      {item}
                    </li>
                  ))}
                </ul>
              </motion.div>
            ))}
          </div>
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
            <h2 className="text-3xl font-bold md:text-4xl">
              Ready to Build Smarter?
            </h2>
            <p className="mx-auto mt-4 max-w-xl text-muted-foreground">
              Get early access to Quant Intelligence and start turning market data
              into probability-based insights.
            </p>
            <div className="mt-8 flex flex-wrap items-center justify-center gap-4">
              <Link
                href="/signup"
                className={buttonVariants({ variant: "default", size: "lg" })}
              >
                Get Early Access
                <ChevronRight className="ml-1 h-4 w-4" />
              </Link>
              <Link
                href="/#pricing"
                className={buttonVariants({ variant: "outline", size: "lg" })}
              >
                View Pricing
              </Link>
            </div>
          </motion.div>
        </div>
      </section>

      <Footer />
    </main>
  );
}
