# LemonSqueezy Setup Guide

## Control Plane Environment Variables

| Env Var | Description |
|---|---|
| `LEMON_SQUEEZY_API_KEY` | API key from LemonSqueezy dashboard |
| `LEMON_SQUEEZY_STORE_ID` | Store ID from LemonSqueezy dashboard |
| `LEMON_SQUEEZY_VARIANT_MAP` | JSON mapping tier names to variant IDs, e.g. `{"growth":"<variant_id>"}` |
| `LEMON_SQUEEZY_WEBHOOK_SECRET` | HMAC signing secret from LemonSqueezy webhook config |

## LemonSqueezy Dashboard Setup

### 1. Product & Variants

Create one product (e.g. "AllSource Subscription") with variants:

- **Team/Growth** — $99/mo (monthly) or $79/mo (annual). Note the variant ID.
- No "free" variant needed — free is the default tier with no subscription.

### 2. Variant Map

Set `LEMON_SQUEEZY_VARIANT_MAP` as JSON on the Control Plane:

```json
{"growth":"<variant_id_for_team_plan>"}
```

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
