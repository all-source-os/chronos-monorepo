---
title: "Marketing Content Drafts — launch-loop index"
status: DRAFT
last_updated: 2026-06-04
source_plan: "docs/proposals/PRICING_EXPOSURE_PLAN.md §4, §5, §6"
---

# Marketing Content Drafts (DRAFT)

> **All files in this folder are DRAFTS for a human to post.** Nothing has been
> posted, scheduled, connected to a social API, or sent anywhere. The founder
> posts; this folder is the blank-page killer. Edit lightly, then ship on your
> own timing.

These drafts execute the 90-day exposure cadence from
`docs/proposals/PRICING_EXPOSURE_PLAN.md` §5, riding the pricing/homepage
relaunch (§4). Voice throughout is founder POV from **`@decebal`** (first
person) — per §5, ~90% of distribution comes from the personal handle.

## Ground rules (apply to every file)

- **Every number traces to source.** All stats come from
  `apps/web/src/lib/config.ts` (`siteConfig`), `CLAUDE.md`, or the plan. Each
  file carries a "Fact-check" table. No invented numbers.
- **On-message with the relaunch (§4):** "Your agents already forget. Stop
  letting them." Mandatory closer on outbound posts:
  `$19/mo. Self-host free. MIT. → all-source.xyz`.
- **Pricing is consistent fleet-wide:** Self-Host free (MIT) / Indie $19 /
  Studio $79 "Popular" / Scale $299 / Enterprise custom.

## Headline numbers (single source: `siteConfig.stats`)

| Number | Meaning |
|---|---|
| 469K events/sec | ingest throughput |
| 11.9μs (p99) / ~12μs / 11.2μs stamp | recall latency |
| 43 | MCP tools |
| 129MB | footprint |
| $0.0001/call | x402 overage rate |

## Index → §6 checklist week

| File | Purpose | Maps to §6 |
|---|---|---|
| `x-pricing-reversal-thread.md` | Contrarian launch thread (`@decebal`): kill the $0 plan, 8 tweets | **Week 1** — pricing surgery (announces the relaunch) |
| `x-hook-templates.md` | 3 filled weekly threads — numbered / benchmark / contrarian | **Weeks 1–3** — weekly X cadence (one per week) |
| `pinned-tweets.md` | Pinned copy for `@decebal` + `@allsourcedev` (60s demo + closer) | **Week 3** — pin demo + new pricing |
| `product-hunt-launch-kit.md` | Tagline, description, founder first comment, what's-new bullets, maker replies, warm-up checklist | **Week 3** — schedule PH launch, record 2-min demo |
| `repurpose-12-microsecond-agent-memory.md` | One blog post → X thread + LinkedIn long-form + 60s video script | **Weeks 1–3** — weekly blog-repurpose loop |
| `free-tool-memcheck-spec.md` | SPEC for the `memcheck` free-tool funnel (future prompt) | **Week 4** — pick next free tool (memcheck recommended) |

## Suggested 4-week run order

- **Week 1:** post `x-pricing-reversal-thread.md` (relaunch announcement) + Week A numbered hook from `x-hook-templates.md`.
- **Week 2:** post Week B benchmark hook + the repurpose-kit X thread/LinkedIn; record the 60s video.
- **Week 3:** Product Hunt launch (`product-hunt-launch-kit.md`), pin tweets (`pinned-tweets.md`), Week C contrarian hook.
- **Week 4:** measure; greenlight `memcheck` from the spec as the next build prompt.

## Repurposed source post

`apps/web/content/12-microsecond-agent-memory.mdx` — chosen because it maps
directly onto the benchmark hook (12μs), the §4 homepage demo (events → recall
stamp), and has a ready `cn`/`allsource-prime` + curl latency demo for the 60s
short.

---

## Note on prior files in this folder

The pre-existing `EARLY_ALPHA_ANNOUNCEMENT.md`, `PRICING_DECISION_2026-04.md`,
`PRODUCTHUNT_LISTING.md`, and `TWITTER_LAUNCH_THREAD.md` predate this relaunch
and the 5-tier pricing. They are left untouched; the files indexed above are the
current, on-message drafts for the §5/§6 launch loop.
