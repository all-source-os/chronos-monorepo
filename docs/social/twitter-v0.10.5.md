---
status: published
---

# Twitter Thread — AllSource Chronos v0.10.5

## Tweet 1/7 — Hook

AllSource Chronos v0.10.5 just dropped.

Server-side projections. No more folding events in your client. Query computed state directly.

Here's everything in the release 🧵

`#opensource #eventsourcing #elixir`

📷 *Image: Chronos logo or a diagram showing "Events → Projection Engine → Computed State"*

---

## Tweet 2/7 — The Problem

Before v0.10.5, every client had to:

1. Query raw events
2. Fold them into state locally
3. Handle snapshots, ordering, deduplication

Same boilerplate in every language. Same bugs. Same latency.

We moved all of that server-side.

📷 *Image: before/after code comparison — client-side fold (10+ lines) vs single projected query call*

---

## Tweet 3/7 — Projection Engine

New in the Query Service:

→ Projection behaviour + registry
→ Fold pipeline with snapshot-aware replay
→ Continuous projections via PubSub (ProjectionServer + DynamicSupervisor)
→ ETS read path with fold-on-read fallback
→ Strong consistency opt-in

One endpoint: POST /api/query/projected

`#elixir #otp #erlang`

📷 *Image: architecture diagram showing ProjectionServer → ETS cache → fold-on-read fallback*

---

## Tweet 4/7 — Built-in Projections

Ships with 5 projection modules out of the box:

• IndexState — composite index tracking
• TradeState — trade lifecycle
• PortfolioState — holdings aggregation
• SagaState — long-running process coordination

All implementing the same behaviour. Add your own in ~40 lines.

📷 *Image: code snippet of one projection module (e.g., IndexState) showing the behaviour callbacks*

---

## Tweet 5/7 — SDK Consolidation

Cleaned up the monorepo:

→ Rust SDK added to sdks/rust/ (circuit breaker, client-side fold, full test suite)
→ TypeScript SDK moved from packages/ to sdks/typescript/
→ Legacy SDK copies in packages/ deleted
→ New MONOREPO_STRUCTURE.md documenting the rules

Four SDKs, one directory. No more duplicates.

`#rustlang #typescript #monorepo`

📷 *Image: directory tree showing sdks/ with rust/, go/, python-client/, typescript/*

---

## Tweet 6/7 — Wire Format + MCP Fixes

Wire format standardized across all list endpoints:

```json
{"data": [...], "count": N, "total": N}
```

Consistent. Predictable. Every controller.

Also fixed MCP client resilience — disabled Hackney connection pooling so stale connections don't break after Core restarts.

📷 *Image: JSON response example showing the standardized format*

---

## Tweet 7/7 — CTA

v0.10.5 means your clients get simpler and your queries get faster. The server does the fold. You get the state.

⭐ github.com/all-source-os/chronos-monorepo

`#opensource #eventdriven #eventsourcing #rust #elixir #database`

📷 *Image: GitHub repo social preview card*
