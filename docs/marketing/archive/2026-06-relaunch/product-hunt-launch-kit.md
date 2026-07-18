---
title: "Product Hunt Launch Kit — next minor version"
status: DRAFT
maps_to: "PRICING_EXPOSURE_PLAN.md §5 (PH per minor version, warm-up, 2-min demo as hero) + §6 Week 3 (launch loop)"
last_updated: 2026-06-04
---

# DRAFT — Product Hunt Launch Kit

> **DRAFT for a human to post.** Nothing has been submitted to Product Hunt or
> any API. Fill the `<VERSION>` placeholder when you lock the launch version.
> Founder voice (`@decebal`). Every number traces to `siteConfig`/`CLAUDE.md`
> (fact-check at bottom).

---

## Tagline (≤60 chars)

> Durable memory for AI agents — recall in microseconds

Alternates:
- Your AI agents forget. AllSource makes them remember.
- Event-sourced memory for agents. ~12μs recall. MIT.

## Name / version line

**AllSource — v<VERSION>**

## Description (the PH "what is it" blurb)

AllSource is durable memory for AI agents. It records every event your agent
emits to a write-ahead log + Parquet (so it survives restarts), then serves
recall from in-memory projections in ~12μs — fast enough to query memory on
every turn. 469K events/sec ingest, 43 MCP tools that drop straight into Claude
Desktop, and your agent can pay per call with x402. MIT-licensed: self-host the
whole thing for free, or start hosted at $19/mo.

---

## First comment (founder story — post immediately on launch)

Hey Product Hunt 👋 I'm Decebal, building AllSource.

I kept hitting the same wall with AI agents: they forget. You stuff yesterday
into the context window, the process restarts, and it's all gone. The fixes on
offer were either a chat-log hack or a vector DB that's too slow to hit every
turn and only tells you what's *similar*, not what actually *happened*.

So I built the thing I wanted: an event store as the agent's memory. Every event
the agent emits is durable (WAL with CRC32 + fsync, then Parquet), and recall
comes back in ~12μs from in-memory projections — invisible, so you can query on
every single message. 469K events/sec, 43 MCP tools so Claude can read and write
memory directly, and x402 so the agent pays per call instead of you renting
capacity you don't use.

It's MIT-licensed. The honest free plan is: self-host it, unlimited events on
your own hardware, forever. Hosted starts at $19/mo if you'd rather I run it.

Would genuinely love your hardest questions — especially on durability and the
microsecond recall numbers. AMA below. 🦫

---

## "What's new in v<VERSION>" — bullets (tie to the relaunch)

- 🧠 **Durable agent memory, not a chat log** — every event hits WAL (CRC32 + fsync) + Parquet; survives restarts.
- ⚡ **~12μs recall (11.9μs p99)** — query memory on every turn without the user feeling it.
- 🚀 **469K events/sec ingest** — the memory layer is never your bottleneck.
- 🔌 **43 MCP tools** — drop AllSource into Claude Desktop; agent reads and writes memory directly.
- 💸 **x402 pay-per-call** — your agent pays per read; overage $0.0001/call. No renting idle capacity.
- 🪶 **~129MB footprint** — the whole stack, not a fleet of sidecars.
- 🆓 **Self-host free, MIT** — unlimited events on your hardware, forever retention.
- 💵 **New 5-tier pricing** — Self-Host free / Indie $19 / Studio $79 (Popular) / Scale $299 / Enterprise.
- 🌐 **Public /pricing** — shareable, honest "why no free plan?" FAQ baked in.

(Trim to 5–8 on launch; keep the durability + 12μs + MCP + x402 + pricing bullets.)

---

## Maker's reply template (for comments)

> Thanks [name]! [One-sentence direct answer.]
>
> [The relevant grounded number — e.g. "Recall is ~12μs (11.9μs p99) because the
> read path is an O(1) DashMap lookup, not a query — events are rebuilt from the
> durable WAL on boot, so fast never means lossy."]
>
> If you want to poke at it: it's MIT, self-host is free —
> github.com/all-source-os/all-source. Happy to go deeper here. 🦫

**Canned answers for predictable questions:**

- *"Isn't it just in-memory / lost on restart?"* → No. Event data is durable: WAL (CRC32 checksums, configurable fsync) + Parquet (Snappy). Only the in-memory projections are rebuilt from the log on boot — the source of truth is on disk.
- *"Why not Postgres / a vector DB?"* → Postgres is for operational metadata, not events. A vector DB tells you what's *similar*, not what *happened* in order. AllSource is the event log; vectors are a projection on top when you want fuzzy search.
- *"How is recall that fast?"* → O(1) concurrent hash-map (DashMap) lookup on in-memory projections — no SQL parse, no network, no disk on the hot path. ~12μs.
- *"Free tier?"* → Self-host is genuinely free and unlimited (MIT). Hosted starts at $19/mo.

---

## Warm-up checklist (§5 — run the week before)

- [ ] **Lock the launch date** (Tue–Thu, 12:01am PT start). Add to calendar.
- [ ] **Line up 10–20 hunters** in the network — DM a week ahead, don't ask for upvotes, ask them to "check it out at launch + leave an honest question."
- [ ] **Record the 2-minute demo video as the hero asset** — same content as the homepage right pane: JSON events streaming in (top), Claude asking "what did the user do yesterday at 3pm?" and the answer rendering out of those events with "returned in 11.2μs ✓" stamped on it.
- [ ] **Pin the demo + new pricing** on `@decebal` and `@allsourcedev` (see `pinned-tweets.md`) the morning of.
- [ ] **Stage the build-in-public thread** (`x-pricing-reversal-thread.md` or a Week hook) to fire on launch morning, linking the PH page.
- [ ] **Prep gallery assets**: hero demo GIF, the 5-tier pricing screenshot, one architecture/latency diagram.
- [ ] **Block 6+ hours launch day** to reply to every comment within minutes (PH ranking rewards maker engagement).
- [ ] **Draft thank-you DM** for hunters to send post-launch.

---

## Fact-check (every claim → source)

| Claim | Source |
|---|---|
| ~12μs recall / 11.9μs p99 / 11.2μs stamp | `siteConfig.stats[1]`, `siteConfig.recallLatency` |
| 469K events/sec | `siteConfig.stats[0]` / CLAUDE.md |
| 43 MCP tools | `siteConfig.stats[2]` / CLAUDE.md |
| ~129MB footprint | `siteConfig.stats[3]` / CLAUDE.md |
| WAL (CRC32 + fsync) + Parquet (Snappy) durability | CLAUDE.md "Architecture Facts" |
| DashMap O(1) projections rebuilt from WAL | CLAUDE.md; blog `12-microsecond-agent-memory.mdx` |
| x402 pay-per-call, $0.0001/call overage | `siteConfig.pricing[*].x402`, plan §2 |
| 5 tiers + prices | `siteConfig.pricing` |
| MIT / self-host free | `siteConfig.pricing[self-host]`, CLAUDE.md |
| Postgres = metadata only / no events | CLAUDE.md "Architecture Facts" |
| github.com/all-source-os/all-source | `siteConfig.links.github` |

**2-min demo content** matches the §4 homepage right-pane spec exactly.
