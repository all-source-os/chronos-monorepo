# Production Architecture: Users, Tenants & Billing

## Current State

### Data Storage Responsibilities

**AllSource Core (Rust) is the source of truth for all event data.** It has a full durability stack: WAL (write-ahead log with CRC32 checksums, configurable fsync), Parquet columnar files (Snappy compression), and in-memory DashMap for sub-microsecond reads. Event data survives restarts via WAL crash recovery.

**The Query Service (Elixir + Postgres) is the source of truth for operational metadata**: users, tenants, API keys, subscriptions, billing, and usage metering. It acts as the API gateway to Core — handling auth, rate limiting, and tenant isolation.

**Core's user/tenant metadata is in-memory only** — this is a known gap. Core has its own DashMap-based user and tenant stores that do not persist across restarts. These are separate from event storage and are planned for removal (the query service should be the sole authority for user/tenant data).

### What works end-to-end today

| Concern | Where it lives | Status |
|---|---|---|
| **Event storage** | Core → WAL + Parquet + DashMap | Durable, crash-safe |
| **Event queries** | Core → DashMap (in-memory) | 11.9μs latency, 469K events/sec |
| **Projections & snapshots** | Core → DashMap | Operational |
| **User accounts** | Query Service → Postgres | Google/GitHub OAuth only |
| **Tenants** | Query Service → Postgres | Auto-created on signup, 14-day trial |
| **Subscriptions** | Query Service → LemonSqueezy | Full webhook lifecycle (7 events) |
| **Billing** | Query Service → LemonSqueezy | Checkout, portal, hybrid metered pricing |
| **Usage quotas** | Query Service → Postgres | Per-request enforcement via plugs |
| **API keys** | Query Service → Postgres | CRUD with scopes |

### What does NOT work

1. **Email/password login** — the web app has the form, but the query service only implements OAuth. `POST /api/auth/login` doesn't exist on the backend.
2. **Core's auth is disconnected** — it has its own in-memory user store with username/password (Argon2), completely separate from the query service's Postgres users. No sync between them.
3. **Control Plane is stateless** — its tenants/users are in-memory maps, lost on restart. It proxies login/register to Core (not the query service), so it's on a different auth system.
4. **No cross-service user sync** — a user who signs up via Google OAuth in the query service has no identity in Core's user store or the Control Plane.

## Production Request Flow

```
Browser → Web App (Next.js)
              ↓
         Query Service (port 3902) ← auth & tenant authority
              ├── Postgres (users, tenants, API keys, usage)
              ├── LemonSqueezy (subscriptions, billing)
              └── HTTP/WebSocket → Core (port 3900) ← event data authority
                                        ├── DashMap (in-memory, sub-μs reads)
                                        ├── WAL (crash-safe durability)
                                        └── Parquet (columnar persistence)
```

Core is the database for event data. The query service authenticates requests and enforces tenant isolation before proxying event operations to Core.

The Control Plane (port 3901) is effectively **unused by the web app** — it's not called by the frontend at all.

## Managing Production

### Users
Sign up via OAuth through the web dashboard. No admin panel to manage users yet — requires direct Postgres access or a new admin API.

### Tenants
Auto-created on first OAuth signup. Manageable via `GET/PUT /api/tenant` (authenticated, query service).

### Subscriptions
- User clicks upgrade in dashboard → `POST /api/billing/checkout` → redirected to LemonSqueezy
- LemonSqueezy sends webhooks → `POST /api/webhooks/lemonsqueezy` → tenant tier/status updated in Postgres
- Customer portal: `GET /api/billing/portal` returns the LemonSqueezy self-service URL

### Usage
Tracked automatically per request. Resets on `subscription_payment_success` webhook. Overage billing via `POST /api/billing/overage/enable`.

## Key Gaps

The biggest architectural issue is that **three services have independent, unsynchronized auth/tenant systems**:

1. Keep the query service as the sole authority for users/tenants/billing (it already is)
2. Core should either trust the query service's JWT tokens or run in dev mode behind a private network
3. The Control Plane needs to either be retired or reworked to talk to the query service instead of Core

## Service Comparison

| Concern | Query Service (Elixir) | Core (Rust) | Control Plane (Go) |
|---|---|---|---|
| **Primary Role** | API gateway, auth, billing | Event storage (the database) | Unused |
| **Event Storage** | None (proxies to Core) | WAL + Parquet + DashMap (durable) | None |
| **User Storage** | Postgres (Ecto) - OAuth users | In-memory DashMap (ephemeral) | In-memory map - proxies to Core |
| **User Auth Model** | Google/GitHub OAuth + Guardian JWT | Username/password + Argon2 + JWT | JWT validation only |
| **Tenant Storage** | Postgres (Ecto) with full billing | In-memory DashMap (ephemeral) | In-memory map, minimal fields |
| **Subscription/Billing** | LemonSqueezy (full integration) | None | None |
| **Usage Metering** | Per-billing-period (events_used, queries_used) | Per-day/hour (events_today, queries_this_hour) | None |
| **Quota Tiers** | free/starter/pro/enterprise | free_tier/professional/unlimited | None |
| **Event Persistence** | N/A | WAL + Parquet (crash-safe, durable) | N/A |
| **User/Tenant Persistence** | PostgreSQL (durable) | In-memory (lost on restart) | In-memory (lost on restart) |
