---
title: "Repurpose kit — '12μs Agent Memory: How We Got There'"
status: DRAFT
source_post: "apps/web/content/12-microsecond-agent-memory.mdx"
maps_to: "PRICING_EXPOSURE_PLAN.md §5 (1 blog → 1 X thread + 1 LinkedIn long-form + 1 60s short)"
last_updated: 2026-06-04
---

# DRAFT — Repurpose Kit

> **DRAFT for a human to post.** Nothing posted, scheduled, or sent. Source post:
> `apps/web/content/12-microsecond-agent-memory.mdx` ("12μs Agent Memory: How We
> Got There"). Three outputs below: X thread, LinkedIn long-form, 60-second
> video script. Founder voice (`@decebal`). Numbers trace to `siteConfig`,
> `CLAUDE.md`, and the source post (fact-check at bottom).
>
> **Note on the headline number:** the blog post and brand say "12μs"; the
> canonical `siteConfig` figure is **11.9μs p99** recall and an **11.2μs** demo
> stamp. Drafts use "~12μs" for the hook and cite 11.9μs p99 / 11.2μs where the
> precise number matters.

---

## 1. X thread (benchmark format)

**1/**
We recall agent memory in ~12μs. Not milliseconds — microseconds (11.9μs p99).

A typical HTTP round-trip is 50-200ms. Cloud memory APIs land in the hundreds of ms. Here's why ours is ~10,000× faster — and how you reproduce it. 🧵

**2/**
The trick isn't a faster query engine. It's *not querying*.

```
query → DashMap projection → O(1) lookup → result
        (no SQL parse, no network, no disk on the hot path)
```

Every projection is a concurrent hash map. A recall is a hash lookup. That's the whole secret.

**3/**
"In-memory" usually means "lost on restart." Not here.

```
event → CRC32 → WAL append → fsync batch → DashMap insert → projections
```

Events are durable on a write-ahead log + Parquet. The fast in-memory projections are *rebuilt from the WAL on boot*. Fast never means lossy.

**4/**
Why microseconds matter for agents:

If memory lookup costs 200-500ms, you ration it — you only check memory when you "need" to. At ~12μs you check on *every* turn and the user never notices. Memory stops being a tool the agent calls and becomes something that's just there.

**5/**
Same engine, same WAL:
• 469K events/sec ingest
• ~129MB footprint for the whole stack
• 43 MCP tools so Claude reads/writes memory directly

**6/**
Reproduce the recall number yourself:

```bash
cargo install allsource-prime
allsource-prime --data-dir ~/.prime/memory --mode http --port 3905

curl -w "time_total: %{time_total}s\n" \
  http://localhost:3905/api/v1/prime/stats
```

**7/**
Full write-up — DashMap shards, HNSW vector index, the projection stack — is on the blog.

Your agents already forget. Stop letting them.

$19/mo. Self-host free. MIT. → all-source.xyz

---

## 2. LinkedIn long-form

**Your AI agent's memory is probably 10,000× slower than it needs to be.**

I've been building durable memory for AI agents, and one number reframed the
whole problem for me: recall latency.

Most agent-memory stacks answer a "what do I know about X?" lookup in hundreds of
milliseconds — a cloud vector API, a round-trip, a similarity scan. That sounds
fine until you realize where the lookup happens: in the hot path, between the
user's message and the agent's reply. Every millisecond there is latency the user
feels. So teams ration memory — they only check it when they think they need to.

A memory you ration isn't really memory.

We went the other way. AllSource recalls in **~12μs** (11.9μs p99). For scale, a
typical HTTP round-trip is 50–200ms. At microseconds, the lookup is invisible —
faster than printing the response — so you can query memory on *every single
turn*. That's the difference between "memory is a tool the agent calls" and
"memory is always there."

Two design choices get you there:

**1. Don't query — look up.** Every projection is a DashMap (Rust's concurrent,
sharded hash map). A recall is an O(1) hash lookup: no SQL parsing, no query
planning, no network, no disk on the hot path.

**2. Fast doesn't mean fragile.** The thing people assume about in-memory speed
is that you lose it on restart. You don't. Every event is written to a
write-ahead log (CRC32 checksums, batched fsync) and compacted to Parquet. The
in-memory projections are rebuilt from that durable log on boot. Durability lives
on disk; speed lives in memory; you get both.

The same engine ingests **469K events/sec** and runs the whole stack in **~129MB**,
with **43 MCP tools** so Claude can read and write memory directly. It's
MIT-licensed — you can self-host the whole thing for free.

If you're building agents that need to remember across sessions, I'd start by
measuring your current recall latency. If it's in the hundreds of milliseconds,
that number is quietly shaping every product decision you make about memory.

Your agents already forget. Stop letting them.

Self-host free (MIT), or hosted from $19/mo → all-source.xyz

*(Full engineering write-up — DashMap shards, the projection stack, the HNSW
vector index — on the AllSource blog.)*

#AIAgents #EventSourcing #Rust #BuildInPublic

---

## 3. 60-second short-form video script (screen recording)

**Format:** screen recording, founder voiceover (`@decebal`). Target 55–60s.
Vertical (9:16) for Shorts/Reels; also export 16:9 for X.

| Time | On screen | Voiceover |
|---|---|---|
| 0:00–0:05 | Terminal, cursor blinking. Title card: "~12μs agent memory." | "Your AI agent forgets everything between sessions. Let me show you the fix — and it's fast." |
| 0:05–0:14 | Type & run: `allsource-prime --data-dir ~/.prime/memory --mode http --port 3905`. Server boots, prints "projections rebuilt from WAL". | "This is AllSource. It boots and rebuilds its memory from a durable write-ahead log — so nothing's lost on restart." |
| 0:14–0:26 | Split view. Left: JSON events stream in (`user.signed_up`, `cart.checkout`, `agent.decided`). Right: an MCP client / Claude Desktop pane. | "Every event the agent emits gets recorded. Durable. On disk. Forty-three MCP tools mean Claude reads and writes this memory directly." |
| 0:26–0:40 | In the Claude pane, type: "What did the user do yesterday at 3pm?" Answer renders from the events. Overlay stamp: **"returned in 11.2μs ✓"**. | "Now I ask it what happened yesterday at 3pm. It answers — straight out of the events — in about eleven microseconds. That's faster than printing the reply." |
| 0:40–0:50 | Run the curl benchmark: `curl -w "time_total: %{time_total}s\n" http://localhost:3905/api/v1/prime/stats`. Tiny number prints. Overlay: "469K events/sec · ~129MB · MIT". | "Same engine does 469 thousand events a second, runs in about a hundred and twenty-nine megs, and it's MIT — self-host the whole thing." |
| 0:50–0:60 | End card: "Your agents already forget. Stop letting them." then "$19/mo · Self-host free · MIT · all-source.xyz". | "Your agents already forget. Stop letting them. Nineteen bucks a month, or self-host for free. Link below." |

**Shot-list notes:**
- The 0:26–0:40 recall moment is the hero beat — same content as the homepage §4 right pane and the PH hero asset. Reuse one recording across all three.
- Keep the "11.2μs ✓" stamp legible; that's the wow.
- If x402 is the feature you want to spotlight instead, swap 0:40–0:50 for a live paid agent request (x402 pay-per-call) and keep the rest.

---

## Fact-check (every numeric/factual claim → source)

| Claim | Source |
|---|---|
| ~12μs recall / 11.9μs p99 | `siteConfig.stats[1]`; blog `12-microsecond-agent-memory.mdx` |
| 11.2μs demo stamp | `siteConfig.recallLatency` |
| 469K events/sec | `siteConfig.stats[0]` / CLAUDE.md / blog |
| ~129MB footprint | `siteConfig.stats[3]` / CLAUDE.md |
| 43 MCP tools | `siteConfig.stats[2]` / CLAUDE.md |
| DashMap O(1), no SQL/network/disk on hot path | blog (DashMap section); CLAUDE.md |
| WAL (CRC32 + fsync) + Parquet, projections rebuilt on boot | CLAUDE.md "Architecture Facts"; blog "Projection Stack" |
| HNSW vector index | blog ("Vector Search") |
| HTTP round-trip 50–200ms (comparison) | blog (intro / comparison table) |
| `cargo install allsource-prime` + curl benchmark | blog ("Try It") |
| $19/mo / Self-host free / MIT | `siteConfig.pricing` |
| all-source.xyz | `siteConfig.url` |

**Voice:** first-person founder (§5); closer + "Your agents already forget…"
match §4.
