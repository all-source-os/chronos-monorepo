"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  Brain,
  ChevronRight,
  Compass,
  Database,
  Layers,
  Network,
  Search,
  Timer,
  Unplug,
  Zap,
} from "lucide-react";
import Link from "next/link";
import { staticMotion as motion } from "@/components/ui/static-motion";

const features = [
  {
    title: "Knowledge Graph",
    description:
      "Model entities and their relationships as a directed graph. BFS traversal discovers multi-hop connections across domains — customers to orders to products to suppliers — without pre-defined joins.",
    icon: Network,
    color: "from-primary/20 to-primary/5",
  },
  {
    title: "Vector Embeddings",
    description:
      "HNSW index stores high-dimensional embeddings alongside your events. Semantic similarity search finds related entities even when they share no common identifiers or schemas.",
    icon: Compass,
    color: "from-primary/20 to-primary/5",
  },
  {
    title: "Compressed Index",
    description:
      "Auto-generated cross-domain scaffolding links entities that co-occur across event streams. A published AllSource project evaluation measured improved cross-domain recall on its test corpus; results depend on your data and queries.",
    icon: Layers,
    color: "from-primary/20 to-primary/5",
  },
  {
    title: "Recall API",
    description:
      "Hybrid search that combines vector similarity, graph traversal, and temporal ordering in a single query. Your AI agents get ranked results with provenance — not just embeddings, but the events that produced them.",
    icon: Search,
    color: "from-green-500/20 to-green-500/5",
  },
  {
    title: "Core-Backed Projection Reads",
    description:
      "A concurrent in-memory map serves Core projection reads separately from event ingestion. The published 11.9us p99 Core benchmark is not an end-to-end Prime hybrid-recall measurement.",
    icon: Timer,
    color: "from-yellow-500/20 to-yellow-500/5",
  },
  {
    title: "Offline / Embedded Mode",
    description:
      "Run Prime in-process via a Rustler NIF — no network hop, no separate service. Ship durable AI agent memory inside your Elixir application or as a standalone embedded binary.",
    icon: Unplug,
    color: "from-orange-500/20 to-orange-500/5",
  },
];

const codeExample = `# Ask Prime to recall relevant context for an AI agent
curl -X POST https://api.all-source.xyz/api/v1/prime/recall \\
  -H "Authorization: Bearer $API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "query": "What do we know about customer acme-corp?",
    "top_k": 5,
    "strategy": "hybrid"
  }'

# Example response shape; scores depend on corpus and query
{
  "nodes": [
    {
      "entity_id": "customer:acme-corp",
      "type": "customer",
      "score": 0.94,
      "source": "graph+vector",
      "relationships": [
        { "to": "order:ord-7821", "rel": "placed", "weight": 0.88 },
        { "to": "ticket:tkt-312", "rel": "opened", "weight": 0.72 }
      ],
      "last_event": "2026-04-15T09:32:11Z"
    },
    {
      "entity_id": "order:ord-7821",
      "type": "order",
      "score": 0.88,
      "source": "graph",
      "payload": { "total": 24500.00, "status": "fulfilled" },
      "last_event": "2026-04-14T14:07:44Z"
    }
  ],
  "strategy_used": "hybrid"
}`;

const coreBullets = [
  {
    icon: Database,
    text: "Hosted Prime reads and writes tenant-scoped prime.* events through Core. Local Prime embeds Core in-process. In both modes, durable events are the source for graph and vector projections.",
  },
  {
    icon: Layers,
    text: "All Prime data is a projection. If you rebuild Prime from scratch, it replays events from Core and arrives at the same state. No separate source of truth.",
  },
  {
    icon: Zap,
    text: "You can run Core without Prime. The event store stands alone for event sourcing, time-travel queries, and projections. Prime is purely additive.",
  },
  {
    icon: Brain,
    text: "Hosted Prime keeps only warm per-tenant projections in memory and rebuilds them from Core after a cold start. Local Prime keeps its event log and projections in the selected data directory.",
  },
];

export default function PrimePage() {
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
          <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-primary/30 bg-primary/10 px-4 py-1.5 text-sm text-foreground">
            <Brain className="h-4 w-4" />
            Add-on Module
          </div>
          <h1 className="text-4xl font-bold tracking-tight sm:text-6xl">
            Knowledge graph and vector recall for AI agents
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            Prime is an optional module that adds knowledge graphs, vector search, and agent recall
            to the AllSource event store. Combine graph traversal, HNSW embeddings, provenance, and
            temporal context in one memory layer.
          </p>
          <div className="mt-8 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
            <Link href="/docs/prime" className={cn(buttonVariants({ variant: "default" }))}>
              Add Prime to your stack <ChevronRight className="ml-1 h-4 w-4" />
            </Link>
            <Link
              href="/platform/event-sourcing"
              className={cn(buttonVariants({ variant: "outline" }))}
            >
              Learn about Core
            </Link>
          </div>
        </motion.div>
      </Section>

      {/* Key metrics */}
      <Section className="py-12">
        <div className="mx-auto grid max-w-4xl grid-cols-2 gap-8 text-center sm:grid-cols-4">
          {[
            { value: "Core", label: "durable source", sub: "WAL + Parquet events" },
            { value: "3", label: "search strategies", sub: "vector + graph + temporal" },
            { value: "Published", label: "recall evaluation", sub: "method and corpus disclosed" },
            { value: "0", label: "network hops", sub: "embedded NIF mode" },
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
        title="Everything an AI agent needs to remember"
        subtitle="Knowledge graph event store meets vector search — purpose-built for durable agent memory in Rust"
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
      <Section
        title="One API call. Full context."
        subtitle="The Recall API combines graph traversal, vector similarity, and temporal ordering into a single ranked response"
        className="py-16"
      >
        <motion.div
          className="bg-brand-ink mx-auto max-w-3xl overflow-hidden rounded-xl border"
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

      {/* Works with Core */}
      <Section
        title="Works with the Core event store"
        subtitle="Prime is a projection layer — all data is derived from events that Core already persists"
        className="py-16"
      >
        <div className="mx-auto max-w-2xl space-y-6">
          {coreBullets.map((bullet, i) => (
            <motion.div
              key={bullet.text}
              initial={{ opacity: 0, x: -10 }}
              whileInView={{ opacity: 1, x: 0 }}
              transition={{ delay: i * 0.08 }}
              viewport={{ once: true }}
              className="flex items-start gap-4"
            >
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border bg-muted/50">
                <bullet.icon className="h-5 w-5 text-primary" />
              </div>
              <p className="text-muted-foreground">{bullet.text}</p>
            </motion.div>
          ))}
        </div>
      </Section>

      {/* CTA */}
      <Section className="py-16 text-center">
        <h2 className="text-3xl font-bold">Add durable memory to your AI agents</h2>
        <p className="mx-auto mt-4 max-w-lg text-muted-foreground">
          Prime plugs into any AllSource Core deployment. Knowledge graphs, vector search, and
          compressed recall — no external dependencies, no separate database.
        </p>
        <div className="mt-8 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
          <Link
            href="/docs/prime"
            className={cn(buttonVariants({ variant: "default", size: "lg" }))}
          >
            Add Prime to your stack
          </Link>
          <Link
            href="/platform/event-sourcing"
            className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
          >
            Learn about the Core event store
          </Link>
        </div>
      </Section>
    </div>
  );
}
