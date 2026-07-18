---
title: "Launch Plan — Memory for the AI-agents era"
status: CURRENT
last_updated: 2026-07-17
owner: "@ddonprogramming"
---

# AllSource Launch — "Memory for the AI-agents era"

Show HN is posted. This plan covers **everything after HN**: repo polish →
awesome-lists → Reddit → Product Hunt → newsletters → sustained X cadence →
long-tail content. All copy is grounded to the canonical facts below — every
number traces to `apps/web/src/lib/config.ts` (`siteConfig`) or the repo.

---

## 1. Positioning (the wedge)

**Category:** memory for AI agents. **Wedge:** everyone else is a vector DB with a
wrapper — memory that overwrites, can't explain itself, and is too slow to hit
every turn. AllSource is an **event store as the agent's memory**: durable,
ordered, and fast enough to query on every message.

- **Hero:** *"Your agents already forget. Stop letting them."*
- **One-liner:** durable memory for AI agents — records every event your agent emits, recalls it in ~12μs, survives restarts.
- **Four claims only AllSource can make** (vs mem0/Zep/Letta):
  1. **Provenance** — every memory traces to the source event ("why does the agent believe this?" is answerable).
  2. **Time-travel** — recall the agent's memory as-of any past moment.
  3. **Durable** — WAL + Parquet; survives restart, no silent loss.
  4. **Auditable** — full event log → deployable in regulated (fin/health/legal) settings.
- **The demo that closes it** (reuse everywhere — PH hero, X video, Reddit GIF):
  agent forgets → restart → **recall in 11.2μs** → **show the provenance chain**.
  Matches the homepage right-pane spec.

---

## 2. Canonical facts — SINGLE SOURCE (all copy must match)

| Fact | Value | Source |
|---|---|---|
| Ingest throughput | **469K events/sec** | `siteConfig.stats[0]` |
| Recall latency | **11.9μs p99** (~12μs; demo stamps **11.2μs**) | `siteConfig.stats[1]`, `.recallLatency` |
| MCP tools | **43** | `siteConfig.stats[2]` ⚠️ see P0 (README says 61) |
| Footprint | **~129MB** | `siteConfig.stats[3]` |
| Durability | WAL (CRC32 + fsync) + Parquet (Snappy) + DashMap | CLAUDE.md |
| Pricing (USD) | Self-Host **Free** · Indie **$19** · Studio **$79** (Popular) · Scale **$299** · Enterprise custom | `siteConfig.pricing` |
| x402 overage | **$0.0001/call** | `siteConfig.pricing[*].x402` |
| License (public claim) | **MIT** (core) | `siteConfig` FAQ, `apps/core/LICENSE` ⚠️ see P0 |
| Repo | github.com/all-source-os/all-source | `siteConfig.links.github` |
| Site | https://www.all-source.xyz | — |
| X handle | **@ddonprogramming** | `siteConfig.twitterHandle` |
| Contact | hello@all-source.xyz · sales@all-source.xyz | `siteConfig.links` |
| Comparison pages (LIVE — link these) | `/vs/mem0` `/vs/letta` `/vs/zep` `/vs/stoolap` `/event-sourcing-for-ai-agents` | `siteConfig.footer` |
| Prime | `/prime` — persistent agent memory via MCP, install in 30s, no embedding API | `siteConfig.header` |

**Mandatory outbound closer:** `Self-host free. $19/mo hosted. MIT. → all-source.xyz`

---

## 3. BLOCKERS — fix before any post goes out

These are consistency bugs that will get caught and cost credibility on HN/Reddit.

### P0 (blocking)
- [ ] **License story is inconsistent.** Repo root `LICENSE`=Apache-2.0, `apps/core/LICENSE`=MIT, `LICENSE-BSL`=BSL 1.1, site FAQ says "MIT". Pick ONE public sentence (recommend: *"Core is MIT; the full repo is Apache-2.0 with an enterprise BSL edition"*) and make site + README + copy agree. Copy currently says **MIT** — change everywhere if you pick differently.
- [ ] **MCP tool count: 43 vs 61.** siteConfig/site say 43; README says "61 tools across 11 categories." Reconcile to one number. Copy uses **43**.

### P1 (should fix — hurts conversion)
- [ ] **US audience sees GBP.** Config says `$19` but the LemonSqueezy store is GBP, so `/pricing` checkout renders **£18.99** (~$24). HN/PH/Reddit are USD-default. Decide: USD store/display for launch, or accept GBP and make copy say £. Copy currently says **$19**.
- [ ] **Repo has 4 stars.** Social proof floor. Warm up before Product Hunt (see checklist).
- [ ] **Verify the one-command install** on a clean machine and the exact Claude Desktop MCP snippet (from `/install`). Every channel links to it.

---

## 4. Channel sequence

Staggered — never cross-post identical copy (HN/Reddit auto-flag it).

| When | Channel | Copy file | Goal |
|---|---|---|---|
| **Now (done)** | Show HN | — | posted |
| **Day 0–2 (soft)** | Repo polish + awesome-lists PRs + GitHub Discussions post | `copy-mcp-lists-discord.md` | evergreen inbound, fix friction |
| **Day 1** | r/rust | `copy-reddit.md` | substrate/perf crowd (warm) |
| **Day 2–3** | r/LocalLLaMA + r/LLMDevs | `copy-reddit.md` | core ICP — the demo post |
| **Day 3** | Framework Discords (Cursor/Cline/LangChain/Latent Space) | `copy-mcp-lists-discord.md` | high-intent, MCP-native |
| **Day 4–5 (peak)** | **Product Hunt** (Tue–Thu) + newsletter pitches timed to it | `copy-product-hunt.md` + `copy-newsletters.md` | reach spike |
| **Launch morning** | X launch thread + pinned tweet | `copy-x-threads.md` | amplify PH |
| **Week 2–3 (long tail)** | X weekly hooks · dev.to repurpose · r/AI_Agents · LinkedIn compliance angle | `copy-x-threads.md` | SEO + durable reach |

---

## 5. Execute

Everything above becomes actions in **[`CHECKLISTS.md`](./CHECKLISTS.md)**.
Copy lives in the `copy-*.md` files. Keep this plan as the source of truth for
numbers — if a fact changes, change it here first, then propagate.
