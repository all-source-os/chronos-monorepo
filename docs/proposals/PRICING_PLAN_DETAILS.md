# Pricing Plan Details

Source of record for per-tier entitlements. **Two layers must always agree:**
- **Displayed** — `apps/web/src/lib/config.ts` (`siteConfig.pricing` features) + live prices from LemonSqueezy via `/api/v1/billing/catalog`.
- **Enforced** — `apps/control-plane/internal/domain/entities/subscription.go` (`TierQuotaMap`: events/queries quota, x402 allowance, retention, max streams, MCP scope).

Change both together or they drift. Prices themselves are sourced live from LemonSqueezy (see `docs/runbooks/PRICING_BILLING_CUTOVER.md`).

## Current tiers (as of 2026-06-07)

| | Self-Host | Indie $19 | Studio $79 | Scale $299 | Enterprise |
|---|---|---|---|---|---|
| **Events/mo** | unlimited (own hw) | 500K | 5M | 50M | negotiated (∞) |
| **Queries/mo** | — | 50K | 500K | 5M | ∞ |
| **x402 included** | — | 50K | 500K | 5M | ∞ |
| **x402 overage** | — | $0.0001/call | $0.0001/call | $0.0001/call | negotiated |
| **Retention** | forever | 14 days | 90 days | 365 days | unlimited |
| **Streams** | ∞ | 3 | ∞ | ∞ | ∞ |
| **MCP** | full (self-host) | read | read + write | read + write + dedicated | dedicated cluster |
| **Support** | GitHub community | email 48h | email 24h + Discord | priority + Slack | 24/7 + dedicated SE |

Backend also has a legacy `free` tier (100K events, 7-day retention, 1 stream, no hosted MCP). Per product direction **no hosted tenant should sit on `free`** — see the early-adopter migration below.

## Open questions (to revisit)

Retention is the flagged one. Decisions still pending:
- **Indie retention** — 14 days may be too short for the value; candidate bump to 30 days.
- **Studio 90d / Scale 365d** — keep, or widen the gap to sharpen the upgrade reason.
- **Headline differentiator** — retention vs events/mo: which leads the pitch?
- **x402** — keep flat `$0.0001/call` overage across tiers, or scale it per tier?
- **Streams** — is Indie's 3-stream cap right, or unlimited everywhere?

When these land, update `siteConfig.pricing` **and** `TierQuotaMap` in the same change, and (if a paid tier's numbers change) confirm the LemonSqueezy variant + this doc.

## Early-adopter migration (decided 2026-06-07)

Existing hosted tenants currently on `free`/Self-Host are migrated off it:
- **All hosted-free tenants → Studio**, with an **early-adopter voucher = 1 year free** (`SubscriptionMetadata.GrandfatherUntil = now + 365d`). After the year they convert to paid Studio.
- **Owner account → Enterprise** (comped, no expiry).
- Run mechanism: env-guarded one-shot on control-plane boot (`RUN_EARLY_ADOPTER_MIGRATION`), idempotent (skips tenants already on a paid tier).
