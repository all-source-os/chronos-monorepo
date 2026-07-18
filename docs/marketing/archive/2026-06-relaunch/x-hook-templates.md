---
title: "X Hook Templates — 3 weekly threads (numbered / benchmark / contrarian)"
status: DRAFT
handle: "@decebal"
maps_to: "PRICING_EXPOSURE_PLAN.md §5 (weekly X cadence, 3 hook formats)"
last_updated: 2026-06-04
---

# DRAFT — Three Filled X Hook Templates (X / @decebal)

> **DRAFT for a human to post.** Nothing posted, scheduled, or sent. These are
> three ready-to-edit threads, one per week, covering the three §5 hook formats:
> **numbered**, **benchmark**, **contrarian**. Each is a full thread, not a
> one-liner. Post from `@decebal`. Numbers trace to `siteConfig`/`CLAUDE.md`
> (fact-check at bottom). The pricing-reversal contrarian thread lives in its
> own file (`x-pricing-reversal-thread.md`) — the contrarian one here is a
> second, reusable angle so you don't repeat yourself.

---

## Week A — NUMBERED hook
### "3 reasons your agent forgets"

**1/**
3 reasons your AI agent forgets everything between sessions — and why none of them are the model's fault. 🧵

**2/**
Reason 1: its "memory" is a chat log.

You're stuffing yesterday into the context window and praying. The moment the window rolls over or the process restarts, it's gone. That's not memory. That's short-term recall with amnesia on a timer.

**3/**
Reason 2: the store isn't durable.

Most agent-memory stacks keep state in RAM or a cache that evaporates on restart. AllSource Core is the opposite: every event hits a write-ahead log (CRC32 + fsync) and Parquet. Restart the process — the events are still there.

**4/**
Reason 3: recall is too slow to use on every turn.

If memory lookup costs 200-500ms, you ration it. AllSource recalls in ~12μs (11.9μs p99). At that speed you query memory on *every* message and the user never feels it. Memory stops being a tool you call and becomes something that's just there.

**5/**
Fix all three: record every event your agent emits, durably, and recall it in microseconds.

469K events/sec ingest. 43 MCP tools straight into Claude Desktop. MIT — self-host the whole thing.

Your agents already forget. Stop letting them.

$19/mo. Self-host free. MIT. → all-source.xyz

---

## Week B — BENCHMARK hook
### "12μs recall. Here's the code."

**1/**
~12μs agent-memory recall. Not milliseconds. Microseconds.

Here's the actual lookup path and how you reproduce the number yourself. 🧵

**2/**
For scale: a typical HTTP round-trip is 50-200ms. Cloud memory APIs land in the hundreds of ms. At 11.9μs (p99) the memory lookup is invisible — faster than printing the response.

**3/**
Why it's that fast: the read path is a concurrent hash-map lookup, not a query.

```
query → DashMap projection → O(1) lookup → result
        (no SQL parse, no network, no disk on the hot path)
```

Projections are rebuilt from the durable WAL on boot, so "fast" never means "lossy."

**4/**
Run it yourself:

```bash
cargo install allsource-prime
allsource-prime --data-dir ~/.prime/memory --mode http --port 3905

curl -w "time_total: %{time_total}s\n" \
  http://localhost:3905/api/v1/prime/stats
```

**5/**
Same engine does 469K events/sec on ingest and runs the whole thing in ~129MB. 43 MCP tools so Claude can read/write memory directly.

12μs recall. Here's the code. The rest is on GitHub.

$19/mo. Self-host free. MIT. → all-source.xyz

---

## Week C — CONTRARIAN hook
### "Stop putting your agent's memory in a vector DB"

**1/**
Hot take: a vector database is the wrong default for agent memory.

I'm building durable memory for AI agents, and the "just embed everything into a vector store" reflex quietly costs you the two things memory needs most. 🧵

**2/**
Thing 1 it costs you: *time*.

Vector search is a similarity scan. It's great for fuzzy retrieval, terrible as the thing you hit on every single turn. When recall is hundreds of ms, you ration it — and a memory you ration isn't memory.

**3/**
Thing 2 it costs you: *truth*.

An embedding tells you what's *similar*. It doesn't tell you what *happened*, in what *order*, at what *time*. Agents reason over sequences of events — "what did the user do, then what did I decide." That's a log, not a nearest-neighbor.

**4/**
The reframe: store the events. Project them.

AllSource keeps an immutable, durable event log (WAL + Parquet) and serves O(1) recall from in-memory projections in ~12μs. Vectors are still there when you want fuzzy search — but they're a projection, not the source of truth.

**5/**
So: events as the substrate, ~12μs exact recall on the hot path, vector search when you actually need similarity. 469K events/sec, 43 MCP tools, MIT.

Your agents already forget. Stop letting them.

$19/mo. Self-host free. MIT. → all-source.xyz

---

## Fact-check (every numeric/factual claim → source)

| Claim | Source |
|---|---|
| ~12μs recall / 11.9μs p99 | `siteConfig.stats[1]` (11.9μs p99); blog rounds to 12μs |
| 469K events/sec | `siteConfig.stats[0]` / CLAUDE.md |
| 43 MCP tools | `siteConfig.stats[2]` / CLAUDE.md |
| 129MB footprint | `siteConfig.stats[3]` |
| WAL (CRC32 + fsync) + Parquet durability | CLAUDE.md "Architecture Facts" |
| DashMap O(1) in-memory reads | CLAUDE.md; blog `12-microsecond-agent-memory.mdx` |
| `cargo install allsource-prime` + curl benchmark | blog `12-microsecond-agent-memory.mdx` ("Try It") |
| Indie $19 / Self-Host free / MIT closer | `siteConfig.pricing`, plan §5 |

**Voice:** first-person founder POV (§5), closer + tagline consistent with §4.
