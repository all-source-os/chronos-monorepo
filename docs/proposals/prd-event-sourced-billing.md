# PRD: Event-Sourced Billing via Core

## Overview

Migrate billing state management from the Control Plane's in-memory metadata to event-sourced billing events stored in AllSource Core. The Control Plane continues to receive LemonSqueezy webhooks but now writes billing events to Core instead of updating local state. The Query Service reads billing state from Core projections to serve the frontend.

This makes billing state **durable** (survives restarts), **auditable** (full event history), and **decoupled** (QS reads from Core, never talks to LemonSqueezy).

## Goals

- Billing state survives Control Plane restarts without re-fetching from LemonSqueezy
- Full audit trail of every subscription change as immutable events in Core
- Query Service derives billing state from Core — no LemonSqueezy secrets needed
- Configurable tier resolution via `LEMON_SQUEEZY_VARIANT_MAP` env var (replaces hardcoded switch)
- Frontend reads billing status from QS (backed by Core), not CP

## Quality Gates

### Epic-Level (run once on epic completion)
- `quality-go` CI gate passes (Control Plane)
- `quality-elixir-full` CI gate passes (Query Service)
- `quality-rust` CI gate passes (Core — if billing event schema added)

### Story-Level (checked per story)
- **Backend stories:** Verify endpoint returns expected response via curl
- **Integration stories:** Verify end-to-end flow by creating a billing event and reading it back

## User Stories

### US-001: CP writes billing events to Core on webhook receipt [Backend]
**Description:** As the system, I want the Control Plane to write billing events to Core when it receives a LemonSqueezy webhook, so that billing state is durable and auditable.

**Acceptance Criteria:**
- [ ] Modify `ProcessLemonSqueezyWebhookUseCase.Execute()` to POST a billing event to Core after processing
- [ ] Event format: `{event_type: "billing.subscription.{action}", entity_id: tenant_id, payload: {subscription_id, customer_id, tier, status, payment_provider, variant_id, variant_name, billing_period, amount_cents, currency}}`
- [ ] Events written for: subscription_created, subscription_updated, subscription_cancelled, subscription_expired, payment_failed
- [ ] Core URL read from `CORE_SERVICE_URL` env var (already set)
- [ ] If Core write fails, log error but still process webhook (don't block subscription updates)
- [ ] Verify by curling the webhook endpoint with a test payload and checking Core for the event

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Replace hardcoded tier switch with env-var-driven variant map [Backend]
**Description:** As an operator, I want tier resolution to use `LEMON_SQUEEZY_VARIANT_MAP` so I can add new tiers without code changes.

**Acceptance Criteria:**
- [ ] Read `LEMON_SQUEEZY_VARIANT_MAP` env var as JSON (e.g. `{"growth":"12345","enterprise":"67890"}`)
- [ ] Replace `resolveTierFromVariantName()` to look up variant name OR variant ID in the map
- [ ] Fall back to "free" if variant not found in map
- [ ] If env var is not set, fall back to existing hardcoded mapping (backwards compatible)
- [ ] Add test for variant map parsing and tier resolution

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: QS billing status endpoint reads from Core [Backend]
**Description:** As the frontend, I want to call `GET /api/billing/status` on the Query Service and get the current billing state derived from Core events.

**Acceptance Criteria:**
- [ ] Add `GET /api/billing/status` endpoint to QS (or modify existing billing controller)
- [ ] Query Core for latest `billing.*` events where `entity_id = tenant_id`
- [ ] Derive current state: tier, status, billing_period, subscription_id, quotas, usage
- [ ] Return JSON: `{tier, status, billing_period, events_quota, queries_quota, events_used, queries_used, subscription_ends_at, trial_ends_at}`
- [ ] If no billing events exist for tenant, return default free tier state
- [ ] Remove redirect to CP for billing status (keep redirects for checkout/portal which need LemonSqueezy API)

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Frontend reads billing from QS instead of CP [Integration]
**Description:** As a user, I want the billing page to load from event-sourced data so it's always consistent with the latest subscription state.

**Acceptance Criteria:**
- [ ] Update `apiClient` to call QS `/api/billing/status` for billing state (instead of deriving from tenant object)
- [ ] Billing page, plan cards, and usage charts use the new billing status response
- [ ] Checkout and portal links still go through CP (these need LemonSqueezy API access)
- [ ] Verify billing page renders correctly with the new data source

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: CP checkout seeds tenant_id in LemonSqueezy custom_data [Backend]
**Description:** As the system, I want the checkout session to include the tenant_id in LemonSqueezy's `custom_data` field so webhooks can be linked back to the correct tenant.

**Acceptance Criteria:**
- [ ] Verify `BillingHandler.Checkout` includes `custom_data: {tenant_id: ...}` in the checkout session creation
- [ ] If not present, add it using the authenticated tenant's ID
- [ ] Verify `extractTenantIDFromWebhook()` correctly extracts tenant_id from `meta.custom_data`
- [ ] Add test for tenant_id extraction from webhook payload

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements

- FR-1: Every LemonSqueezy webhook event must be persisted as a billing event in Core
- FR-2: Billing events must follow the standard AllSource event format (event_type, entity_id, payload)
- FR-3: The QS must derive current billing state from the latest billing event per tenant
- FR-4: If no billing events exist for a tenant, the default state is free tier
- FR-5: Core write failures must not block webhook processing (graceful degradation)
- FR-6: Tier resolution must be configurable via `LEMON_SQUEEZY_VARIANT_MAP` env var
- FR-7: Checkout and portal operations remain on the CP (they need the LemonSqueezy API key)

## Non-Goals

- Migrating existing in-memory billing state to Core (fresh state from next webhook)
- Adding Core projections for billing (simple query-latest-event is sufficient for now)
- Changing the LemonSqueezy webhook URL (stays at CP)
- Adding Stripe event sourcing (separate scope)
- Real-time billing notifications via WebSocket

## Technical Considerations

- **Event types follow the convention:** `billing.subscription.created`, `billing.subscription.updated`, `billing.payment.failed`, etc.
- **Entity ID = tenant ID** — consistent with how Core organizes events
- **Eventual consistency:** There's a brief window after webhook receipt where CP has updated but Core event hasn't propagated to QS queries. This is acceptable for billing state.
- **Backwards compatibility:** The CP's existing in-memory tenant metadata update should continue working in parallel during migration. Once QS reads from Core, the CP metadata becomes secondary.
- **Core API key:** CP uses `CORE_API_KEY` (if set) as Bearer token for Core API calls. Already configured on Fly.

## Success Metrics

- Billing events appear in Core after every LemonSqueezy webhook
- QS `/api/billing/status` returns correct tier after subscription change
- Billing state survives CP restart (verified by querying Core)
- No LemonSqueezy secrets on QS deployment

## Open Questions

- Should we backfill existing subscriptions as billing events in Core? (Can be done via a one-time script)
- Should billing events use a dedicated Core projection for faster queries, or is query-latest-event sufficient?
