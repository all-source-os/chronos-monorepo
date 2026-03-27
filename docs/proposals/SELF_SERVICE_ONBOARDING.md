# Self-Service Onboarding — Gap Analysis

**Date:** 2026-03-27
**Priority:** Critical — blocks team sync and any paid usage

## The Goal

A user runs `/allsource-onboard` (or signs up on all-source.xyz) and gets:
1. An isolated tenant (their data is private)
2. An API key for `cn sync`
3. Dashboard access to visualize their tasks
4. A way to pay for usage

## Current Reality — 4 Gaps

### Gap 1: No tenant isolation on signup (Critical)

**Problem:** `POST /api/v1/auth/register` creates users in the `default` tenant. All users in `default` share the same 44K+ events. A new chronis sync user can see every event ever ingested.

**Fix:** Registration must create a dedicated tenant per user (or per team):

```
POST /api/v1/auth/register
  → Create user
  → Create tenant "tenant-{user_id}"
  → Assign user to tenant
  → Return JWT with tenant_id = "tenant-{user_id}"
```

**Effort:** Rock (4h) — needs changes in Core's auth handler + tenant creation logic.

### Gap 2: Dashboard auth ≠ Core auth (Critical)

**Problem:** The web dashboard uses OAuth (Google/GitHub) via Control Plane → sets httpOnly cookie. Core uses its own JWT auth (register/login endpoints). These are two completely separate identity systems. A user who signs up via OAuth can't get a Core API key, and a user who registers on Core can't log into the dashboard.

**Fix:** Unify auth — either:
- **Option A:** Control Plane provisions a Core user + API key during OAuth signup (adds a step to the OAuth callback)
- **Option B:** Core accepts Control Plane JWTs (add CP's JWT secret as a trusted issuer)
- **Option C:** Move Core auth to Control Plane entirely (Core becomes auth-free, CP is the gateway)

**Effort:** Rock (1-2 days) — depends on which option. Option A is smallest change.

### Gap 3: No billing (Blocks monetization)

**Problem:** There's a LemonSqueezy integration in Control Plane (webhook handler exists) but no connection between payment → tenant → quota enforcement. A user can sync unlimited events for free.

**Fix:** The billing PRD exists (`docs/proposals/prd-event-sourced-billing.md`). Key steps:
1. LemonSqueezy checkout creates a subscription
2. Webhook handler provisions tenant quotas
3. Core enforces quotas on ingest (reject events over limit)

**Effort:** Rock (1 week) — needs end-to-end wiring.

### Gap 4: Sync pulls ALL events, no pagination (Critical)

**Problem:** `cn sync` queries remote Core for all events since last sync. On first sync with no timestamp, it pulls ALL events in the tenant. With 44K events, this causes a 502 timeout from Fly.io's proxy (30s limit).

**Fix:** Add pagination to the sync pull:

```rust
// Instead of one query for all events:
let events = remote.query_events(pull_since).await?;

// Paginate in chunks of 1000:
loop {
    let events = remote.query_events_page(pull_since, offset, 1000).await?;
    if events.is_empty() { break; }
    // ingest batch...
    offset += events.len();
}
```

**Effort:** Sand (2h) — needs pagination support in HttpCoreClient + sync loop.

## Priority Order

| # | Gap | Blocks | Effort |
|---|-----|--------|--------|
| 1 | Tenant isolation on signup | Team sync privacy | Rock (4h) |
| 2 | Sync pagination | First sync on any non-empty tenant | Sand (2h) |
| 3 | Unified auth | Dashboard access for sync users | Rock (1-2d) |
| 4 | Billing | Monetization | Rock (1w) |

## Immediate Workaround (today)

For the team: use the bootstrap API key to create a dedicated tenant manually, create users in that tenant, and set the `tenant_id` in API key creation. This gives isolation without product changes. Requires the bootstrap key which is in Fly.io secrets.
