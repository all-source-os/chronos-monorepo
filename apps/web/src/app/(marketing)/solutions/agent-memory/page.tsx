"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  Brain,
  Check,
  ChevronRight,
  Clock,
  Code2,
  Database,
  GitBranch,
  Layers,
  Network,
  Search,
  Shield,
  Terminal,
  Zap,
} from "lucide-react";
import { motion } from "motion/react";
import Link from "next/link";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

const capabilities = [
  {
    question: "How does an agent remember?",
    answer:
      "Every fact is an immutable event stored in a durable WAL + Parquet engine. Knowledge graph nodes, vector embeddings, and relationships are all events — giving you full history and time-travel for free.",
    icon: Database,
    details: [
      "Graph nodes: entities, concepts, decisions",
      "Vector embeddings: semantic similarity search",
      "Compressed index: navigational scaffolding",
    ],
    metric: "469K events/sec",
    color: "from-blue-500/20 to-blue-500/5",
  },
  {
    question: "How fast is recall?",
    answer:
      "12μs projection lookups via DashMap. Hybrid recall combines vector similarity, graph traversal, and temporal recency in a single query.",
    icon: Zap,
    details: [
      "Vector search: HNSW index over embeddings",
      "Graph expansion: 1-hop BFS from matches",
      "Compressed index: cross-domain reasoning",
    ],
    metric: "12μs p99",
    color: "from-yellow-500/20 to-yellow-500/5",
  },
  {
    question: "What about cross-domain questions?",
    answer:
      "The compressed index — an auto-generated markdown summary organized by domain — bridges the gap that pure vector search misses. When you ask 'how does X relate to Y?', the index provides cross-domain pointers that double retrieval accuracy.",
    icon: Network,
    details: [
      "Auto-generated from graph events",
      "Organized by domain with cross-references",
      "80%+ cross-domain recall accuracy",
    ],
    metric: "80% cross-ref",
    color: "from-purple-500/20 to-purple-500/5",
  },
  {
    question: "Can I time-travel?",
    answer:
      "Every mutation is an append-only event. Query any entity's state at any past timestamp. See who added what, when, and why.",
    icon: Clock,
    details: [
      "as_of queries: reconstruct past state",
      "Full audit trail: every create, update, delete",
      "Graph diff: what changed between two timestamps",
    ],
    metric: "Full provenance",
    color: "from-green-500/20 to-green-500/5",
  },
];

const comparison = [
  {
    feature: "Compressed Index",
    zerodex: "Manual",
    mem0: "No",
    letta: "No",
    zep: "No",
    prime: "Auto-generated",
  },
  { feature: "Temporal Queries", zerodex: "No", mem0: "No", letta: "No", zep: "Yes", prime: "Yes" },
  {
    feature: "Provenance",
    zerodex: "No",
    mem0: "No",
    letta: "Partial",
    zep: "Partial",
    prime: "Full event audit",
  },
  {
    feature: "Cross-Domain Recall",
    zerodex: "80%",
    mem0: "~50%",
    letta: "~37%",
    zep: "~85%",
    prime: "80%+",
  },
  {
    feature: "Offline/Embedded",
    zerodex: "Yes",
    mem0: "No",
    letta: "No",
    zep: "Optional",
    prime: "Yes + sync",
  },
  {
    feature: "Latency",
    zerodex: "70ms",
    mem0: "Variable",
    letta: "Variable",
    zep: "Variable",
    prime: "12μs",
  },
  { feature: "Cost", zerodex: "$0", mem0: "$0-249/mo", letta: "Cloud", zep: "Cloud", prime: "$0" },
];

const useCases = [
  {
    title: "Personal AI Assistant",
    description:
      "Claude remembers your project context across sessions. Yesterday's decisions inform today's answers.",
    icon: Brain,
  },
  {
    title: "Multi-Agent Knowledge Sharing",
    description:
      "Three agents work on different parts of a codebase. Findings flow through the shared graph.",
    icon: GitBranch,
  },
  {
    title: "Incident Response Memory",
    description:
      "Your oncall agent remembers every past incident. 'What happened last time this alert fired?'",
    icon: Shield,
  },
  {
    title: "Research Assistant",
    description:
      "Read 50 papers, build a knowledge graph. The compressed index surfaces unexpected cross-domain connections.",
    icon: Search,
  },
  {
    title: "Code Review Context",
    description:
      "Agent remembers past review feedback. 'Last time you said X about error handling in this module.'",
    icon: Code2,
  },
  {
    title: "Audit & Compliance",
    description:
      "Full provenance on every memory. Who added what, when, from what source. Time-travel to any past state.",
    icon: Layers,
  },
];

export default function AgentMemoryPage() {
  return (
    <>
      <Header />
      <main className="relative overflow-hidden">
        {/* Hero */}
        <Section className="relative pt-24 pb-16 text-center">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
          >
            <h1 className="text-4xl font-bold tracking-tight sm:text-6xl">
              Give your AI agent memory that
              <br />
              <span className="bg-gradient-to-r from-purple-400 to-blue-400 bg-clip-text text-transparent">
                remembers every event
              </span>
            </h1>
            <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
              Knowledge graph + vector search + compressed index in one binary. No cloud dependency.
              No monthly fees. 12μs queries.
            </p>
            <div className="mt-8 flex items-center justify-center gap-4">
              <div className="rounded-lg border bg-muted/50 px-4 py-2 font-mono text-sm">
                cargo install allsource-prime
              </div>
              <Link
                href="https://github.com/all-source-os/all-source"
                className={cn(buttonVariants({ variant: "outline" }))}
              >
                View on GitHub <ChevronRight className="ml-1 h-4 w-4" />
              </Link>
            </div>
          </motion.div>
        </Section>

        {/* Claude Desktop Config */}
        <Section className="pb-16">
          <div className="mx-auto max-w-2xl">
            <h2 className="mb-4 text-center text-2xl font-semibold">
              30 seconds to persistent memory
            </h2>
            <div className="rounded-lg border bg-muted/30 p-6">
              <p className="mb-3 text-sm text-muted-foreground">
                Add to <code>~/.claude/claude_desktop_config.json</code>:
              </p>
              <pre className="overflow-x-auto rounded bg-black/80 p-4 text-sm text-green-400">
                {`{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/.prime/memory"]
    }
  }
}`}
              </pre>
              <p className="mt-3 text-sm text-muted-foreground">
                13 MCP tools: graph CRUD, vector search, hybrid recall, compressed index, temporal
                queries.
              </p>
            </div>
          </div>
        </Section>

        {/* Capabilities */}
        <Section className="pb-16">
          <h2 className="mb-12 text-center text-3xl font-bold">How it works</h2>
          <div className="grid gap-8 md:grid-cols-2">
            {capabilities.map((cap, i) => (
              <motion.div
                key={cap.question}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.1 }}
                viewport={{ once: true }}
                className="rounded-xl border p-6"
              >
                <div className="mb-4 flex items-center gap-3">
                  <div
                    className={cn(
                      "flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br",
                      cap.color
                    )}
                  >
                    <cap.icon className="h-5 w-5" />
                  </div>
                  <span className="rounded-full bg-muted px-3 py-1 text-xs font-medium">
                    {cap.metric}
                  </span>
                </div>
                <h3 className="mb-2 text-lg font-semibold">{cap.question}</h3>
                <p className="mb-4 text-sm text-muted-foreground">{cap.answer}</p>
                <ul className="space-y-1">
                  {cap.details.map((d) => (
                    <li key={d} className="flex items-start gap-2 text-sm">
                      <Check className="mt-0.5 h-4 w-4 shrink-0 text-green-500" />
                      {d}
                    </li>
                  ))}
                </ul>
              </motion.div>
            ))}
          </div>
        </Section>

        {/* Comparison Table */}
        <Section className="pb-16">
          <h2 className="mb-4 text-center text-3xl font-bold">How we compare</h2>
          <p className="mb-8 text-center text-muted-foreground">
            vs. zer0dex, Mem0, Letta, and Zep
          </p>
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-sm">
              <thead>
                <tr className="border-b">
                  <th className="p-3 text-left font-medium">Feature</th>
                  <th className="p-3 text-center font-medium">zer0dex</th>
                  <th className="p-3 text-center font-medium">Mem0</th>
                  <th className="p-3 text-center font-medium">Letta</th>
                  <th className="p-3 text-center font-medium">Zep</th>
                  <th className="p-3 text-center font-medium text-purple-400">Prime</th>
                </tr>
              </thead>
              <tbody>
                {comparison.map((row) => (
                  <tr key={row.feature} className="border-b border-muted">
                    <td className="p-3 font-medium">{row.feature}</td>
                    <td className="p-3 text-center text-muted-foreground">{row.zerodex}</td>
                    <td className="p-3 text-center text-muted-foreground">{row.mem0}</td>
                    <td className="p-3 text-center text-muted-foreground">{row.letta}</td>
                    <td className="p-3 text-center text-muted-foreground">{row.zep}</td>
                    <td className="p-3 text-center font-semibold text-purple-400">{row.prime}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="mt-4 text-center text-xs text-muted-foreground">
            Sources:{" "}
            <Link href="https://github.com/roli-lpci/zer0dex" className="underline">
              zer0dex
            </Link>
            {" · "}
            <Link
              href="https://vectorize.io/articles/best-ai-agent-memory-systems"
              className="underline"
            >
              Vectorize 2026 Comparison
            </Link>
          </p>
        </Section>

        {/* Use Cases */}
        <Section className="pb-16">
          <h2 className="mb-12 text-center text-3xl font-bold">Use cases</h2>
          <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
            {useCases.map((uc, i) => (
              <motion.div
                key={uc.title}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.08 }}
                viewport={{ once: true }}
                className="rounded-xl border p-6"
              >
                <uc.icon className="mb-3 h-8 w-8 text-purple-400" />
                <h3 className="mb-2 font-semibold">{uc.title}</h3>
                <p className="text-sm text-muted-foreground">{uc.description}</p>
              </motion.div>
            ))}
          </div>
        </Section>

        {/* Architecture */}
        <Section className="pb-16">
          <h2 className="mb-8 text-center text-3xl font-bold">One engine, not three databases</h2>
          <div className="mx-auto max-w-2xl">
            <pre className="overflow-x-auto rounded-lg border bg-muted/30 p-6 text-xs leading-relaxed">
              {`┌─────────────────────────────────────────────┐
│              AllSource Prime                 │
│                                              │
│  Graph    Vectors    Temporal    Compressed   │
│  Nodes    HNSW       History    Index        │
│  Edges    Embed      Time-travel Cross-refs  │
│           Similar    Diff                    │
│                                              │
│  ┌──────────────────────────────────────┐   │
│  │         AllSource Core Engine         │   │
│  │  WAL + Parquet + DashMap + HLC + CRDT │   │
│  │  469K events/sec │ 12μs queries       │   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘`}
            </pre>
            <p className="mt-4 text-center text-sm text-muted-foreground">
              Other agent memory frameworks glue together a vector DB + graph DB + event store.
              Prime is one engine where vectors, graph nodes, and edges are <em>all events</em> in
              the same durable WAL.
            </p>
          </div>
        </Section>

        {/* CTA */}
        <Section className="pb-24 text-center">
          <h2 className="mb-4 text-3xl font-bold">Start remembering</h2>
          <p className="mb-8 text-muted-foreground">One command. No cloud account. No API key.</p>
          <div className="flex items-center justify-center gap-4">
            <div className="rounded-lg border bg-muted/50 px-6 py-3 font-mono text-sm">
              <Terminal className="mr-2 inline h-4 w-4" />
              cargo install allsource-prime
            </div>
          </div>
          <div className="mt-6 flex items-center justify-center gap-6 text-sm text-muted-foreground">
            <Link href="/docs" className="underline">
              Documentation
            </Link>
            <Link
              href="https://github.com/all-source-os/all-source/tree/main/apps/core/examples"
              className="underline"
            >
              Examples
            </Link>
            <Link href="https://github.com/all-source-os/all-source" className="underline">
              GitHub
            </Link>
          </div>
        </Section>
      </main>
      <Footer />
    </>
  );
}
