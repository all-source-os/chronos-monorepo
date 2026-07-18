---
title: "Pinned-tweet copy — @decebal and @allsourcedev"
status: DRAFT
maps_to: "PRICING_EXPOSURE_PLAN.md §5 (pin demo + pricing) + §6 Week 3"
last_updated: 2026-06-04
---

# DRAFT — Pinned Tweets

> **DRAFT for a human to post.** Nothing pinned, posted, or scheduled. Attach the
> 60s demo video (the §4 homepage right-pane recording: events streaming → Claude
> recalls → "returned in 11.2μs ✓"). Closer line is mandatory and identical on
> both. Numbers trace to `siteConfig` (fact-check at bottom).

---

## `@decebal` (founder POV — personal handle, ~90% of distribution per §5)

> I'm building durable memory for AI agents.
>
> Your agent emits events. AllSource records every one (durably — survives
> restarts) and recalls them in ~12μs. Fast enough to query memory on every
> turn. Watch it remember 👇
>
> [attach 60s demo: events stream in → Claude asks "what did the user do
> yesterday at 3pm?" → answer renders out of the events, "returned in 11.2μs ✓"]
>
> Your agents already forget. Stop letting them.
>
> $19/mo. Self-host free. MIT. → all-source.xyz

---

## `@allsourcedev` (product handle)

> AllSource is durable memory for AI agents.
>
> 469K events/sec ingest. ~12μs recall (11.9μs p99). 43 MCP tools straight into
> Claude Desktop. Your agent pays per call with x402. MIT — self-host the whole
> thing.
>
> 60-second demo 👇
>
> [attach 60s demo: same recording]
>
> Your agents already forget. Stop letting them.
>
> $19/mo. Self-host free. MIT. → all-source.xyz

---

## Notes for the poster

- Pin **after** the demo video is uploaded natively (X autoplays native video; links don't).
- Re-pin the morning of any Product Hunt launch (see `product-hunt-launch-kit.md`).
- The `@decebal` version leads with "I'm building…" (first person) per §5; `@allsourcedev` leads with the product. Both end identically.

## Fact-check (every claim → source)

| Claim | Source |
|---|---|
| ~12μs recall / 11.9μs p99 / 11.2μs stamp | `siteConfig.stats[1]`, `siteConfig.recallLatency` |
| 469K events/sec | `siteConfig.stats[0]` / CLAUDE.md |
| 43 MCP tools | `siteConfig.stats[2]` / CLAUDE.md |
| x402 pay-per-call | `siteConfig.pricing[*].x402`, plan §2 |
| $19/mo Indie / Self-Host free / MIT | `siteConfig.pricing` |
| all-source.xyz | `siteConfig.url` |
| Closer + "Your agents already forget…" | plan §5 / §4 |
