# Neotoma vs. AllSource: Two Designs for Agent Memory

*Date: 2026-05-23*
*Status: Internal competitive analysis — drives the AllSource roadmap. Do not publish as-is.*

---

## TL;DR

Neotoma and AllSource are aimed at the same problem — durable, queryable memory for AI agents — with two genuinely different design philosophies:

- **Neotoma** says: *consistency above all else*. Observations are append-only; entity state is recomputed every time from a built-in deterministic reducer with four declarative merge policies. No user code in the hot path. Local-first SQLite. Schema-bound entity types. The pitch is reproducibility and provenance.

- **AllSource** says: *the event log is the truth*. Events are append-only; you project any view you want from them — graph (Prime), vectors (Prime recall), snapshots, projections. WAL + Parquet + DashMap, multi-tenant hosted Core with sub-microsecond reads. The pitch is performance and composability.

The overlap is much bigger than I expected before doing this comparison. The genuine gaps Neotoma exposes are real but addressable — most are ergonomics rather than missing infrastructure.

---

## What Neotoma is (verbatim from their docs)

> *"Neotoma's architecture is built on three foundations: append-only observation logs for immutability, deterministic reducers for consistent state composition, and schema-bound entity types for structural guarantees."*

Seven first-class primitives:

| Primitive | What it is |
|---|---|
| **Observation** | Granular immutable fact from a raw source (with provenance) |
| **Source** | Content-addressed, deduplicated raw data (file, JSON, text) |
| **Reducer** | Built-in deterministic function: merges observations → entity snapshot |
| **Entity snapshot** | Current truth for an entity, derived via reduction, per-field provenance |
| **Relationship** | Typed edge between entities (SETTLES, REFERS_TO, etc.) |
| **Schema** | Entity type definition; structural guarantees enforced at rest |
| **Memory graph** | Unified structure of entities + observations + sources + events |

Merge policies (declarative, **no inline code**):

- **last_write** — most recent observation by `observed_at` (status, amount, address)
- **highest_priority** — highest `source_priority` (user correction beats AI extraction)
- **most_specific** — highest `specificity_score` (dense schema-aligned > shallow)
- **merge_array** — union values (aliases, tags)

Deployment: SQLite local-first by default; CLI + REST + MCP + Inspector. Cross-tool integrations marketed: Claude, ChatGPT, Cursor, OpenCode, OpenClaw, IronClaw. Stage: "developer preview."

Notably absent from their feature set:
- **Vectors** — explicitly not present (Neotoma positions itself as the *anti*-RAG)
- **Multi-tenancy** — not a thing (local-first)
- **Hosted commercial offering** — not yet (preview)
- **Sub-millisecond read claims** — no perf numbers

---

## What AllSource is (refresher for completeness)

- **Core**: Rust event store. WAL (CRC32, fsync) + Parquet (Snappy) + DashMap. Multi-tenant. Documented 11.9μs reads and 469K events/sec throughput. Source of truth for events. Hosted at `api.all-source.xyz`.
- **Prime**: agent memory engine on top of Core. Graph + vectors (fastembed AllMiniLML6V2, 384d) + temporal recency. MCP server (stdio or HTTP). Local data dir + optional `--sync-to` to push to hosted Core.
- **Query Service**: API gateway. Auth, rate-limiting, billing.
- **SDKs**: Rust + TypeScript + Python + Go.
- **Deployment**: Fly.io for backend services, Vercel for the dashboard.
- **Stage**: shipped, in production, with paying tenants.

---

## Where each design wins (head-to-head)

| Dimension | Neotoma | AllSource | Edge |
|---|---|---|---|
| **Append-only event log** | ✓ observations | ✓ events | Tie |
| **Time-travel queries** | ✓ per-entity snapshots over versions | ✓ event-log range queries + as-of reads | **AllSource** (full event log, not just per-entity) |
| **Per-field provenance** | ✓ first-class (observation → snapshot field) | ✓ first-class `provenance(node, field)` → source event id/time/policy, on MCP + REST + all four SDKs *(shipped 2026-05/06)* | Tie |
| **Declarative merge policies** | ✓ four built-in policies, no user code | ✓ same four (`last_write`/`highest_priority`/`most_specific`/`merge_array`), no user code, on MCP + REST + SDKs *(shipped)* | Tie |
| **Schema enforcement at write** | ✓ schema-bound entity types | ✓ opt-in per-tenant `permissive`/`warn`/`strict`, dashboard toggle + gateway API *(shipped)* | Tie |
| **Knowledge graph** | ✓ relationships first-class | ✓ Prime graph (nodes + edges) | Tie |
| **Vector search** | ✗ explicitly absent | ✓ Prime vectors + hybrid recall | **AllSource** |
| **MCP integration** | ✓ MCP server | ✓ allsource-prime MCP server + .dxt | Tie |
| **Cross-tool persistence (same memory across Claude/ChatGPT/Cursor)** | ✓ marketed explicitly | ✓ works (MCP is MCP), but not marketed | Tie — Neotoma wins on positioning |
| **Local-first** | ✓ default | ✓ supported (Prime stdio) | Tie |
| **Hosted multi-tenant** | ✗ tunnel only | ✓ production, with auth + billing + quotas | **AllSource** |
| **Sub-millisecond reads** | ✗ SQLite-bound | ✓ documented 11.9μs | **AllSource** |
| **Production throughput claims** | ✗ none | ✓ 469K events/sec | **AllSource** |
| **Pre-built entity templates** | ✓ contacts, tasks, events, transactions, contracts, decisions | ✗ schema-agnostic | **Neotoma** |
| **Multi-language SDKs** | ✗ REST + CLI only | ✓ Rust/TS/Py/Go | **AllSource** |
| **Stage / maturity** | Developer preview | Shipping, paid tenants | **AllSource** |
| **Public pricing / quotas** | ✗ not disclosed | ✓ free tier + paid | **AllSource** |

---

## What Neotoma's design philosophy gets right (where we should learn)

Stripping the marketing, Neotoma made three load-bearing design choices that produce genuine ergonomic wins for the agent-memory use case:

### 1. Deterministic reducers as a built-in primitive

Today in AllSource, if you want "current state of entity X," you either:
- Query the latest event for that entity and trust the payload (works if every write is a full-state replace), OR
- Write a projection that folds events into a snapshot (correct, but it's *your* code)

Neotoma collapses this: the reducer is a built-in function, the merge policy is a per-field declarative choice from four options, and the snapshot is always reproducible. No projection code to maintain, no risk of the projection drifting from the events.

This is genuinely a better ergonomic for the 80% case (user writes facts, agent reads current state, no custom aggregation logic). AllSource's flexibility wins on the 20% case (custom aggregations, time-window reductions, cross-entity rollups) but it's heavier for the common path.

### 2. Per-field provenance as a first-class read

When the system can say *"Entity contact:alice.email was set to alice@example.com by observation obs_42 from source slack_message_99 on 2026-03-14"*, that's an audit and debugging primitive nothing else in our space offers cleanly.

AllSource has the raw material — every event is preserved with timestamps and (optional) metadata. But assembling "which event was responsible for field X of entity Y as of time T?" requires user code that filters and folds events. It's not a primitive.

### 3. Declarative merge policies covering the common cases

Four strategies (`last_write`, `highest_priority`, `most_specific`, `merge_array`) cover most of what users actually want when reconciling observations from different sources. They're declarative — no inline code — which makes them safe to compose and reason about.

This is small but matters: it turns "I need to write a CRDT merge function" into "I pick from a dropdown." Even users who could write the merge function would rather not.

---

## Gaps in AllSource that Neotoma exposes

> **Status (2026-06): Gaps 1–6 are all shipped.** Gap 1 (declarative projection primitive), Gap 2 (per-field provenance), and Gap 3 (schema enforcement) are now reachable on every surface — prime-mcp tools, Core REST under `/api/v1/prime/*`, the gateway, and all four SDKs — plus a per-tenant enforcement toggle (`/api/v1/tenants/{id}/schema-enforcement` → dashboard). Gap 4 templates ship as MCP guides (enforced-schema registration deferred until a tenant asks). Gaps 5–6 (cross-tool-sync marketing, `/compare/agent-memory`, plus a per-tool `/install` hub) shipped. Per-commit record: `docs/proposals/NEOTOMA_PARITY_COMPLETION_PLAN.md`. The gap descriptions below are kept as the original analysis.

These aren't existential — none of them is "AllSource doesn't work." They're places where Neotoma's framing makes us look heavier than we need to. In priority order:

### Gap 1 — Built-in deterministic projection primitive with declarative merge policies

**What it'd look like:** A new Prime primitive `prime_define_projection(entity_type, field_merge_policies)` where `field_merge_policies` is a map of `field_name` → one of `{last_write, highest_priority, most_specific, merge_array, custom_fn}`. The projection runs deterministically over the event stream filtered to that entity_type; the snapshot is recomputable at any time.

**Why it matters:** This is Neotoma's headline differentiator. If a user evaluates both and the question is "do I have to write a projection?" Neotoma's answer is no, ours is yes.

**Effort:** Medium. The folding logic is straightforward; the API surface and the schema for "what fields exist on this entity type" need design.

**Risk if we don't ship it:** Users picking a memory system for the first time will find Neotoma's path-of-least-resistance more appealing. Loses on the "what do I have to write to get started?" axis.

### Gap 2 — Per-field provenance as a first-class query

**What it'd look like:** A new endpoint `GET /api/v1/prime/nodes/{id}/fields/{field}/provenance?as_of=...` that returns the event(s) responsible for the current value of a specific field, with timestamps and metadata.

**Why it matters:** Debugging agent memory today means scanning event history manually. Provenance-per-field is a power feature for the audit-driven personas (compliance, finance, regulated industries) that AllSource Core is well-positioned for.

**Effort:** Medium-small. The data is already there in the WAL; this is an index + query.

**Risk if we don't ship it:** Niche-but-real loss to Neotoma in compliance-adjacent sales conversations.

### Gap 3 — Entity schema enforcement at write time

**What it'd look like:** Make Core's existing `/schemas` endpoint **enforcing** rather than advisory. Configurable per-tenant: strict mode (reject events that don't match the registered schema for the entity_type), permissive mode (current behavior — anything goes).

**Why it matters:** Neotoma's "schema-bound entity types" pitch implies discipline at the storage layer. AllSource has the registry but doesn't enforce. For tenants who want strictness, they should be able to opt into it without rolling their own validation layer.

**Effort:** Small. The schema registry exists; ingest needs a new validation step.

**Risk if we don't ship it:** Loss in conversations where the buyer wants "the system enforces the model, not the agent."

### Gap 4 — Pre-built entity templates

**What it'd look like:** A library of common entity type schemas — `contact`, `task`, `event`, `transaction`, `decision`, `meeting`, `document` — that users can register with one call. Like database migrations but for Prime entity types.

**Why it matters:** Neotoma ships these out of the box. AllSource's "tell Claude as you go" pitch is honest but slower to first useful query. A user opening AllSource for the first time stares at an empty graph and has to decide what "contact" means; a user opening Neotoma sees `contact` already exists.

**Effort:** Small. A library of JSON schema files + a one-call registration endpoint.

**Risk if we don't ship it:** Slower time-to-value, weaker landing-page demos.

### Gap 5 — Marketing: lead with cross-tool sync

**What it'd look like:** A page (or section on `/prime`) explicitly positioned as "same memory across Claude Desktop, Cursor, ChatGPT, OpenCode" — with a setup guide for each MCP client. Mirror the breadth Neotoma advertises.

**Why it matters:** AllSource Prime already does this — any MCP-compliant client can use the same hosted Prime. We just don't say so on the landing page. Neotoma does, prominently. This is the cheapest win on the list.

**Effort:** Pure marketing.

**Risk if we don't ship it:** We get out-positioned on a feature we already have.

### Gap 6 — Honest competitor-comparison page on the marketing site

**What it'd look like:** `/compare/agent-memory` — a 5-way comparison of platform memory, RAG (Mem0/Zep), file-based, database CRUD, and "event-sourced" (us). Adapt Neotoma's 5-axis breakdown to put AllSource in its lane.

**Why it matters:** Neotoma's "memory models" page is good SEO and good honest framing. It hands buyers a vocabulary that puts Neotoma in the favorable square. We should do the same for ourselves before someone else maps the space.

**Effort:** Half-day writing + design.

**Risk if we don't ship it:** Buyers comparing Neotoma vs Mem0 vs platform memory don't even consider AllSource because we're not in any of their mental categories.

---

## Where AllSource should defend, not chase

A few capabilities of Neotoma's design are not gaps we should close — they reflect different bets:

- **No vectors by design.** Neotoma's bet is that deterministic schema lookups + graph beat semantic similarity for production agent memory. Ours is that hybrid recall (vectors + graph + recency) is strictly better. Don't drop vectors.
- **SQLite-local by default.** Neotoma optimizes for the local case; we optimize for the hosted multi-tenant case. Don't pivot to SQLite — but **do** keep Prime's local mode first-class so the developer-on-laptop story stays competitive.
- **Developer preview maturity.** Their "we're an alpha product" framing earns them latitude on perf and missing features. We're past that; our pitch is "shipped, paid tenants." Don't dilute it.

---

## Recommended sequencing

If we ship the 6 gaps above, the ones that move the most ground per unit effort:

1. **Gap 5 (marketing — cross-tool sync)** — half-day. Cheapest. Closes the positioning gap.
2. **Gap 6 (comparison page)** — half-day. Pairs with #5. SEO + framing.
3. **Gap 4 (entity templates library)** — 1-2 days. Improves first-30-seconds UX measurably.
4. **Gap 3 (schema enforcement opt-in)** — 2-3 days. Unblocks regulated-industry conversations.
5. **Gap 1 (declarative projection primitive)** — 1-2 weeks. Neutralizes Neotoma's headline differentiator.
6. **Gap 2 (per-field provenance query)** — 1 week. Niche but defensible feature.

Total to fully close the ergonomic gap: ~3-4 weeks of focused work, with marketing wins (#5, #6) shippable inside a week.

---

## Conclusion: who picks what

If a buyer is evaluating Neotoma vs AllSource today, the honest answer is:

- **Pick Neotoma if:** you want strictly local-first (SQLite, no account), you don't need vector search, and you're OK being early on a preview product. That's now the whole list — the declarative-merge-policy and provenance ergonomics it used to win on are matched (see below).
- **Pick AllSource if:** you need hosted multi-tenant, you want vector + graph + temporal hybrid recall, you're scale-sensitive (469K events/sec matters to you), you want SDKs in your language of choice, you want the deterministic primitives (declarative projections, per-field provenance, opt-in schema enforcement) reachable from REST/SDK and not just MCP, or you're already in production and need a system that's also in production.

**Status update (2026-06):** Gaps 1, 2, 3 are shipped, and on **every surface** — the declarative merge policies, `provenance(node, field)`, and per-tenant schema enforcement are now reachable via the prime-mcp tools, Core REST (`/api/v1/prime/*`, `/api/v1/tenants/{id}/schema-enforcement`), the gateway, and all four SDKs, with a dashboard enforcement toggle. The three rows Neotoma used to win (per-field provenance, declarative merge policies, schema enforcement) are now Tie. The "pick Neotoma if" list has collapsed to the local-first / no-vectors lane, exactly as predicted. See `docs/proposals/NEOTOMA_PARITY_COMPLETION_PLAN.md` for the per-commit record.

---

## Open questions for follow-up

- Is Neotoma's "deterministic memory" framing going to become the dominant vocabulary in agent-memory marketing? If so, we should adopt parts of it (provenance, reproducibility) rather than fight it.
- Should we publish a *public* short-form version of this comparison, the way we did with Oracle 26ai? Or keep it internal until the gaps are closed?
- Pricing comparison is missing because Neotoma hasn't disclosed prices. Revisit once they do.
