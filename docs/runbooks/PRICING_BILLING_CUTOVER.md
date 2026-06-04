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

## What is NOT done — manual / human-gated steps

1. **Stripe products & prices (TEST then LIVE).** Create, in Stripe **test mode** first:
   - Indie — $19/mo + annual (−20% ⇒ ~$182/yr)
   - Studio — $79/mo + annual (~$758/yr)
   - Scale — $299/mo + annual (~$2,870/yr)
   Prices are **immutable** — to change an amount, create a NEW price and archive
   the old; never mutate a live price.
2. **Wire tier → Stripe price IDs.** The 011 diff adds Stripe client plumbing but
   **no per-tier price-id table** yet. Add a `tier → {monthly,annual} priceID`
   map (server config / env, never `NEXT_PUBLIC_`) and resolve it where the
   checkout session is created; fill the `// 011: map tier -> price id` seam in
   `apps/web` so `/pricing` CTAs hit the right Stripe price.
3. **Add the `scale` backend binding.** 010's `config.ts` left `scale.billingTier`
   null. Once the Scale Stripe price exists, set it.
4. **Retired-tier backfill.** Run a one-shot over existing tenants applying
   `MapRetiredTier` so no live subscription points at a removed price — AFTER the
   quota decision above is made.
5. **Grandfather the free cohort** (§6): set `GrandfatherUntil = cutover + 90d` on
   current free/hosted tenants; send the $9-Indie launch-discount email (ops).
6. **Promote test → live** Stripe products only after a human verifies test-mode
   checkout end-to-end.

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
