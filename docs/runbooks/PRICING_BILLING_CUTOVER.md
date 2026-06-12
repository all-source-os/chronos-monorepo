# Pricing & Billing Cutover Runbook (011)

Cutover from the legacy 4-tier model (`free` / `pro` / `growth` / `enterprise`,
LemonSqueezy) to the 5-tier 011 model from
[`docs/proposals/PRICING_EXPOSURE_PLAN.md`](../proposals/PRICING_EXPOSURE_PLAN.md) §2:

| Public (web) | Backend tier | Price | Events/mo | x402 incl. | Retention | Streams | MCP |
|---|---|---|---|---|---|---|---|
| Self-Host | `free` | $0 | 100K* | 0 | 7d* | 1* | none (self-host) |
| Indie | `indie` | $19 | 500K | 50K | 14d | 3 | read |
| Studio | `studio` | $79 | 5M | 500K | 90d | ∞ | read+write |
| Scale | `scale` | $299 | 50M | 5M | 365d | ∞ | read+write+dedicated |
| Enterprise | `enterprise` | Custom | ∞ | ∞ | ∞ | ∞ | dedicated |

\* `free` here is the hosted/grandfather fallback; true Self-Host runs on the
user's own hardware with no hosted quota.

Source of truth: `apps/control-plane/internal/domain/entities/subscription.go`
(`TierQuotaMap`, `QuotasForTier`, `MapRetiredTier`). Public tier copy lives in
`apps/web/src/lib/config.ts` (`siteConfig.pricing`, prompt 010).

> ⚠️ **Storage model:** subscription + entitlement state is persisted as **Core
> tenant-metadata JSON**, not Postgres columns. The new entitlement fields
> (`x402_allowance`, `retention_days`, `max_streams`, `mcp_scope`,
> `grandfather_until`) are additive JSON — **no SQL migration is required.**

---

## Existing-customer quota — RESOLVED (no-downgrade floor)

`MapRetiredTier` remaps legacy tiers to 011 successors for **price + x402 scope**:

```
developer/starter → indie    pro → indie    growth/team → studio
```

A naive remap would **lower** the events quota of existing paid customers
(`pro` 1M→500K, `growth` 10M→5M). To prevent that silent downgrade,
`QuotasForTier` applies a **no-downgrade floor** (`legacyEventsFloor` in
`subscription.go`):

- A retired PAID tier keeps its **pre-011 events/queries quota** — `pro` stays
  at 1,000,000 / 100,000; `growth`/`team` stay at 10,000,000 / 1,000,000.
- The **new 011 dimensions** (x402 allowance, retention, streams, MCP scope) are
  taken from the successor tier. These didn't exist pre-011, so applying them is
  additive, not a reduction (e.g. a `pro` tenant keeps 1M events and *gains*
  indie's 50K x402 allowance + `read` MCP scope).
- New signups can never land on a retired tier, so the floor only ever protects
  existing subscriptions.

So existing customers are never downgraded on the dimension they paid for.
`GrandfatherUntil` / `IsGrandfathered()` remain available for the §6 90-day free
cohort. The §6 launch-discount email is still the recommended nudge to migrate
pro/growth customers onto a canonical tier, but it is no longer required to avoid
harm.

## x402 allowance overshoot — MITIGATED

The allowance counter (`x402_used`) is reconciled from Core events by the
scheduler. Two mitigations bound how far a tenant can exceed its included
allowance between reconciliations:

- Reconciliation interval shortened **15 min → 1 min** (`scheduler.go`).
- `CoreQuotaChecker` keeps a **per-instance in-process counter** of allowance
  calls served free since the last Core read, so the gate denies further free
  calls the moment the allowance is locally spent — it does not wait for the
  reconciler. Residual multi-instance overshoot is bounded by
  (instances × calls-since-last-tick), not a full window of traffic. Core
  remains the source of truth (rebaselines whenever its counter changes).

---

## What 011 implemented (in code, uncommitted at time of writing)

- Canonical `indie`/`studio`/`scale` tiers + retired aliases with `MapRetiredTier`.
- Full entitlement set per tier (events, queries, x402 allowance, retention,
  streams, MCP scope) in `TierQuotaMap`; `QuotasForTier` auto-resolves retired ids.
- `GrandfatherUntil` + `IsGrandfathered()` on `SubscriptionMetadata`.
- x402 allowance metering, **event-sourced through Core**: `SyncX402UsageUseCase`
  reconciles `x402_used` from `x402.allowance.consumed` + `x402.payment.settled`
  events; `x402.AllowanceChecker` / `quota_gate.go` enforce the allowance bucket
  before falling through to the pay-per-call x402 middleware.
- Stripe client/webhook plumbing alongside the existing LemonSqueezy provider
  (`PaymentProvider` is `"lemonsqueezy" | "stripe"`; `BillingPeriod` recorded).
- Tests updated to the new contract: `go test ./internal/...` green (391 pass).

> **Provider: LemonSqueezy.** This deployment bills on LemonSqueezy
> (`payment_provider` defaults to `lemonsqueezy`). The Stripe path exists in code
> but is secondary — do the LemonSqueezy steps below for the live cutover. Both
> paths now canonicalize retired tiers (`MapRetiredTier`) and resolve a variant
> per billing period.

## DONE — LemonSqueezy provisioning (2026-06-06, allsource-control-plane, TEST mode)

- [x] **Variants created** in store `282851` (All Source) — Indie/Studio/Scale ×
      monthly/annual. Published variant IDs:
      `indie:monthly 1755406`, `indie:annual 1755405`,
      `studio:monthly 1755391`, `studio:annual 1755367`,
      `scale:monthly 1755412`, `scale:annual 1755411`.
      (Each product also has a `Default` variant in `pending` status — unused cruft.)
- [x] **Fly secrets set + deployed** on `allsource-control-plane`:
      `LEMON_SQUEEZY_STORE_ID`, `LEMON_SQUEEZY_VARIANT_MAP` (`<tier>:<period>` keys),
      `LEMON_SQUEEZY_API_KEY` (test), `LEMON_SQUEEZY_WEBHOOK_SECRET`.
- [x] **Webhook registered** (id `108007`) →
      `POST https://allsource-control-plane.fly.dev/api/v1/webhooks/lemonsqueezy`,
      events: subscription_created/updated/cancelled/expired/payment_failed.
- [x] **Checkout proven** — LS API checkout for variant `1755406` returned a live
      URL accepting our `custom_data` (tenant_id/tier/billing_period).

## Still TODO — manual / human-gated steps

1. **End-to-end purchase verification** — complete a real test checkout
   (`4242 4242 4242 4242`) via `/pricing`; confirm webhook fires, signature
   verifies, tenant metadata gets the right tier + `billing_period`.
2. **Set up the `sales@all-source.xyz` inbox.** The `/pricing` Enterprise CTA (010)
   points at `mailto:sales@…`; there is currently no inbox behind it, so Enterprise
   leads are dropped. Provision the mailbox / alias before promoting the new pricing.
3. **Swap TEST → LIVE LemonSqueezy.** Prod currently runs the TEST API key + test
   variants (intentional, to verify in prod). Before real launch: create live
   variants, set live `LEMON_SQUEEZY_API_KEY` + live `LEMON_SQUEEZY_VARIANT_MAP` +
   a live webhook, and **rotate the test API key** (it was shared in chat).
4. **Add the `scale` backend binding.** 010's `config.ts` left `scale.billingTier`
   null. Bind it to the canonical `scale` tier.
5. **Retired-tier backfill.** One-shot over existing tenants applying
   `MapRetiredTier` so no live subscription points at a removed variant.
6. **Grandfather the free cohort** (§6): set `GrandfatherUntil = cutover + 90d` on
   current free/hosted tenants; send the $9-Indie launch-discount email (ops).
7. **Stripe (SECONDARY, optional).** If ever enabled: create products/prices and set
   `STRIPE_PRICE_MAP` with the same `tier:period` keys. Stripe prices are immutable
   — new price + archive old, never mutate.

## Verification (test mode)

- [ ] Checkout for indie/studio/scale (monthly + annual) completes; webhook fires;
      tenant metadata shows correct tier + full entitlement set.
- [ ] Indie tenant capped at 500K events + 50K x402; overage path bills per call.
- [ ] A grandfathered free tenant still serves requests inside the window.
- [ ] No Stripe/x402 secret in any `NEXT_PUBLIC_` value or the web bundle.

## Rollback

- Entitlement/tier changes are JSON metadata — revert by re-applying the prior
  tenant metadata (or `git revert` the 011 commit and re-sync).
- Archive (do not delete) any Stripe prices created; deletion is impossible once
  a subscription references them.
- No schema migration ⇒ no DDL to reverse.

## Scaling note — x402 allowance counter is per-instance (t-d7aabc)

The included-allowance gate (`CoreQuotaChecker`, `apps/control-plane/internal/infrastructure/x402/quota_gate.go`)
keeps an **in-process** counter of allowance calls served free between the
1-minute Core reconciler ticks. Core is the durable source of truth (every free
serve writes an `x402.allowance.consumed` event), but the fast-path tightening
counter is not shared.

- **Single instance (today):** EXACT at the allowance boundary. The Control Plane
  runs one Fly machine, so there is no overshoot.
- **Multiple instances:** the counter is per-instance, so residual overshoot at
  the boundary is bounded by `(instances - 1) × calls-served-since-the-last-tick`
  (≤ ~1 minute of the other instances' allowance traffic). Still far tighter than
  a full reconciliation window, but non-zero.

**If you scale the Control Plane out:**
1. Set `CONTROL_PLANE_MULTI_INSTANCE=true` on every instance. Boot then logs a
   loud `WARNING` so the bound is visible (it is otherwise silent).
2. To make the gate exact again, move the counter to a **shared store**. Prefer a
   **Core-side atomic counter** (a Core endpoint that atomically increments and
   returns `x402_used`) over Redis — reconciliation already flows through Core,
   so this avoids adding a second stateful dependency. Redis is the fallback if a
   Core change isn't feasible.

Until CP scales out, the accepted bound is **zero** and no action is required.
