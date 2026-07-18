---
title: "X Thread — Pricing Reversal (contrarian launch)"
status: DRAFT
handle: "@decebal"
maps_to: "PRICING_EXPOSURE_PLAN.md §5 (contrarian hook) + §6 Week 1 (pricing surgery)"
last_updated: 2026-06-04
---

# DRAFT — Pricing-Reversal Launch Thread (X / @decebal)

> **DRAFT for a human to post.** Nothing here has been posted, scheduled, or
> sent anywhere. Edit freely, then post from `@decebal` (founder POV, first
> person). Every number traces to `apps/web/src/lib/config.ts` (`siteConfig`)
> or `CLAUDE.md` — see the "Fact-check" block at the bottom.

**Format:** contrarian (§5). 8 tweets. **Mandatory closer:**
`$19/mo. Self-host free. MIT. → all-source.xyz`

---

## Thread

**1/**
Free plans are killing your indie SaaS.

I just did the opposite of what every "growth" playbook says and killed our $0 hosted plan.

Here's the pricing reversal — and why I think the free tier was quietly bleeding us. 🧵

**2/**
The old page did two indie-hostile things at once:

- It signaled "free forever is the main path."
- It buried our single best lever (pay-per-call agent payments) on the cheapest tier.

So the one thing nobody else can do was hiding behind a $0 anchor.

**3/**
Indie SaaS math is brutal.

At ~3% conversion, every free hosted user is worth pennies a month — against real infra + support cost. A perpetual free *hosted* plan isn't generous. It's a tax you pay on people who were never going to pay you.

**4/**
So I flipped the default.

New tiers:
• Self-Host — free, MIT, your hardware, forever retention
• Indie — $19/mo
• Studio — $79/mo (Popular)
• Scale — $299/mo
• Enterprise — custom

$0 isn't gone. It's just honest now: run it yourself.

**5/**
"But you removed free!"

No. We're MIT-licensed. Free already exists — you self-host it. Unlimited events on your own hardware, full MCP, forever.

Hosted pricing just reflects what it costs *me* to run it for *you*. That's the whole FAQ.

**6/**
The $19 tier is the real shift.

Below the "ask my manager" line. Below most indie tool stacks. The job of $19 isn't margin — it's flipping your brain from "$0 default" to "yeah, $19 is real money I'll happily pay for durable agent memory."

**7/**
And the lever I stopped hiding: x402 pay-per-call.

Your agent pays per read. Every paid tier ships an x402 allowance, overage is $0.0001/call. You're not renting capacity you don't use — you pay when the agent actually recalls something.

**8/**
The product behind the price:
• 469K events/sec ingest
• ~12μs recall (11.9μs p99)
• 43 MCP tools, drop into Claude Desktop
• MIT, self-host the whole thing

Your agents already forget. Stop letting them.

$19/mo. Self-host free. MIT. → all-source.xyz

---

## Fact-check (every claim → source)

| Claim in thread | Source |
|---|---|
| Self-Host free, MIT, unlimited events, forever retention | `siteConfig.pricing[self-host]` |
| Indie $19/mo | `siteConfig.pricing[indie].price` |
| Studio $79/mo, "Popular" | `siteConfig.pricing[studio].price`, `.isPopular: true` |
| Scale $299/mo | `siteConfig.pricing[scale].price` |
| Enterprise custom | `siteConfig.pricing[enterprise].price` |
| x402 overage $0.0001/call | `siteConfig.pricing[*].x402.overage` |
| 469K events/sec | `siteConfig.stats[0]` / CLAUDE.md |
| ~12μs recall / 11.9μs p99 | `siteConfig.stats[1]` (11.9μs p99); blog rounds to "12μs" |
| 43 MCP tools | `siteConfig.stats[2]` / CLAUDE.md |
| MIT licensed / self-host | `siteConfig.pricing[self-host].features`, CLAUDE.md |
| "Why no free plan?" framing | `siteConfig.faqs[0]`, plan §3 |
| Closer line | plan §5 "Personal brand" |

**Voice:** first-person founder, matches §4 relaunch line "Your agents already
forget. Stop letting them."
