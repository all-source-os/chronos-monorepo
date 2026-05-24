"use client";

import { Badge, buttonVariants, cn, Section } from "@allsource/ui";
import { ChevronRight, Minus, Plus } from "lucide-react";
import { motion } from "motion/react";
import Link from "next/link";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

// Five approaches to AI agent memory, ordered roughly by how "managed" each
// is — least infrastructure first, most infrastructure last. AllSource Prime
// is "event-sourced memory"; called out explicitly but the page is written
// to be useful even for readers who pick something else.
type Approach = {
  id: string;
  name: string;
  examples: string;
  blurb: string;
  wins: string[];
  loses: string[];
  // Set to render the "Try AllSource Prime →" CTA on this card.
  ours?: boolean;
};

const approaches: Approach[] = [
  {
    id: "platform",
    name: "Platform memory",
    examples: "Claude built-in memory · ChatGPT memory · Gemini saved info",
    blurb:
      "Memory features built into the model vendor's product surface. You enable a setting, the assistant starts remembering across conversations.",
    wins: [
      "Zero configuration — turn it on in settings, done",
      "Native UX integrated with the chat interface",
      "No code, no infrastructure",
    ],
    loses: [
      "Locked to one vendor — your memory in Claude isn't available in ChatGPT",
      "No programmatic access — you can't query or export what's been remembered",
      "Limited or no audit trail / version history",
      "Memory shape is decided by the vendor, not you",
    ],
  },
  {
    id: "retrieval",
    name: "Retrieval / RAG memory",
    examples: "Mem0 · Zep · raw vector DBs (Pinecone, Weaviate, pgvector)",
    blurb:
      "Store conversation chunks (or extracted facts) as vector embeddings; retrieve via semantic similarity at query time. The dominant pattern for the past two years.",
    wins: [
      "Works for unstructured content — chat logs, documents, notes",
      "Multi-tool friendly via REST APIs",
      "Semantic recall finds adjacent ideas, not just exact matches",
    ],
    loses: [
      "Similarity is not truth — the top match can be plausible and wrong",
      "Hard to verify what's in the store without querying with the right phrasing",
      "No first-class graph — relationships between facts are not modeled",
      '"What did I tell you last Tuesday?" requires you to already know what to ask',
    ],
  },
  {
    id: "files",
    name: "File-based memory",
    examples: "CLAUDE.md · AGENTS.md · project-local markdown / JSON",
    blurb:
      "Human-readable files in a folder. Agents read and edit them directly. Often committed to git so changes are reviewable.",
    wins: [
      "Trivial to inspect — open the file in any editor",
      "Version-controlled for free via git",
      "Zero infrastructure",
      "Works offline, no service to babysit",
    ],
    loses: [
      "Manual merge conflicts when two agents (or an agent and a human) edit the same file",
      'No structured queries — "all decisions involving Alice" requires grep',
      "No typed relations between facts",
      "Scales poorly past a few hundred facts — the file becomes a wall of text the model can't parse efficiently",
    ],
  },
  {
    id: "database",
    name: "Database memory",
    examples: "Postgres + CRUD · Supabase · Drizzle/Prisma + agent functions",
    blurb:
      "A relational table with rows for entities; the agent does CRUD via function-call tools. Leverages skills your team already has.",
    wins: [
      "Structured, queryable, durable",
      "Uses tooling your team already knows (migrations, ORMs, SQL)",
      "Constraints enforce schema at write time",
    ],
    loses: [
      "Every schema change is a migration",
      "No time-travel without bitemporal columns (`valid_from`, `valid_to`) and your own query layer",
      "No graph — adjacency requires explicit join tables you maintain",
      "Vector recall isn't there without bolting on pgvector or a sidecar",
    ],
  },
  {
    id: "event-sourced",
    name: "Event-sourced memory",
    examples: "AllSource Prime · Neotoma · roll-your-own event store",
    blurb:
      "Memory as an append-only log of events. Current state is projected from the log; full history is preserved. AllSource Prime adds a knowledge graph and vector recall on top of the same event spine.",
    wins: [
      'Time-travel by construction — "what did I know about X as of last Tuesday?" is a query',
      "Full audit trail — every fact is preserved with provenance and timestamp",
      "Graph + vector recall in one query (Prime's `prime_recall`)",
      "Hosted multi-tenant or local-first — same data shape both ways",
      "Cross-tool sync via MCP — same memory in Claude Desktop, the Anthropic CLI, Cursor, OpenCode",
    ],
    loses: [
      "More infrastructure than a markdown file or platform memory",
      "Conceptually different from CRUD (events, not rows) — learning curve",
      "Newer category — fewer drop-in tutorials than for Postgres or vector DBs",
    ],
    ours: true,
  },
];

function Card({ approach }: { approach: Approach }) {
  return (
    <div
      className={cn(
        "rounded-xl border bg-card p-6",
        approach.ours && "border-primary/50 bg-primary/5"
      )}
    >
      <div className="mb-3 flex flex-wrap items-baseline justify-between gap-2">
        <h3 className="text-xl font-semibold">{approach.name}</h3>
        {approach.ours && (
          <Badge variant="default" className="text-xs">
            AllSource Prime
          </Badge>
        )}
      </div>
      <div className="mb-3 font-mono text-xs text-muted-foreground">{approach.examples}</div>
      <p className="mb-5 text-sm text-muted-foreground">{approach.blurb}</p>

      <div className="grid gap-4 md:grid-cols-2">
        <div>
          <div className="mb-2 text-xs font-medium uppercase tracking-wide text-green-600 dark:text-green-400">
            When it wins
          </div>
          <ul className="space-y-1.5">
            {approach.wins.map((w) => (
              <li key={w} className="flex items-start gap-2 text-sm">
                <Plus className="mt-0.5 h-3.5 w-3.5 shrink-0 text-green-500" />
                <span>{w}</span>
              </li>
            ))}
          </ul>
        </div>
        <div>
          <div className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            When it loses
          </div>
          <ul className="space-y-1.5">
            {approach.loses.map((l) => (
              <li key={l} className="flex items-start gap-2 text-sm">
                <Minus className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground/60" />
                <span>{l}</span>
              </li>
            ))}
          </ul>
        </div>
      </div>

      {approach.ours && (
        <div className="mt-5 flex flex-wrap gap-2">
          <Link href="/connect" className={cn(buttonVariants({ variant: "default" }), "gap-1.5")}>
            Try AllSource Prime <ChevronRight className="h-4 w-4" />
          </Link>
          <Link href="/prime" className={cn(buttonVariants({ variant: "ghost" }), "text-sm")}>
            Read about Prime
          </Link>
        </div>
      )}
    </div>
  );
}

export default function CompareAgentMemoryPage() {
  return (
    <>
      <Header />
      <main className="relative overflow-hidden">
        <Section className="relative pt-24 pb-12 text-center">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
          >
            <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">
              Agent memory: five approaches, honestly compared
            </h1>
            <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
              Every team building with AI agents picks one of these five patterns — usually
              accidentally. Here's where each wins, where each loses, and how to pick on purpose.
            </p>
          </motion.div>
        </Section>

        <Section className="pb-16">
          <div className="mx-auto flex max-w-3xl flex-col gap-6">
            {approaches.map((a) => (
              <Card key={a.id} approach={a} />
            ))}
          </div>
        </Section>

        <Section className="pb-24">
          <div className="mx-auto max-w-3xl rounded-xl border bg-muted/20 p-6">
            <h2 className="mb-3 text-xl font-semibold">How to pick</h2>
            <ul className="space-y-2 text-sm text-muted-foreground">
              <li>
                <strong className="text-foreground">Single-user, single-tool, low volume?</strong>{" "}
                Platform memory or a CLAUDE.md file. Stop reading, start writing.
              </li>
              <li>
                <strong className="text-foreground">
                  Unstructured content, semantic search is the killer feature?
                </strong>{" "}
                Mem0 or Zep. Accept the truthiness tradeoff.
              </li>
              <li>
                <strong className="text-foreground">
                  Structured entities, no time-travel needs?
                </strong>{" "}
                Postgres + CRUD. You already know the playbook.
              </li>
              <li>
                <strong className="text-foreground">
                  Multi-tool, multi-user, audit-driven, or you want both graph and vector recall?
                </strong>{" "}
                Event-sourced.{" "}
                <Link href="/prime" className="underline">
                  AllSource Prime
                </Link>{" "}
                is one of the few productized options in this category.
              </li>
            </ul>
          </div>
        </Section>
      </main>
      <Footer />
    </>
  );
}
