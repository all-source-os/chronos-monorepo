# Proposal: Service Responsibility Realignment (v0.11.0)

**Status:** Draft
**Date:** 2025-02-14
**Depends on:** v0.10.0 (Control Plane PostgreSQL removal, Core-backed metadata)

## Motivation

The v0.10.0 release moved the Control Plane off PostgreSQL onto Core as its backing store. However, the Query Service still maintains a parallel tenant/user store in PostgreSQL, a full OAuth stack, and billing integration — creating dual sources of truth and blurred responsibilities.

This proposal realigns service boundaries to eliminate overlap and establish clear ownership:

| Concern | Current Owner | Proposed Owner |
|---------|--------------|----------------|
| Tenant lifecycle | QS (PG) + CP (Core) | **CP only** (Core-backed) |
| User identity | QS (PG) + Core (auth) | **Core** (auth API) |
| Billing / LemonSqueezy | QS | **CP** |
| OAuth (Google/GitHub) | QS | **Removed** (API keys + Core auth) |
| Usage metering & quotas | QS | QS (reads tenant quotas from CP/Core) |
| Event API gateway | QS | QS (unchanged) |
| HAL cross-references | N/A | **All services** |

---

## Decision 1: Move LemonSqueezy Billing to Control Plane

### Rationale

Billing is an operational/management concern. It belongs with the service that manages tenants, not with the data plane that proxies events. The Control Plane already owns tenant lifecycle (create, suspend, activate, delete) — billing is a natural extension.

### What Moves

| Component | From (QS) | To (CP) |
|-----------|-----------|---------|
| LemonSqueezy API client | `lib/query_service_ex/billing/lemon_squeezy.ex` | `internal/infrastructure/clients/lemon_squeezy_client.go` |
| Hybrid pricing logic | `lib/query_service_ex/billing/hybrid_pricing.ex` | `internal/application/usecases/billing/` |
| Webhook handler | `controllers/webhook_controller.ex` | `internal/interfaces/http/webhook_handler.go` |
| Billing API endpoints | `controllers/billing_controller.ex` | `internal/interfaces/http/billing_handler.go` |
| Subscription fields | `tenants.tenant` schema (PG) | Core tenant metadata |
| Overage tracking | `tenants.tenant` PG columns | Core tenant metadata |

### New CP Billing Endpoints

```
POST /api/v1/billing/checkout          → Create LemonSqueezy checkout
GET  /api/v1/billing/portal            → Customer portal URL
GET  /api/v1/billing/overage           → Current overage summary
POST /api/v1/billing/overage/enable    → Enable hybrid pricing
POST /api/v1/billing/overage/disable   → Disable hybrid pricing
GET  /api/v1/billing/projected-charges → Projected billing breakdown
POST /api/v1/webhooks/lemonsqueezy     → Webhook receiver (HMAC-verified)
```

### Subscription Data in Core

The tenant entity in Core gains billing metadata (stored as tenant metadata or dedicated fields):

```json
{
  "id": "tenant-123",
  "name": "Acme Corp",
  "status": "active",
  "metadata": {
    "subscription": {
      "lemon_squeezy_customer_id": "cus_...",
      "lemon_squeezy_subscription_id": "sub_...",
      "status": "active",
      "tier": "pro",
      "trial_ends_at": null,
      "subscription_ends_at": "2026-03-14T00:00:00Z"
    },
    "quotas": {
      "events_quota": 1000000,
      "queries_quota": 100000,
      "events_used": 45230,
      "queries_used": 8721,
      "usage_reset_at": "2026-02-01T00:00:00Z"
    },
    "overage": {
      "enabled": true,
      "rate_events_cents": 100,
      "rate_queries_cents": 1000,
      "events_overage": 0,
      "queries_overage": 0
    }
  }
}
```

### What Stays in QS

- **Usage enforcement plug** — QS still checks quotas before allowing events/queries through, but reads quota data from Core (via CP or direct Core tenant API) instead of local PG.
- **Usage increment** — QS increments `events_used` / `queries_used` by calling Core's tenant update API after successful operations.
- **Rate limiting** — Tier-based rate limiting stays in QS (reads tier from tenant metadata).

### QS Quota Flow (After)

```
Request → QS UsageEnforcement plug
             │
             ├─ Cache tenant quotas (TTL: 60s, refreshed from Core)
             ├─ Check: used < quota? → Allow
             ├─ Check: used >= quota + overage disabled? → 402
             ├─ Check: used >= quota + overage enabled? → Allow + headers
             │
             └─ After success: POST Core /api/v1/tenants/{id}/usage/increment
```

---

## Decision 2: Remove Tenant & User Store from Query Service

### Rationale

The Query Service should not own tenant or user identity. Core is the source of truth for tenants (via CP), and Core's auth API manages users. QS maintaining a parallel PostgreSQL store creates sync problems identified in the C4 analysis.

### What Gets Removed from QS

| Component | File | Action |
|-----------|------|--------|
| Tenant Ecto schema | `lib/query_service_ex/tenants/tenant.ex` | Delete |
| Tenants context | `lib/query_service_ex/tenants.ex` | Replace with Core client calls |
| User Ecto schema | `lib/query_service_ex/accounts/user.ex` | Delete |
| Accounts context | `lib/query_service_ex/accounts.ex` | Replace with Core client calls |
| Ecto Repo | `lib/query_service_ex/repo.ex` | Delete |
| PG migrations | `priv/repo/migrations/*` | Archive |
| PG config | `config/*/ecto + postgres config` | Remove |
| `ecto`, `ecto_sql`, `postgrex` deps | `mix.exs` | Remove |

### What Replaces It

QS gets a thin `TenantCache` GenServer that:

1. On authenticated request, fetches tenant from Core (`GET /api/v1/tenants/{id}`)
2. Caches tenant data in ETS (TTL: 60s)
3. Provides `get_tenant/1`, `get_quota/1`, `get_tier/1` functions
4. Invalidated on webhook from CP (new internal endpoint: `POST /internal/tenant-updated`)

```
QS TenantContext plug
  │
  ├─ Extract tenant_id from JWT claims (issued by Core auth)
  ├─ TenantCache.get_tenant(tenant_id)
  │    ├─ ETS hit? → return cached
  │    └─ ETS miss? → GET Core /api/v1/tenants/{id} → cache → return
  ├─ Check tenant.status == "active" → proceed
  └─ Check subscription.status in ["active", "trialing"] → proceed
```

### CP → QS Notification

When CP changes tenant state (suspend, quota update, tier change), it notifies QS:

```
CP suspends tenant
  │
  ├─ Core /api/v1/tenants/{id}/deactivate
  ├─ Core /api/v1/audit/events (log)
  └─ POST QS /internal/tenant-updated { "tenant_id": "...", "action": "suspended" }
       └─ QS invalidates ETS cache for that tenant
```

This requires CP to know QS's internal URL — configured via `QUERY_SERVICE_INTERNAL_URL` env var in CP.

---

## Decision 3: Introduce HAL for Cross-Service Entity References

### Rationale

With responsibilities split across services, clients need a standard way to navigate between entities that live in different services. HAL (Hypertext Application Language, `application/hal+json`) provides `_links` for discoverability without tight coupling.

### HAL Convention

All API responses that reference entities include a `_links` object following [RFC 8288](https://datatracker.ietf.org/doc/html/rfc8288) link relations:

```json
{
  "_links": {
    "self": { "href": "/api/v1/resource/id" },
    "relation": { "href": "/api/v1/other-resource/id", "title": "Human label" }
  }
}
```

Service base URLs are resolved by the client (or a gateway) using a service registry convention:

```json
{
  "_links": {
    "self": { "href": "/api/v1/tenants/t-123" },
    "events": { "href": "{data_plane}/api/v1/events?tenant_id=t-123", "templated": true },
    "billing": { "href": "/api/v1/billing/t-123" },
    "audit": { "href": "/api/v1/audit?tenant_id=t-123" },
    "usage": { "href": "/api/v1/tenants/t-123/usage" }
  }
}
```

### HAL by Service

#### Control Plane (Management Plane)

**Tenant response:**
```json
{
  "id": "t-123",
  "name": "Acme Corp",
  "status": "active",
  "subscription": { "tier": "pro", "status": "active" },
  "_links": {
    "self":     { "href": "/api/v1/tenants/t-123" },
    "stats":    { "href": "/api/v1/tenants/t-123/stats" },
    "usage":    { "href": "/api/v1/tenants/t-123/usage" },
    "billing":  { "href": "/api/v1/billing/portal?tenant_id=t-123" },
    "audit":    { "href": "/api/v1/audit?tenant_id=t-123" },
    "events":   { "href": "{data_plane}/events?tenant_id=t-123", "templated": true },
    "schemas":  { "href": "{data_plane}/schemas?tenant_id=t-123", "templated": true }
  }
}
```

**Audit event response:**
```json
{
  "id": "aud-456",
  "action": "tenant.suspended",
  "tenant_id": "t-123",
  "user_id": "u-789",
  "_links": {
    "self":   { "href": "/api/v1/audit/aud-456" },
    "tenant": { "href": "/api/v1/tenants/t-123" },
    "user":   { "href": "{core}/api/v1/auth/users/u-789", "templated": true }
  }
}
```

**Operation response:**
```json
{
  "id": "op-101",
  "type": "compaction",
  "status": "running",
  "_links": {
    "self":    { "href": "/api/v1/operations/op-101" },
    "tenant":  { "href": "/api/v1/tenants/t-123" },
    "cluster": { "href": "/api/v1/cluster/status" }
  }
}
```

#### Query Service (Data Plane)

**Event response:**
```json
{
  "id": "evt-789",
  "stream_id": "order-stream",
  "event_type": "OrderPlaced",
  "tenant_id": "t-123",
  "data": { "order_id": "ord-001", "amount": 99.99 },
  "_links": {
    "self":       { "href": "/events/evt-789" },
    "stream":     { "href": "/events?stream_id=order-stream" },
    "event_type": { "href": "/events?event_type=OrderPlaced" },
    "entity":     { "href": "/events?entity_id=ord-001" },
    "schema":     { "href": "/schemas/OrderPlaced" },
    "tenant":     { "href": "{mgmt_plane}/api/v1/tenants/t-123", "templated": true }
  }
}
```

**Query result response:**
```json
{
  "events": [...],
  "count": 42,
  "_links": {
    "self":     { "href": "/query?stream_id=order-stream&limit=50" },
    "next":     { "href": "/query?stream_id=order-stream&limit=50&after=evt-789" },
    "tenant":   { "href": "{mgmt_plane}/api/v1/tenants/t-123", "templated": true }
  }
}
```

**Projection response:**
```json
{
  "name": "order-totals",
  "entity_id": "ord-001",
  "state": { "total": 299.97, "count": 3 },
  "_links": {
    "self":      { "href": "/projections/order-totals/ord-001/state" },
    "events":    { "href": "/events?entity_id=ord-001" },
    "snapshot":  { "href": "/snapshots?entity_id=ord-001" }
  }
}
```

#### Core (Database Engine)

Core responses are lower-level and consumed by QS and CP, not directly by end users. HAL links in Core responses reference Core-internal routes:

```json
{
  "events": [...],
  "count": 42,
  "_links": {
    "self": { "href": "/api/v1/events/query?stream_id=order-stream" }
  }
}
```

Core does NOT link to QS or CP — it doesn't know about them. Only the gateway services (QS, CP) add cross-service HAL links.

### HAL Implementation Pattern

Each service implements a `hal` helper module:

**Go (Control Plane):**
```go
type Link struct {
    Href      string `json:"href"`
    Title     string `json:"title,omitempty"`
    Templated bool   `json:"templated,omitempty"`
}

type HALResource struct {
    Links map[string]Link `json:"_links,omitempty"`
}
```

**Elixir (Query Service):**
```elixir
defmodule QueryServiceExWeb.HAL do
  def self(path), do: %{"self" => %{"href" => path}}

  def link(rel, href, opts \\ []) do
    link = %{"href" => href}
    link = if opts[:templated], do: Map.put(link, "templated", true), else: link
    link = if opts[:title], do: Map.put(link, "title", opts[:title]), else: link
    %{rel => link}
  end
end
```

### Service Discovery for Templated Links

Services resolve `{data_plane}`, `{mgmt_plane}`, `{core}` from environment:

```
# Control Plane
DATA_PLANE_URL=https://api.allsource.io       # QS public URL
CORE_SERVICE_URL=http://core:3900              # Core internal URL

# Query Service
MGMT_PLANE_URL=https://admin.allsource.io     # CP public URL
CORE_URL=http://core:3900                      # Core internal URL
```

Clients receive templated links and resolve them against known base URLs, or a future API gateway resolves them before returning to the client.

---

## Decision 4: Remove OAuth from Query Service

### Rationale

With billing and tenant management moving to CP, and user identity managed by Core's auth API, the QS no longer needs to be an identity provider. Clients authenticate via:

1. **API keys** (programmatic access) — already supported by Core
2. **Core auth tokens** (JWT from Core's `/api/v1/auth/login`) — for interactive use
3. **CP-issued JWT** (admin operations) — for management plane

QS validates tokens but doesn't issue them.

### What Gets Removed

| Category | Files | Count |
|----------|-------|-------|
| Dependencies | `ueberauth`, `ueberauth_google`, `ueberauth_github`, `guardian` | 4 deps |
| Guardian module | `accounts/guardian.ex` | 1 file |
| Auth controller | `controllers/auth_controller.ex` | 1 file |
| Auth pipeline | `plugs/auth_pipeline.ex` (rewrite) | 1 file |
| Auth error handler | `plugs/auth_error_handler.ex` | 1 file |
| OAuth config | `config.exs`, `dev.exs`, `runtime.exs` blocks | 3 files |
| OAuth test helpers | `test/support/auth_helpers.ex` | 1 file |
| OAuth tests | `test/**/auth_controller_test.exs` | 1 file |
| User schema | `accounts/user.ex` (OAuth fields) | 1 file |
| Accounts context | `accounts.ex` (OAuth functions) | 1 file |
| Database | `repo.ex`, migrations, PG config | ~5 files |

### What Replaces It

QS auth pipeline becomes a **token validator**, not a token issuer:

```elixir
defmodule QueryServiceExWeb.Plugs.AuthPipeline do
  @doc """
  Validates auth tokens from two sources:
  1. API key: X-API-Key header → verified against Core
  2. JWT: Authorization: Bearer <token> → verified with shared secret

  Sets conn.assigns.tenant_id and conn.assigns.user_id on success.
  """

  def call(conn, _opts) do
    cond do
      api_key = get_req_header(conn, "x-api-key") ->
        verify_api_key(conn, api_key)

      bearer = get_bearer_token(conn) ->
        verify_jwt(conn, bearer)

      dev_mode?() ->
        assign_dev_context(conn)

      true ->
        unauthorized(conn)
    end
  end
end
```

JWT verification uses a shared secret (same `JWT_SECRET` env var across CP and QS) so tokens issued by Core or CP are valid in QS without a round-trip.

### Auth Flow (After)

```
Developer signs up:
  Browser → CP /api/v1/auth/register → Core creates user → CP issues JWT

Developer uses API:
  App → QS with X-API-Key header → QS verifies key via Core → proceeds

  OR

  App → QS with Bearer JWT → QS validates locally (shared secret) → proceeds

Admin manages platform:
  Browser → CP with Bearer JWT → CP validates + RBAC → proceeds
```

---

## Migration Plan

### Phase 1: HAL Foundation (Non-Breaking)
- Add `_links` to all CP responses (tenant, audit, config, operations)
- Add `_links` to all QS responses (events, projections, schemas, analytics)
- Core responses unchanged (internal protocol)
- **Zero breaking changes** — `_links` is additive

### Phase 2: Billing Migration (CP Gains, QS Keeps Temporarily)
- Implement LemonSqueezy client in CP (Go)
- Implement webhook handler in CP
- Implement billing endpoints in CP
- Add `POST /internal/tenant-updated` to QS for cache invalidation
- **Run both billing stacks in parallel during transition**

### Phase 3: QS Auth Simplification
- Replace Guardian pipeline with shared-secret JWT validator
- Add API key verification via Core
- Remove OAuth dependencies, controllers, schemas
- Keep dev mode bypass (`AUTH_DISABLED=true`)
- Remove user schema and Accounts context

### Phase 4: QS Tenant Store Removal
- Implement `TenantCache` GenServer backed by ETS + Core API
- Replace all `Repo.get(Tenant, id)` calls with `TenantCache.get(id)`
- Replace usage increment with Core API call
- Remove Ecto, postgrex, PG config
- Remove migrations (archive)
- **QS becomes fully stateless** (no database dependency)

### Phase 5: Cleanup
- Remove deprecated QS billing endpoints (return 301 to CP)
- Remove QS OAuth routes
- Update OpenAPI specs
- Update Docker compose (remove QS PostgreSQL dependency)

---

## Architecture After Realignment

```
┌─────────────────────────────────────────────────────────────────────┐
│                     AllSource DBaaS Platform                        │
│                                                                     │
│  ┌──────────────────────────┐     ┌──────────────────────────┐     │
│  │   Query Service           │     │   Control Plane           │     │
│  │   (Data Plane)            │     │   (Management Plane)      │     │
│  │   Elixir · port 3902     │     │   Go · port 3901          │     │
│  │                           │     │                           │     │
│  │ OWNS:                     │     │ OWNS:                     │     │
│  │ • Event API gateway       │     │ • Tenant lifecycle        │     │
│  │ • Read consistency routing│     │ • Billing (LemonSqueezy)  │     │
│  │ • Analytics extensions    │     │ • Subscription management │     │
│  │ • Rate limiting (by tier) │     │ • RBAC + Policy engine    │     │
│  │ • Usage quota enforcement │     │ • Operations (snap/comp)  │     │
│  │ • Usage metering          │     │ • Audit trail             │     │
│  │ • Cluster health tracking │     │ • Config management       │     │
│  │                           │     │ • Webhook processing      │     │
│  │ DELEGATES:                │     │                           │     │
│  │ • Tenant data → Core     │     │ DELEGATES:                │     │
│  │ • Auth → Core/shared JWT │     │ • All data → Core         │     │
│  │                           │     │ • Auth → Core             │     │
│  │ NO DATABASE               │     │                           │     │
│  │ (ETS cache only)          │     │ NO DATABASE               │     │
│  └────────────┬──────────────┘     └────────────┬──────────────┘     │
│               │                                 │                    │
│               │ events, queries,                │ tenants, audit,    │
│               │ projections, schemas            │ config, billing,   │
│               │                                 │ operations, auth   │
│               └────────────┬────────────────────┘                    │
│                            ▼                                         │
│               ┌──────────────────────────┐                           │
│               │      AllSource Core      │                           │
│               │      (Rust · port 3900)  │                           │
│               │                          │                           │
│               │  Source of truth for:    │                           │
│               │  • Events (WAL+Parquet)  │                           │
│               │  • Projections, Schemas  │                           │
│               │  • Tenants (metadata)    │                           │
│               │  • Users & API keys      │                           │
│               │  • Audit log             │                           │
│               │  • System config         │                           │
│               │  • Snapshots, Pipelines  │                           │
│               └──────────────────────────┘                           │
│                                                                      │
│  PostgreSQL: REMOVED from all services                               │
│  (LemonSqueezy is the billing system of record)                     │
└──────────────────────────────────────────────────────────────────────┘
```

### HAL Link Flow (Example: Admin Views Tenant)

```
GET /api/v1/tenants/t-123  →  Control Plane

Response:
{
  "id": "t-123",
  "name": "Acme Corp",
  "status": "active",
  "subscription": {
    "tier": "pro",
    "status": "active",
    "events_quota": 1000000,
    "events_used": 45230
  },
  "_links": {
    "self":      { "href": "/api/v1/tenants/t-123" },
    "stats":     { "href": "/api/v1/tenants/t-123/stats" },
    "billing":   { "href": "/api/v1/billing/portal?tenant_id=t-123" },
    "audit":     { "href": "/api/v1/audit?tenant_id=t-123" },
    "events":    { "href": "https://api.allsource.io/events?tenant_id=t-123" },
    "schemas":   { "href": "https://api.allsource.io/schemas?tenant_id=t-123" },
    "config":    { "href": "/api/v1/config" }
  }
}
```

Client follows `_links.events.href` → hits QS → QS validates JWT → fetches from Core → returns events with their own `_links` back to tenant, schema, stream, etc.

---

## Open Questions

1. **Usage increment latency** — Should QS fire-and-forget usage increments to Core, or wait for acknowledgment? Fire-and-forget is faster but risks undercounting on QS crash.

2. **Tenant cache TTL** — 60s is proposed. Too long means stale quotas (up to 60s of overage before enforcement). Too short means extra Core round-trips. Consider WebSocket subscription to Core tenant change events instead.

3. **Shared JWT secret vs token exchange** — Shared secret is simpler but means all services must rotate together. Token exchange (QS calls Core to validate) adds latency but decouples secrets.

4. **LemonSqueezy metered billing** — Currently QS reports overage directly to LemonSqueezy. After migration, CP reports it. The metered billing item IDs must be stored in Core tenant metadata.
