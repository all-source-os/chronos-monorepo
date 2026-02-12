# Production Architecture: Users, Tenants & Billing

## Current State

**The Query Service (Elixir + Postgres) is the only production-ready source of truth.** The Rust Core and Go Control Plane both use in-memory storage that resets on every deploy.

### What works end-to-end today

| Concern | Where it lives | Status |
|---|---|---|
| **User accounts** | Query Service → Postgres | Google/GitHub OAuth only |
| **Tenants** | Query Service → Postgres | Auto-created on signup, 14-day trial |
| **Subscriptions** | Query Service → LemonSqueezy | Full webhook lifecycle (7 events) |
| **Billing** | Query Service → LemonSqueezy | Checkout, portal, hybrid metered pricing |
| **Usage quotas** | Query Service → Postgres | Per-request enforcement via plugs |
| **API keys** | Query Service → Postgres | CRUD with scopes |

### What does NOT work

1. **Email/password login** — the web app has the form, but the query service only implements OAuth. `POST /api/auth/login` doesn't exist on the backend.
2. **Rust Core auth is disconnected** — it has its own in-memory user store with username/password (Argon2), completely separate from the query service's Postgres users. No sync between them.
3. **Control Plane is stateless** — its tenants/users are in-memory maps, lost on restart. It proxies login/register to the Rust Core (not the query service), so it's on a different auth system.
4. **No cross-service user sync** — a user who signs up via Google OAuth in the query service has no identity in the Rust Core or Control Plane.

## Production Request Flow

```
Browser → Web App (Next.js)
              ↓
         Query Service (port 3902) ← source of truth
              ├── Postgres (users, tenants, API keys, usage)
              ├── LemonSqueezy (subscriptions, billing)
              └── WebSocket → Rust Core (port 3900, event storage only)
```

The Rust Core serves as a **dumb event store** in production — the query service connects to it for event ingestion/queries, but auth and tenant context come from the query service's own middleware (JWT from Guardian, tenant from Postgres).

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
2. The Rust Core should either trust the query service's JWT tokens or run in dev mode behind a private network
3. The Control Plane needs to either be retired or reworked to talk to the query service instead of the Core

## Service Auth/Tenant Comparison

| Concern | Query Service (Elixir) | Rust Core | Control Plane (Go) |
|---|---|---|---|
| **User Storage** | Postgres (Ecto) - OAuth users | In-memory DashMap - password users | In-memory map - proxies to Core |
| **User Auth Model** | Google/GitHub OAuth + Guardian JWT | Username/password + Argon2 + JWT | JWT validation only |
| **Tenant Storage** | Postgres (Ecto) with full billing | In-memory DashMap, operational only | In-memory map, minimal fields |
| **Subscription/Billing** | LemonSqueezy (full integration) | None | None |
| **Usage Metering** | Per-billing-period (events_used, queries_used) | Per-day/hour (events_today, queries_this_hour) | None |
| **Quota Tiers** | free/starter/pro/enterprise | free_tier/professional/unlimited | None |
| **Data Persistence** | PostgreSQL (production-ready) | In-memory (lost on restart) | In-memory (lost on restart) |
