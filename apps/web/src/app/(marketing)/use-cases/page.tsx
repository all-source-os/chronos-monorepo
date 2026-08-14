"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  Brain,
  ChevronRight,
  Clock,
  DollarSign,
  FileCheck,
  RotateCcw,
  Search,
  Shield,
  Terminal,
} from "lucide-react";
import Link from "next/link";
import { staticMotion as motion } from "@/components/ui/static-motion";

const useCases = [
  {
    id: "audit-trails",
    title: "Audit Trails & Compliance",
    subtitle: "History queryable by timestamp, actor, and entity",
    icon: FileCheck,
    color: "from-blue-500/20 to-blue-500/5",
    description:
      "Each accepted state change is an append-only event with integrity metadata. Query provenance by timestamp, actor, or entity instead of reconstructing it from mutable rows.",
    scenario:
      "A regulator asks what your system knew about a customer on 2024-09-12. Run an `as_of` query and export the reconstructed state with its source events.",
    bullets: [
      "Append-only WAL with CRC32 checksums",
      "`as_of` queries reconstruct any past state",
      "Actor + metadata captured on every event",
      "Exportable history for compliance review",
    ],
  },
  {
    id: "event-replay",
    title: "Event Replay & Debugging",
    subtitle: "Reproduce bugs by rewinding history",
    icon: RotateCcw,
    color: "from-purple-500/20 to-purple-500/5",
    description:
      "Stop guessing how production got into a broken state. Replay the exact sequence of events that led to the bug in a staging environment, or time-travel the projection to any past moment.",
    scenario:
      "A checkout silently fails at 2 AM. Replay the last 10 minutes of events for that order id against a fresh projection — the divergence shows up in the first diff.",
    bullets: [
      "Deterministic replay against any projection",
      "11.9μs p99 published Core indexed-read reference",
      "Rebuild projections without dropping data",
      "Diff two points in time to find what changed",
    ],
  },
  {
    id: "agent-memory",
    title: "AI Agent Memory",
    subtitle: "Give LLMs persistent context",
    icon: Brain,
    color: "from-green-500/20 to-green-500/5",
    description:
      "AllSource gives agents durable memory beyond the current conversation: graph nodes, vector embeddings, provenance, and temporal recall through MCP.",
    scenario:
      "Your coding agent remembers that two weeks ago you decided to drop a column. Today, when it sees that column referenced in a migration, it flags the regression before you commit.",
    bullets: [
      "55 default tenant MCP tools; 73 with fleet controls",
      "Graph + vector + compressed index in one engine",
      "Shared memory across agents, sessions, machines",
      "Embedded mode: no cloud dependency",
    ],
  },
  {
    id: "financial-history",
    title: "Financial Transaction History",
    subtitle: "Balances derived from ordered transaction events",
    icon: DollarSign,
    color: "from-yellow-500/20 to-yellow-500/5",
    description:
      "Ledgers built on mutable rows are a liability. Store every debit, credit, reversal, and correction as an immutable event and derive balances by projection — with full reconciliation built in.",
    scenario:
      "A customer disputes a charge from three months ago. You query the entity's event stream, replay the balance at each moment, and show exactly when and why it moved — with no reconstruction guesswork.",
    bullets: [
      "Append-only ledger — no destructive updates",
      "469K events/sec published Core batch-ingest reference",
      "Point-in-time balances via `as_of`",
      "Corrections are events, not rewrites",
    ],
  },
];

export default function UseCasesPage() {
  return (
    <div className="relative overflow-hidden">
      {/* Hero */}
      <Section className="relative pt-24 pb-16 text-center">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6 }}
        >
          <h1 className="text-4xl font-bold tracking-tight sm:text-6xl">
            Four jobs for an immutable event history
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            AllSource is an event store, and there are four jobs it does that a mutable database
            cannot: regulator-grade audit trails, replaying history to rebuild state, giving AI
            agents memory that survives a restart, and financial transaction history you can query
            at any past point in time.
          </p>
          <div className="mt-8 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
            <Link href="/signup" className={cn(buttonVariants({ variant: "default" }))}>
              Start 14-day trial <ChevronRight className="ml-1 h-4 w-4" />
            </Link>
            <Link
              href="https://github.com/all-source-os/all-source"
              className={cn(buttonVariants({ variant: "outline" }))}
            >
              View on GitHub
            </Link>
          </div>
        </motion.div>
      </Section>

      {/* Demo video */}
      <Section className="pb-8">
        <motion.div
          className="mx-auto max-w-3xl overflow-hidden rounded-xl border shadow-lg shadow-primary/10"
          initial={{ opacity: 0, scale: 0.95 }}
          whileInView={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.5 }}
          viewport={{ once: true }}
        >
          <video src="/assets/demo-video.mp4" className="w-full" autoPlay loop muted playsInline />
        </motion.div>
      </Section>

      {/* Use cases */}
      <Section className="pb-16">
        <div className="space-y-12">
          {useCases.map((uc, i) => (
            <motion.div
              key={uc.id}
              id={uc.id}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.05 }}
              viewport={{ once: true }}
              className="rounded-xl border p-8"
            >
              <div className="grid gap-8 md:grid-cols-5">
                <div className="md:col-span-2">
                  <div
                    className={cn(
                      "mb-4 flex h-12 w-12 items-center justify-center rounded-lg bg-gradient-to-br",
                      uc.color
                    )}
                  >
                    <uc.icon className="h-6 w-6" />
                  </div>
                  <h2 className="text-2xl font-bold">{uc.title}</h2>
                  <p className="mt-1 text-sm text-muted-foreground">{uc.subtitle}</p>
                  <p className="mt-4 text-sm text-muted-foreground">{uc.description}</p>
                </div>
                <div className="md:col-span-3">
                  <div className="mb-4 rounded-lg bg-muted/40 p-4">
                    <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
                      Example
                    </p>
                    <p className="mt-2 text-sm">{uc.scenario}</p>
                  </div>
                  <ul className="space-y-2">
                    {uc.bullets.map((b) => (
                      <li key={b} className="flex items-start gap-2 text-sm">
                        <ChevronRight className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
                        {b}
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      </Section>

      {/* Secondary signals */}
      <Section className="pb-16">
        <div className="mx-auto max-w-3xl rounded-xl border bg-muted/20 p-8 text-center">
          <h2 className="text-2xl font-bold">Not sure if you need event sourcing?</h2>
          <p className="mt-4 text-sm text-muted-foreground">
            If you can answer "yes" to any of these, an event store is a better foundation than a
            mutable database:
          </p>
          <div className="mt-6 grid gap-3 text-left sm:grid-cols-2">
            {[
              { icon: Shield, text: "Regulators can ask what you knew and when" },
              { icon: Clock, text: "Bugs need to be reproduced against past state" },
              { icon: Search, text: "Agents need memory beyond a single session" },
              { icon: DollarSign, text: "A lost or silently mutated record costs money" },
            ].map((item) => (
              <div key={item.text} className="flex items-start gap-3 rounded-lg border p-3">
                <item.icon className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
                <span className="text-sm">{item.text}</span>
              </div>
            ))}
          </div>
          <div className="mt-8">
            <Link href="/signup" className={cn(buttonVariants({ variant: "default" }))}>
              Start 14-day trial <Terminal className="ml-2 h-4 w-4" />
            </Link>
          </div>
        </div>
      </Section>
    </div>
  );
}
