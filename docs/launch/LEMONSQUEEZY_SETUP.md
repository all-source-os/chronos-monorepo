# LemonSqueezy Setup Guide

## Control Plane Environment Variables

| Env Var | Description |
|---|---|
| `LEMON_SQUEEZY_API_KEY` | API key from LemonSqueezy dashboard |
| `LEMON_SQUEEZY_STORE_ID` | Store ID from LemonSqueezy dashboard |
| `LEMON_SQUEEZY_VARIANT_MAP` | JSON mapping tier names to variant IDs, e.g. `{"pro":"<variant_id>","growth":"<variant_id>"}` |
| `LEMON_SQUEEZY_WEBHOOK_SECRET` | HMAC signing secret from LemonSqueezy webhook config |

## LemonSqueezy Dashboard Setup

### 1. Product & Variants

Create one product (e.g. "AllSource Subscription") with the following variants. **Numbers come from `docs/marketing/PRICING_DECISION_2026-04.md` (April 2026 pricing decision).** All three paid-tier variants must exist before the website's pricing page will convert — the Control Plane reads `LEMON_SQUEEZY_VARIANT_MAP` and will fail checkout for any tier whose variant isn't mapped.

| Variant | Monthly | Yearly | Events quota | Notes |
|---|---|---|---|---|
| **Pro** | $29/mo | $24/mo billed yearly | 1,000,000/mo | Unlocks x402 agent endpoints. Headline differentiator vs Developer. |
| **Growth** | $99/mo | $79/mo billed yearly | 10,000,000/mo | Renamed from TEAM on 2026-04-16. Existing variant IDs can be reused — the tier name changed, not the SKU. |
| **Enterprise** | Custom | Custom | Unlimited | Sales-led, no self-serve checkout; no LemonSqueezy variant required. |

- No "free" variant needed — free is the default tier with no subscription.
- For each new variant in LemonSqueezy, note the numeric variant ID — you'll paste them into `LEMON_SQUEEZY_VARIANT_MAP` below.
- **Action required (launch blocker):** create `pro_monthly` and `pro_yearly` variants in the LemonSqueezy dashboard. These do not exist yet as of 2026-04-16.

### 2. Variant Map

Set `LEMON_SQUEEZY_VARIANT_MAP` as JSON on the Control Plane. Both `pro` and `growth` keys must be present for the pricing page to work end-to-end:

```json
{
  "pro": "<variant_id_for_pro_plan>",
  "growth": "<variant_id_for_growth_plan>"
}
```

Deploy with `fly secrets set LEMON_SQUEEZY_VARIANT_MAP='{"pro":"...","growth":"..."}' -a allsource-control-plane`.

### 3. Webhook

Configure a webhook in LemonSqueezy pointing to:

```
https://<control-plane-domain>/api/v1/webhooks/lemonsqueezy
```

Subscribe to these events:
- `subscription_created`
- `subscription_updated`
- `subscription_cancelled`
- `subscription_expired`
- `subscription_payment_failed`

Copy the HMAC signing secret into `LEMON_SQUEEZY_WEBHOOK_SECRET`.

## Data Flow

```
User clicks Upgrade → POST /api/v1/billing/checkout (tier, period)
  → Control Plane looks up variant ID from LEMON_SQUEEZY_VARIANT_MAP
  → Creates LemonSqueezy checkout session
  → Returns checkout_url → frontend redirects

User completes payment → LemonSqueezy fires webhook
  → POST /api/v1/webhooks/lemonsqueezy
  → Control Plane verifies HMAC signature
  → Extracts tenant_id from meta.custom_data (seeded during checkout)
  → Writes billing event to Core (event-sourced)

Query Service reads billing state from Core
  → GET /api/billing/status → derives current tier from latest billing events
```

See `docs/current/BILLING_ARCHITECTURE.md` for full C4 diagrams and sequence flows.

## Webhook Event Handling

| Event | Action |
|---|---|
| `subscription_created` | Writes `billing.subscription.created` to Core, sets tier |
| `subscription_updated` | Writes `billing.subscription.updated` to Core |
| `subscription_cancelled` | Writes event + suspends tenant |
| `subscription_expired` | Writes event + suspends tenant |
| `subscription_payment_failed` | Writes `billing.payment.failed`, sets status to `past_due` |

## Important

- **Only the Control Plane** talks to LemonSqueezy. No other service needs LemonSqueezy secrets.
- **Query Service** reads billing state from Core events — no direct LemonSqueezy integration.
- Billing events are durable in Core (WAL + Parquet) and provide a full audit trail.
