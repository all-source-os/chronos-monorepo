# AllSource Pricing & Exposure Plan

Applying Marc Lou's indie-SaaS pricing playbook to a developer-infrastructure product.

## TL;DR — the bet

The current pricing page and homepage do two indie-hostile things at once: they signal "free forever is the main path" *and* they hide your strongest pricing lever — **x402 micropayments**. Marc Lou's playbook isn't a literal template (he sells $199 boilerplates, not durable infrastructure), but four of his principles map cleanly:

1. Kill the $0 plan as the headline option.
2. Make the second tier the "obvious" one with popcorn anchoring (a visible high tier above it).
3. Where costs scale with usage, charge **credits**, not subscriptions.
4. The first 100% of the homepage viewport must show the product *working*.

## 1. What's wrong today

### Pricing page (4 tiers: $0 / $29 / $79 / Custom)

- **Broken toggle copy.** "$79 / month, billed yearly" with "billed monthly" stacked underneath is contradictory. First-time viewers can't tell whether $79 is the monthly or the yearly-discounted-monthly price.
- **Free tier too generous.** 100K events/mo, MCP access, 7-day retention. Marc Lou math: at 3% conversion, each free user is worth ~$0.87/mo to a $29 tier. That's a lot of support and infra cost per cent.
- **USP lives on the cheap tier.** Pro at $29 is the entry plan, but it's also where x402 Agent Endpoints live. Your only truly differentiated feature is on your low-anchor plan instead of being the lead pitch.
- **No popcorn.** Growth at $79 is labelled "Popular" but has no visible anchor above it — Enterprise is "Custom" with no number. There's no kernel above to make the middle tier the obvious choice.
- **Vague MCP labels.** "MCP Server (read-only)" on Pro and "MCP Server Access" on Growth — same words, undefined difference. A buyer can't price the upgrade.

### Homepage

- **Abstract headline.** "Give your AI agents perfect memory" — Marc Lou's rewriting-tweets advice: replace abstract nouns ("memory", "momentum") with concrete ones ("remember every event", "recall every decision").
- **Zero-flash hero.** The stat strip animates from "0K events/sec, 0μs". If the animation hasn't fired (or a user lands on a slow connection), the first thing they see is zeros next to your best numbers.
- **No product-in-action above the fold.** No split-screen demo. The "wow" — sub-microsecond recall on durable events — is invisible until the user scrolls.
- **Version-number banner buries the news.** "v0.19 — x402 Agent Payments + Pro Tier" leads with the version, not the user-visible promise.

### Exposure

- Blog ships heavily (engineering wins), but post hooks aren't tuned for virality (numbered headlines, contrarian takes, screenshot-as-thumbnail).
- No visible Product Hunt cadence per release. v0.17 → v0.21 = at least 4 PH-launch opportunities you've earned.
- No "free tool" funnel — Marc Lou's other distribution pattern (e.g., MakeLanding, IndiePage as standalone hooks).
- The pricing page is the same as a section on `/`. There's no public `/pricing` URL to share on Twitter or paste in launch posts (confirm and fix).

## 2. New pricing model

Three public tiers in the middle, a free Self-Host on the left, a custom Enterprise on the right. Drop the $0 hosted plan as the default. Move free usage into a **14-day trial** plus a clearly-CTA'd **self-host** option (which is genuinely free — you're MIT-licensed, that's a feature, not a giveaway).

| | Self-Host | Indie | Studio | Scale | Enterprise |
|---|---|---|---|---|---|
| **Headline** | Free, your infra | $19/mo | $79/mo | $299/mo | Custom |
| **For** | Tinkerers, OSS, on-prem | Solo builders, one agent | Teams running 1–5 agents | Companies with 50+ agents | Regulated / SLA workloads |
| **Events/mo** | Unlimited (your hardware) | 500K + 50K x402 credits | 5M + 500K x402 credits | 50M + 5M x402 credits | Negotiated |
| **Retention** | Forever | 14 days | 90 days | 365 days | Unlimited |
| **MCP** | Full (self-host) | Hosted read | Hosted read + write | Hosted read + write + dedicated | Dedicated cluster |
| **Streams** | Unlimited | 3 | Unlimited | Unlimited | Unlimited |
| **Support** | GitHub community | Email 48h | Email 24h + Discord | Priority + Slack | 24/7 + dedicated SE |
| **Anchoring** | "Run it yourself" | (low) | **Popular** | (high anchor) | (custom) |

Key moves and why:

- **$19, not $29, for the entry tier.** Below the "ask my manager" threshold, below most indie tool stacks, still 9-ending. The job of this tier is to flip the user's mental default from "$0" to "yes, $19 is real."
- **$299 Scale tier above $79 creates the popcorn.** $79 now feels like the obvious middle, not the ceiling. Today there's no kernel above Growth; the popcorn never pops.
- **Trial, not free tier.** Replace Developer with "14-day Indie trial — 50K events, no card." Marc Lou's 3% conversion math kills the case for an infinite free hosted plan. (This is B2B SaaS — subscriptions still make sense per his own B2B carve-out — but the perpetual free *hosted* plan does not.)
- **Self-Host as a real tier, not an asterisk.** You're MIT-licensed. Owning that on the pricing page differentiates you from Snowflake / Datadog / Confluent (who can't say this) and from mem0 / Letta / Zep (who can but don't lead with it).
- **x402 credits surfaced on every paid tier.** Each tier ships an allowance; overage = pay-as-you-go via x402. This is exactly the credits-based "good-better-best" pricing Marc Lou recommends for usage-cost products — and it's already your unique tech.
- **Explicit MCP verbs.** Replace "MCP Server (read-only)" / "MCP Server Access" with `read`, `read + write`, `read + write + dedicated`. A buyer can price the upgrade in one glance.

## 3. Pricing page redesign

Above the fold, in order:

1. One sentence: "Pay for the events your agents write. Self-host for free if you'd rather."
2. Toggle: Monthly | Yearly (-20%) — single label, no nested "billed monthly" line.
3. Four cards in one row: Self-Host (outlined), Indie, **Studio** (filled-highlight + "Popular"), Scale.
4. Enterprise as a quieter full-width strip below the cards. Dark background, "Talk to us" CTA.
5. Under each card: the x402 credit allowance as a *single line* — "Includes 500K x402 calls. $0.0001/call after."

Below the fold:

- The comparison matrix above, with Self-Host as a column.
- A short **"Why no free plan?"** FAQ. Be direct: "We're MIT-licensed. Free already exists — run it yourself. Hosted pricing reflects what it costs us to run it for you." This pre-empts the predictable Hacker News objection in one paragraph.

## 4. Homepage first-viewport rewrite

Marc Lou's rule: in the first 100% viewport, show what it *does*, not what it *is*. Concretely:

**Left half (above the fold)**
- Headline: **"Your agents already forget. Stop letting them."**
- Subhead: "AllSource records every event your agent emits, with 12μs recall on every restart. Pay only when the agent reads."
- Buttons: `Start Indie — $19` (primary), `Self-host on GitHub` (ghost).

**Right half (above the fold)**

A live or pseudo-live two-pane demo:
- Top pane: a stream of JSON events scrolling in (`user.signed_up`, `cart.checkout`, `agent.decided`).
- Bottom pane: an LLM chat box where "Claude" asks "What did the user do yesterday at 3pm?" and a response renders out of those events in real time, with "returned in 11.2μs ✓" stamped on the message.

This replaces the current stat-strip hero. Stats (469K events/sec, 12μs, 43 MCP tools, 129MB) move into the demo's chrome and a strip below the fold — never as the first thing the visitor sees.

Other small fixes:

- Version banner "v0.19 — x402 Agent Payments + Pro Tier" → **"Your AI agent can now pay per call (x402, live in v0.19) →"**. Lead with the user-visible promise.
- "Perfect memory" → "remembers every event."
- "Temporal Intelligence" → "Time-travel your data."

## 5. Exposure plan

Marc Lou's loop: build in public on X → launch on Product Hunt → blog feeds the X loop → email list compounds. You already have most pieces. You're missing **cadence** and **hooks**.

### Weekly (next 90 days)

- 1 X thread from `@decebal` or `@allsourcedev`. Use one of three hook formats: numbered ("3 reasons your agent forgets"), benchmark ("12μs recall. Here's the code."), or contrarian ("Free plans are killing your indie SaaS — here's our pricing reversal").
- 1 blog post repurposed into: 1 X thread + 1 LinkedIn long-form + 1 60-second YouTube/X Short (screen recording of the feature).
- 1 short-form video: `cn serve` terminal, MCP query latency demo, or a live x402 payment.

### Monthly

- 1 Product Hunt launch per minor version. v0.17 → v0.21 is 4–5 PH launches you didn't take. Each launch: warm the audience a week ahead, line up 10–20 hunters in the network, ship with a 2-minute demo video as the hero asset.
- 1 free tool every 6–8 weeks. Candidates:
  - **`memcheck`** — paste your agent's stack (mem0 / Letta / Zep / none), get a benchmark vs AllSource on recall + cost.
  - **x402 playground** — send a paid agent request without signing up; AllSource pays the gas.
  - **Event schema linter** — paste an event JSON, get a schema-evolution risk score.

### Personal brand

- "Technical Leaders" / `@decebal` is the natural front. Marc Lou earns ~90% of ShipFast distribution from `@marclou`, not `@shipfast`. Position AllSource posts as one founder's POV ("I'm building durable memory for AI agents because…"), not as a corporate handle.
- Pin a tweet on `@decebal`: 60s demo + "$19/mo. Self-host free. MIT. → all-source.xyz".

### SEO debt (background, slower payoff)

- ~30 strong blog posts but no internal pillar pages. Build a `/event-sourcing-for-ai-agents` pillar that links out to the existing posts.
- Add `/vs/mem0`, `/vs/letta`, `/vs/zep` comparison pages — you already have the matrix on the homepage; turn each row into its own page targeting "AllSource vs X" search intent.

## 6. 30-day execution checklist

**Week 1 — pricing surgery**
- [ ] Ship the new 5-tier layout on `/pricing` (Self-Host, Indie $19, Studio $79, Scale $299, Enterprise).
- [ ] Add public `/pricing` URL if not already public; link from homepage nav.
- [ ] Fix the "billed yearly / billed monthly" toggle copy.
- [ ] Rewrite MCP feature labels with read/write/dedicated verbs.
- [ ] Add the "Why no free plan?" FAQ.
- [ ] Grandfather existing free users for 90 days; email them with a $9 launch discount on Indie for 6 months.

**Week 2 — homepage first-viewport**
- [ ] Replace stat-strip hero with the split-screen events-streaming + agent-recalling demo.
- [ ] Rewrite headline + subhead per Section 4.
- [ ] Rewrite the version banner to lead with x402.
- [ ] Preload stat numbers so no "0K" flash on first paint.

**Week 3 — launch loop**
- [ ] Schedule the next Product Hunt launch. Lock the date, line up hunters.
- [ ] Record the 2-minute demo video (same content as the homepage right pane).
- [ ] Pin the demo + new pricing on `@decebal` and `@allsourcedev`.
- [ ] Ship `/vs/mem0`.

**Week 4 — measure & iterate**
- [ ] Set conversion KPIs: trial → paid, paid distribution across Indie / Studio / Scale.
- [ ] Compare new MRR cohort vs free-plan cohort 30-day economics.
- [ ] Pick the next free tool (`memcheck` recommended); block calendar.

## 7. Where I'd push back on Marc Lou

A few of his pieces of advice don't transfer cleanly — flagging so you don't over-apply:

- **"Ditch your subscription"** is right for $5 consumer SaaS. AllSource is B2B infra — subscriptions are fine, and Marc Lou himself carves out B2B as an exception. Don't go lifetime-only.
- **"Free plan is dead weight"** is right *as a headline plan*, but a generous **trial** plus a **self-host** path captures 90% of the developer-marketing benefit a free tier gave you. Don't go zero-free; just go zero-free-as-default.
- **"Increase prices over time"** is solid, but for infra you should *publish* a price ladder ($19 → $29 → $49 over 12 months) so existing customers feel grandfathered, not betrayed.

## Sources

- Marc Lou, [Ditch your free plan](https://newsletter.marclou.com/p/ditch-your-free-plan)
- Marc Lou, [Ditch your subscription](https://newsletter.marclou.com/p/ditch-your-subscription)
- Marc Lou, [Rewriting 3 tweets](https://newsletter.marclou.com/p/rewriting-3-tweets) (popcorn pricing, replace abstract nouns)
- Marc Lou, [How to get customers with free tool marketing](https://newsletter.marclou.com/p/marketing-for-product-obsessed-developers)
- AllSource pricing & homepage as of 2026-06-03 ([all-source.xyz](https://www.all-source.xyz/), [/dashboard/billing](https://www.all-source.xyz/dashboard/billing))
