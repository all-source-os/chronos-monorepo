"use client";

import { cn, FlickeringGrid, Ripple, Section } from "@allsource/ui";
import { Bot, Clock, Database, GitBranch, RefreshCw, Search, Shield, Zap } from "lucide-react";
import { motion } from "motion/react";

const metrics = [
  { value: "11.9μs", label: "Query Latency" },
  { value: "469K", label: "Events/Second" },
  { value: "27", label: "MCP Tools" },
  { value: "~129MB", label: "Binary Size" },
];

const capabilities = [
  {
    icon: Clock,
    title: "Time-Travel Queries",
    description:
      "Reconstruct any entity's state at any point in time with as_of queries. Debug issues, audit changes, or replay historical states instantly.",
    color: "hover:bg-purple-500/10",
    metric: "Any timestamp",
  },
  {
    icon: Zap,
    title: "Microsecond Latency",
    description:
      "11.9μs p99 query latency with Parquet-based storage. Optimized for both high-throughput ingestion and fast analytical queries.",
    color: "hover:bg-yellow-500/10",
    metric: "11.9μs p99",
  },
  {
    icon: RefreshCw,
    title: "Event Replay",
    description:
      "Rebuild projections, fix incorrect state, or migrate data by replaying events. Filter by entity, type, or time range.",
    color: "hover:bg-green-500/10",
    metric: "Full replay",
  },
  {
    icon: GitBranch,
    title: "Projections",
    description:
      "Materialized views that update automatically. Entity snapshots, counters, time series, funnels, and custom projections.",
    color: "hover:bg-blue-500/10",
    metric: "5 types",
  },
  {
    icon: Database,
    title: "Schema Registry",
    description:
      "Versioned schemas with backward, forward, and full compatibility modes. Validate events at ingestion time.",
    color: "hover:bg-orange-500/10",
    metric: "Schema evolution",
  },
  {
    icon: Shield,
    title: "Multi-Tenant Isolation",
    description:
      "Every event, projection, and query is scoped to a tenant. RBAC with 4 roles and configurable quotas per tier.",
    color: "hover:bg-pink-500/10",
    metric: "Full isolation",
  },
];

const architectureFlow = [
  { label: "Ingest", icon: "📥" },
  { label: "Store", icon: "💾" },
  { label: "Project", icon: "🔄" },
  { label: "Query", icon: "🔍" },
  { label: "AI Tools", icon: "🤖" },
];

export default function QuantIntelligence() {
  return (
    <Section
      id="architecture"
      title="Event Intelligence"
      subtitle="AI-Native Event Store Architecture"
      description="Built in Rust for performance. Designed for AI agents. Every feature accessible through 27 MCP tools — let Claude or GPT manage your event streams autonomously."
      className="bg-gradient-to-b from-background to-neutral-100 dark:to-neutral-900"
    >
      {/* Metrics Bar */}
      <motion.div
        className="mx-auto mt-12 grid max-w-4xl grid-cols-2 gap-4 md:grid-cols-4"
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
        viewport={{ once: true }}
      >
        {metrics.map((metric, index) => (
          <motion.div
            key={metric.label}
            className="flex flex-col items-center justify-center rounded-xl border bg-background/50 p-4 backdrop-blur-sm"
            initial={{ opacity: 0, scale: 0.9 }}
            whileInView={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.3, delay: index * 0.1 }}
            viewport={{ once: true }}
          >
            <span className="text-2xl font-bold text-primary md:text-3xl">{metric.value}</span>
            <span className="text-sm text-muted-foreground">{metric.label}</span>
          </motion.div>
        ))}
      </motion.div>

      {/* Architecture Flow */}
      <motion.div
        className="mx-auto mt-12 max-w-4xl"
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        transition={{ duration: 0.5, delay: 0.2 }}
        viewport={{ once: true }}
      >
        <div className="relative flex items-center justify-between overflow-hidden rounded-2xl border bg-background/50 p-6 backdrop-blur-sm">
          <FlickeringGrid
            className="absolute inset-0 z-0 [mask:radial-gradient(ellipse_at_center,#fff_300px,transparent_100%)]"
            squareSize={4}
            gridGap={6}
            color="#f97316"
            maxOpacity={0.1}
            flickerChance={0.1}
            height={200}
            width={1000}
          />
          <div className="relative z-10 flex w-full items-center justify-between">
            {architectureFlow.map((step, index) => (
              <div key={step.label} className="flex items-center">
                <motion.div
                  className="flex flex-col items-center gap-2"
                  initial={{ opacity: 0, y: 10 }}
                  whileInView={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.3, delay: 0.3 + index * 0.1 }}
                  viewport={{ once: true }}
                >
                  <span className="text-2xl md:text-3xl">{step.icon}</span>
                  <span className="text-xs font-medium text-foreground md:text-sm">
                    {step.label}
                  </span>
                </motion.div>
                {index < architectureFlow.length - 1 && (
                  <div className="mx-2 hidden h-0.5 w-8 bg-gradient-to-r from-primary/50 to-primary md:mx-4 md:block md:w-16" />
                )}
              </div>
            ))}
          </div>
        </div>
      </motion.div>

      {/* Capability Cards */}
      <div className="mx-auto mt-12 grid max-w-6xl grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
        {capabilities.map((capability, index) => (
          <motion.div
            key={capability.title}
            className={cn(
              "group relative overflow-hidden rounded-2xl border bg-background/50 p-6 backdrop-blur-sm transition-all duration-500",
              capability.color
            )}
            initial={{ opacity: 0, y: 30 }}
            whileInView={{ opacity: 1, y: 0 }}
            transition={{
              duration: 0.4,
              delay: index * 0.1,
              type: "spring",
              stiffness: 100,
              damping: 20,
            }}
            viewport={{ once: true }}
          >
            <div className="flex items-start justify-between">
              <div className="rounded-lg bg-primary/10 p-2">
                <capability.icon className="h-5 w-5 text-primary" />
              </div>
              <span className="rounded-full bg-primary/10 px-2 py-1 text-xs font-medium text-primary">
                {capability.metric}
              </span>
            </div>
            <h3 className="mt-4 font-semibold text-foreground">{capability.title}</h3>
            <p className="mt-2 text-sm text-muted-foreground">{capability.description}</p>
          </motion.div>
        ))}
      </div>

      {/* MCP Tools Preview */}
      <motion.div
        className="mx-auto mt-12 max-w-4xl"
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, delay: 0.3 }}
        viewport={{ once: true }}
      >
        <div className="relative overflow-hidden rounded-2xl border bg-neutral-900 p-6 text-white dark:bg-neutral-950">
          <Ripple className="absolute -bottom-1/2 opacity-30" />
          <div className="relative z-10">
            <div className="flex items-center gap-2 text-sm text-neutral-400">
              <Bot className="h-4 w-4" />
              <span>MCP Server — Claude Desktop Integration</span>
            </div>
            <div className="mt-4 font-mono text-sm">
              <div className="text-neutral-500">{"// AI agent query"}</div>
              <div className="mt-2 text-green-400">
                {"Claude: Show me all user.signed_up events from last week"}
              </div>
              <div className="mt-4 rounded-lg bg-neutral-800/50 p-4">
                <div className="text-neutral-300">
                  Found <span className="text-primary">847 events</span> from 7 days:
                </div>
                <div className="mt-2 flex flex-wrap gap-4">
                  <div>
                    <span className="text-2xl font-bold text-primary">121</span>
                    <span className="ml-2 text-neutral-400">events/day avg</span>
                  </div>
                  <div>
                    <span className="text-neutral-400">Peak:</span>
                    <span className="ml-2 text-neutral-300">Tuesday 2pm</span>
                  </div>
                </div>
                <div className="mt-3 text-xs text-neutral-500">
                  Query executed in 11.2μs via mcp_query_events tool
                </div>
              </div>
            </div>
          </div>
        </div>
      </motion.div>

      {/* Use Cases */}
      <motion.div
        className="mx-auto mt-12 max-w-4xl"
        initial={{ opacity: 0 }}
        whileInView={{ opacity: 1 }}
        transition={{ duration: 0.5, delay: 0.4 }}
        viewport={{ once: true }}
      >
        <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
          {[
            {
              title: "Event Sourcing",
              items: ["Immutable audit logs", "State reconstruction", "Event replay"],
              icon: <Database className="h-4 w-4" />,
            },
            {
              title: "AI Workflows",
              items: ["27 MCP tools", "Claude integration", "Autonomous agents"],
              icon: <Bot className="h-4 w-4" />,
            },
            {
              title: "Analytics",
              items: ["Stream processing", "Real-time pipelines", "Correlation analysis"],
              icon: <Search className="h-4 w-4" />,
            },
          ].map((useCase, index) => (
            <motion.div
              key={useCase.title}
              className="rounded-xl border bg-background/50 p-4"
              initial={{ opacity: 0, x: -20 }}
              whileInView={{ opacity: 1, x: 0 }}
              transition={{ duration: 0.3, delay: 0.5 + index * 0.1 }}
              viewport={{ once: true }}
            >
              <div className="flex items-center gap-2">
                <div className="rounded-lg bg-primary/10 p-1.5">{useCase.icon}</div>
                <h4 className="font-semibold text-foreground">{useCase.title}</h4>
              </div>
              <ul className="mt-3 space-y-1">
                {useCase.items.map((item) => (
                  <li key={item} className="flex items-center gap-2 text-sm text-muted-foreground">
                    <div className="h-1 w-1 rounded-full bg-primary" />
                    {item}
                  </li>
                ))}
              </ul>
            </motion.div>
          ))}
        </div>
      </motion.div>
    </Section>
  );
}
