/**
 * Per-competitor data for the `/vs/<slug>` comparison pages.
 *
 * DRY contract: the three pages (`mem0`, `letta`, `zep`) are thin shells over
 * `<ComparisonTable>` + this map. Change a claim here and all pages update.
 *
 * Claim discipline (see CLAUDE.md + prompt 013):
 *   - AllSource cells are sourced from product facts: durable WAL+Parquet event
 *     store, 11.9μs recall, 469K events/sec, MIT/self-host, x402 per-call
 *     pricing, 43 MCP tools. These are defensible against our own docs.
 *   - Competitor cells mirror the homepage matrix (`sections/social-proof.tsx`)
 *     where that matrix took a position, and otherwise read "unknown" / "varies"
 *     rather than inventing a number. NEVER fabricate competitor figures.
 *   - The homepage matrix asserted: temporal `as_of` — zep yes, mem0/letta no;
 *     full event provenance / compressed index / offline-embedded / sub-ms
 *     recall — AllSource only; MIT self-host — mem0 yes, letta/zep no.
 */

/** A table cell. Booleans render as check/cross; strings render verbatim. */
export type Cell = "yes" | "no" | "partial" | "unknown" | "varies" | (string & {});

export type ComparisonRow = {
  feature: string;
  allsource: Cell;
  competitor: Cell;
  /** One honest sentence of context shown under the feature name. */
  note?: string;
};

export type FaqEntry = { question: string; answer: string };

export type Competitor = {
  slug: "mem0" | "letta" | "zep";
  /** Display name used in H1, table header, metadata. */
  name: string;
  /** One-line, defensible verdict shown under the H1. */
  verdict: string;
  /** Meta description for `constructMetadata`. */
  metaDescription: string;
  /** "Pick AllSource if…" bullets. */
  pickAllsource: string[];
  /** "Pick <Competitor> if…" bullets — kept honest, never disparaging. */
  pickCompetitor: string[];
  rows: ComparisonRow[];
  faqs: FaqEntry[];
};

// Rows shared by every competitor — the durable differentiators. Per-competitor
// cells are filled below so each page can diverge where the homepage matrix did.
function baseRows(competitorCells: Record<string, Cell>): ComparisonRow[] {
  return [
    {
      feature: "Durable event store (survives restart)",
      allsource: "WAL + Parquet",
      competitor: competitorCells.durable ?? "unknown",
      note: "AllSource Core is the database: a Rust WAL (CRC32, fsync) with columnar Parquet persistence. Event data survives restarts.",
    },
    {
      feature: "Full event provenance / replay",
      allsource: "yes",
      competitor: competitorCells.provenance ?? "no",
      note: "Every change is an immutable event you can replay to reconstruct any past state.",
    },
    {
      feature: "Temporal / as_of queries",
      allsource: "yes",
      competitor: competitorCells.temporal ?? "unknown",
      note: "Point-in-time projections are a first-class query, not a snapshot you have to manage yourself.",
    },
    {
      feature: "Recall latency",
      allsource: "11.9μs (p99)",
      competitor: competitorCells.latency ?? "unknown",
      note: "DashMap-backed in-memory reads over the durable log. Cloud memory APIs typically measure recall in tens of milliseconds.",
    },
    {
      feature: "Ingestion throughput",
      allsource: "469K events/sec",
      competitor: competitorCells.throughput ?? "unknown",
      note: "Lock-free Rust core. We do not publish competitor throughput figures we cannot verify.",
    },
    {
      feature: "Offline / embedded mode",
      allsource: "yes",
      competitor: competitorCells.embedded ?? "no",
      note: "Run AllSource in-process as a library — no separate server or network hop required.",
    },
    {
      feature: "MCP tools for AI agents",
      allsource: "43 tools",
      competitor: competitorCells.mcp ?? "unknown",
      note: "A Model Context Protocol server ships out of the box for Claude and other MCP clients.",
    },
    {
      feature: "x402 per-call agent payments",
      allsource: "yes",
      competitor: competitorCells.x402 ?? "no",
      note: "Native x402 micropayments let agents pay per call instead of negotiating a seat-based contract.",
    },
    {
      feature: "License / self-host",
      allsource: "MIT, self-host",
      competitor: competitorCells.license ?? "unknown",
      note: "Run the whole stack yourself for free, or use the hosted tiers.",
    },
  ];
}

export type CompetitorSlug = "mem0" | "letta" | "zep";

// Keyed by the literal slug union (not `Record<string, …>`) so lookups like
// `competitors.mem0` are non-optional under noUncheckedIndexedAccess.
export const competitors: Record<CompetitorSlug, Competitor> = {
  mem0: {
    slug: "mem0",
    name: "mem0",
    verdict:
      "mem0 is a memory layer that bolts onto your LLM app. AllSource is a durable event store underneath it — you keep full provenance and time-travel instead of a lossy summary.",
    metaDescription:
      "AllSource vs mem0: a durable WAL+Parquet event store with 11.9μs recall, full event provenance, embedded mode, 43 MCP tools, and x402 per-call pricing vs a managed LLM memory layer. Honest, sourced comparison.",
    pickAllsource: [
      "You need every memory write to be auditable and replayable, not summarized away",
      "You want microsecond recall over a durable log, not a network round-trip to a memory API",
      "You want to embed the store in-process or self-host it for free under MIT",
      "Your agents pay per call (x402) instead of per seat",
    ],
    pickCompetitor: [
      "You want a managed memory abstraction and are happy to self-host the open-source core",
      "You do not need full event provenance or point-in-time replay",
      "A higher-level 'remember this' API matters more to you than the underlying store",
    ],
    rows: baseRows({
      // Homepage matrix: mem0 is MIT/self-host = yes; temporal/provenance = no.
      durable: "unknown",
      provenance: "no",
      temporal: "no",
      latency: "unknown",
      throughput: "unknown",
      embedded: "no",
      mcp: "unknown",
      x402: "no",
      license: "MIT, self-host",
    }),
    faqs: [
      {
        question: "Is mem0 better than AllSource?",
        answer:
          "They solve different layers. mem0 is a managed memory abstraction for LLM apps; AllSource is a durable event store (WAL + Parquet) with full provenance, 11.9μs recall, and time-travel queries. If you need auditable, replayable memory you keep yourself, AllSource is the stronger base. If you want a hosted 'remember this' API and do not need provenance, mem0 may be enough.",
      },
      {
        question: "Can I self-host both AllSource and mem0?",
        answer:
          "Yes. AllSource is MIT licensed and self-hostable, and mem0's core is open source. AllSource additionally ships an embedded in-process mode and a 43-tool MCP server out of the box.",
      },
      {
        question: "Does AllSource keep full event history like mem0?",
        answer:
          "AllSource keeps the complete, immutable event log and lets you replay it to any past state. Memory layers like mem0 typically store distilled memories rather than the full provenance.",
      },
    ],
  },

  letta: {
    slug: "letta",
    name: "Letta",
    verdict:
      "Letta (formerly MemGPT) gives agents a stateful memory loop. AllSource gives that loop a durable, queryable foundation — full event history with 11.9μs recall instead of memory you have to manage in-context.",
    metaDescription:
      "AllSource vs Letta (MemGPT): a durable WAL+Parquet event store with 11.9μs recall, full provenance, embedded mode, 43 MCP tools, and x402 pricing vs a stateful agent framework. Sourced, no fabricated numbers.",
    pickAllsource: [
      "You want a durable store of record under your agents, not just an in-context memory manager",
      "You need point-in-time queries and full replay for audit or debugging",
      "You want microsecond recall and the option to embed or self-host under MIT",
      "You want per-call (x402) economics for autonomous agents",
    ],
    pickCompetitor: [
      "You want an opinionated agent runtime with memory management built in",
      "You are standardizing on Letta's agent abstractions and tooling",
      "You do not need a separate durable event store of record",
    ],
    rows: baseRows({
      // Homepage matrix: letta temporal/provenance/MIT-self-host = no.
      durable: "unknown",
      provenance: "no",
      temporal: "no",
      latency: "unknown",
      throughput: "unknown",
      embedded: "no",
      mcp: "unknown",
      x402: "no",
      license: "unknown",
    }),
    faqs: [
      {
        question: "Is Letta better than AllSource?",
        answer:
          "Letta is an agent framework with a stateful memory loop; AllSource is the durable event store that can sit underneath it. Letta wins if you want a batteries-included agent runtime. AllSource wins if you need an auditable, replayable store of record with 11.9μs recall and time-travel queries. Many teams use a store like AllSource as the persistence layer beneath a framework like Letta.",
      },
      {
        question: "What is the difference between Letta and AllSource?",
        answer:
          "Letta orchestrates an agent and its working memory. AllSource is infrastructure: a Rust WAL + Parquet event store with full provenance, embedded mode, and a 43-tool MCP server. They operate at different layers and can be combined.",
      },
      {
        question: "Does AllSource replace Letta?",
        answer:
          "Not directly. AllSource replaces the durable persistence and recall layer; you can keep a framework like Letta on top, or drive AllSource directly over HTTP and MCP.",
      },
    ],
  },

  zep: {
    slug: "zep",
    name: "Zep",
    verdict:
      "Zep adds a temporal memory service for chat history. AllSource makes temporal a property of the whole store — every event is replayable, with 11.9μs recall and an embedded or self-hosted deployment.",
    metaDescription:
      "AllSource vs Zep: a durable WAL+Parquet event store with 11.9μs recall, full provenance, embedded mode, 43 MCP tools, and x402 pricing vs a temporal memory service for LLM apps. Honest, sourced comparison.",
    pickAllsource: [
      "You want temporal queries across all your data, not only chat memory",
      "You need full event provenance and replay, plus microsecond recall",
      "You want to embed in-process or self-host the whole stack under MIT",
      "You want per-call (x402) agent economics out of the box",
    ],
    pickCompetitor: [
      "You specifically want a managed memory service tuned for conversational history",
      "You are happy with a hosted temporal memory API and do not need an embedded store",
      "You do not need a general-purpose event store beyond agent memory",
    ],
    rows: baseRows({
      // Homepage matrix: zep temporal = yes; provenance/compressed-index/
      // embedded/sub-ms/MIT-self-host = no.
      durable: "unknown",
      provenance: "no",
      temporal: "yes",
      latency: "unknown",
      throughput: "unknown",
      embedded: "no",
      mcp: "unknown",
      x402: "no",
      license: "unknown",
    }),
    faqs: [
      {
        question: "Is Zep better than AllSource?",
        answer:
          "Zep is a temporal memory service focused on conversational history; AllSource is a general-purpose durable event store where temporal queries apply to all your data. Zep wins if you only need managed chat memory. AllSource wins if you want full event provenance, 11.9μs recall, embedded mode, and the option to self-host under MIT.",
      },
      {
        question: "Does Zep support temporal queries like AllSource?",
        answer:
          "Zep offers temporal memory for chat history. AllSource makes temporal a first-class property of the entire event log — any past state of any entity is reconstructable via as_of projections.",
      },
      {
        question: "Can I self-host AllSource instead of using a hosted memory service?",
        answer:
          "Yes. AllSource is MIT licensed, runs embedded in-process or as a server, and has hosted tiers if you prefer managed. You are never locked into a single hosted memory API.",
      },
    ],
  },
};

export const competitorSlugs = Object.keys(competitors) as CompetitorSlug[];
