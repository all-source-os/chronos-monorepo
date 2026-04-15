# AllSource Pricing Decision — April 2026

Closes the Phase G open item: "Pricing page review (config.ts:105-162)". This memo answers the two blocking questions — **which option** and **what the Pro tier unlocks** — with competitor and Longhand analysis to back the call.

## TL;DR

- **Option 3 (Hybrid).** Add a $29 Pro tier between Developer (free) and Team. Keep the rest.
- **Pro differentiator vs Developer:** production-grade retention and volume, plus the one feature solo operators will actually pay for — **x402 agent-monetization endpoints**. Developer is for learning; Pro is for anyone running a live agent, side-project API, or personal audit log that generates revenue.
- Rename `TEAM` button price to lead with `$79/mo billed yearly` (what Stripe charges). The `$99` monthly-billing headline is what costs us the $29-buyer today.

## Why not Option 1 or 2

**Option 1 (keep current 3-tier)** loses the exact segment that converts best on self-serve: the solo dev who already pays $20–$30 for Supabase Pro, Railway, Fly.io, or Neon. The $99 monthly price point is psychologically "team budget approval" territory; $29 is "personal card, no approval needed". There is no reason we can't capture that $29 as long as we don't cannibalize Team.

**Option 2 (full Turso-style 4-tier: Free / Pro / Team / Scale)** is more re-work than it's worth. Turso itself has moved on from that structure in 2026 — their current ladder is Free / Developer $4.99 / Scaler $24.92 / Pro $416.58 / Enterprise, which is a completely different shape. Copying a year-old competitor plan doesn't help us. Also, "Scale" at $199 would undercut Team at $79 in perceived value ("for only $120 more I get dedicated support?"). Two inflection points in the same ladder is one too many for a 3-person marketing team to message clearly.

**Option 3 (Hybrid)** is the lowest-risk expansion. The copy on Developer and Team barely changes. Pro slots in as the "obvious next step when you outgrow free".

## Competitor context (April 2026)

| Vendor | Free | Solo/Hobby | Team | High-tier | Enterprise |
|---|---|---|---|---|---|
| **AllSource (today)** | $0, 100K events | — (gap) | $99 / $79 yr | — | Custom |
| **AllSource (proposed)** | $0, 100K events | **$29 Pro** | $99 / $79 yr | — | Custom |
| Turso (2026) | 5GB, 100 DBs | $4.99 Developer | $24.92 Scaler | $416.58 Pro | Custom |
| Supabase | 500MB, 50K MAU | — | $25 Pro | $599 Team | Custom |
| Neon | 0.5GB, 100 CU-hr | compute-metered | compute-metered | compute-metered | Custom |
| EventStoreDB / Kurrent Cloud | Shared infra (free cluster) | infra-priced | infra-priced | infra-priced | Custom |

**Read of the market:** The $25–$29 self-serve slot is the single most competitive price point in dev-infra SaaS right now. Everyone has it because it's the price at which individual engineers charge to personal cards. Not having it is a conversion leak. Our closest positioning-competitor (EventStoreDB / Kurrent) doesn't have a self-serve tier at all — they are cluster-priced. That is a **genuine opening** for us to own "event sourcing for indie devs" as a category the category leader refuses to serve.

**Don't compete on price.** At $29 we are 6× more expensive than Turso Developer — that is fine. We are selling something Turso doesn't sell: immutable event history with time-travel queries. The price is a signal, not a race.

## Longhand as a precedent (tiering philosophy, not $ amount)

Longhand uses a 4-tier certification ladder (Beginner → Explorer → Operator → Certified) across three learning paths (Executive, Operator, Builder). The tier ladder progression is driven by the **user's job-to-be-done**, not by feature gates. A user moves up when their work changes, not when they want a checkbox.

Applied to AllSource, this argues for naming tiers by **who the buyer is** rather than by feature count:

- Developer — "I'm learning event sourcing"
- Pro — "I'm running one real thing in production"
- Team — "Multiple engineers share this"
- Enterprise — "Compliance and SLAs are a line item"

This keeps decision fatigue low. A user looks at the descriptions and self-identifies; they don't do feature math.

## Recommended tier definitions

### Developer — Free
100K events/month · 1 stream · 7-day retention · Community support · Basic analytics
> "Perfect for learning, prototyping, and side-project exploration."

### Pro — $29/mo (or $24/mo billed yearly)   ← NEW
1M events/month · 5 streams · **30-day retention** · **Email support (48h)** · Advanced analytics · **MCP Server (read-only)** · **x402 agent endpoints unlocked**
> "For solo operators running one production system or monetizing an AI agent."

**Why this bundle:** The three things a solo dev running real traffic actually hits on the free tier are (1) 7-day retention is too short for monthly reporting, (2) 1 stream forces everything into one namespace, and (3) no MCP means they can't plug Claude/GPT into their own data. Pro fixes all three. **x402 agent endpoints** is the feature that justifies the $29 — it's the one thing Turso/Supabase/Neon cannot offer at any price, and it's how an indie dev turns AllSource into a revenue-generating piece of their stack.

### Team — $99/mo (or $79/mo billed yearly)   ← UNCHANGED
10M events/month · Unlimited streams · 90-day retention · Priority support · Advanced analytics · MCP Server (full)
> "For teams building production systems together."

Keep as-is. Lead with `$79/mo billed yearly` in the card header; move `$99 monthly` into the fine print. This reduces the sticker shock that is currently pushing $29-buyers away.

### Enterprise — Custom   ← UNCHANGED
Unlimited events · Dedicated infrastructure · 24/7 premium support · Unlimited retention · Custom integrations · SLA & compliance
> "For high-volume, mission-critical deployments."

## What Pro specifically **does not** include (guardrails against cannibalization)

- **No unlimited streams.** If you need more than 5, you're a team.
- **No priority support SLA.** Email with 48h response, no phone, no Slack connect.
- **No multi-seat RBAC.** Single-user account. The moment a second engineer joins you, you're on Team.
- **No compliance artifacts.** SOC2 letters, DPAs, and audit exports stay on Team/Enterprise.

These four guardrails are what prevent small teams from living on Pro forever. They map cleanly to the "who is the buyer" ladder from the Longhand framing.

## Answers to the two blocking questions

1. **Which option?** Option 3 (Hybrid).
2. **Pro differentiator vs Developer?** Production retention (30 days vs 7), 10× volume (1M vs 100K), multi-stream (5 vs 1), MCP Server read access, email support, and the one non-commoditized unlock: **x402 agent-monetization endpoints**.

## Concrete edit to `apps/web/src/lib/config.ts`

Insert a new block between the existing `DEVELOPER` and `TEAM` entries (currently at lines 105-162):

```ts
{
  name: "PRO",
  tier: "pro" as const,
  href: "#",
  price: "$29",
  period: "month",
  yearlyPrice: "$24",
  features: [
    "1M events/month",
    "5 Streams",
    "30-day retention",
    "Email Support (48h)",
    "MCP Server (read-only)",
    "x402 Agent Endpoints",
  ],
  description: "For solo operators running one production system",
  buttonText: "Start Pro",
  isPopular: false,
},
```

And update the `TEAM` entry headline so the yearly price leads:

```ts
price: "$79",       // was "$99"
period: "month, billed yearly",
yearlyPrice: "$79",
```

(Keep `$99` as the month-to-month option in the fine print / toggle.)

## Downstream changes to flag

- **Billing (LemonSqueezy):** new `pro_monthly` and `pro_yearly` SKUs. See `docs/launch/LEMONSQUEEZY_SETUP.md`.
- **Quota enforcement:** Query Service needs a `pro` tier row in its plan-limits config — 1M events/month, 5 streams, 30-day retention TTL.
- **x402 gating:** the x402 agent endpoint middleware currently allows any authenticated tenant. It needs to check `tier in {pro, growth, enterprise}` before serving.
- **MCP read-only scope:** Pro should get a scoped MCP token that excludes mutation tools. This is a new permission preset in the auth service.
- **FAQ update:** Add "What's the difference between Developer and Pro?" with a one-line answer focused on production readiness, not a feature checklist.

## Open questions for Decebal

- Comfortable with **x402 as the Pro headline differentiator**, or would you rather lead with retention/volume and treat x402 as a Team+ feature? (My read: lead with x402 — it's the only thing competitors can't copy.)
- Keep the name `TEAM` or rename to `GROWTH` to match the internal `tier: "growth"` string? I'd keep `TEAM` — buyers don't care about our internal enum names.
