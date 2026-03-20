# PRD: Service Responsibility Realignment (v0.11.0)

## Overview

Realign service boundaries across the AllSource Chronos platform to eliminate dual sources of truth and establish clear ownership. The Query Service (Elixir) currently maintains a parallel tenant/user store in PostgreSQL, a full OAuth stack, and billing integration that overlaps with the Control Plane (Go) and Core (Rust). This effort moves billing to Control Plane, removes OAuth from QS, removes QS's PostgreSQL dependency entirely, and adds HAL links for cross-service discoverability.

**Key outcome:** QS becomes a stateless data-plane gateway (ETS cache only, no database). CP becomes the single management plane for tenants, billing, and admin operations. Core remains the database.

## Goals

- Eliminate dual tenant/user stores (QS PG vs Core-backed CP) — single source of truth in Core via CP
- Move LemonSqueezy billing from QS to CP where tenant lifecycle already lives
- Remove OAuth (Google/GitHub) from QS — clients authenticate via API keys or Core-issued tokens
- Make QS fully stateless — remove Ecto, Postgrex, and all PostgreSQL configuration
- Add HAL `_links` to all API responses for cross-service entity navigation
- Implement tiered plans with event volume caps for billing model
- Use API key header authentication between services (skip JWT for now)
- Add Docker integration tests to verify cross-service flows after each phase
- Tenant cache with 120-second TTL and CP→QS invalidation webhook
- Async usage increment with retry queue from QS to Core

## Quality Gates

These commands must pass for every user story:

**Control Plane (Go) stories:**
- `cd apps/control-plane && go test ./...` - Unit tests
- `cd apps/control-plane && go vet ./...` - Static analysis

**Query Service (Elixir) stories:**
- `cd apps/query-service && mix test` - Unit tests
- `cd apps/query-service && mix compile --warnings-as-errors` - Compilation checks

**Core (Rust) stories:**
- `cd apps/core && cargo test` - Unit tests
- `cd apps/core && cargo clippy -- -D warnings` - Linting

**Integration stories:**
- Docker Compose stack starts and all health checks pass
- Cross-service API calls succeed end-to-end

## User Stories

### Phase 1: HAL Foundation (Non-Breaking)

### US-001: Add HAL link helper module to Control Plane
As a developer, I want a reusable HAL link builder in the Control Plane so that all CP responses include standardized `_links`.

**Acceptance Criteria:**
- [ ] Create `internal/interfaces/http/hal.go` with `Link` struct (`Href`, `Title`, `Templated` fields)
- [ ] Add `HALResource` composable struct with `Links map[string]Link` serialized as `_links`
- [ ] Support helper functions: `SelfLink(path)`, `NewLink(href, opts...)`, `MergeLinks(...)`
- [ ] Read `DATA_PLANE_URL` env var for templated cross-service links to QS
- [ ] Unit tests cover link generation, templated links, and JSON serialization

### US-002: Add HAL links to Control Plane tenant responses
As an API consumer, I want tenant responses from CP to include `_links` for related resources (stats, billing, audit, events) so I can navigate the API without hardcoding URLs.

**Acceptance Criteria:**
- [ ] `GET /api/v1/tenants` list response includes `_links.self` on each tenant
- [ ] `GET /api/v1/tenants/:id` includes links: `self`, `stats`, `usage`, `billing`, `audit`, `events` (templated), `schemas` (templated)
- [ ] `POST /api/v1/tenants` response includes `_links.self`
- [ ] Existing response fields are unchanged (additive only)
- [ ] Templated links use `{data_plane}` placeholder resolved from `DATA_PLANE_URL` env var

### US-003: Add HAL links to Control Plane audit, config, and operation responses
As an API consumer, I want audit events, config entries, and operation responses to include `_links` so I can navigate between related entities.

**Acceptance Criteria:**
- [ ] Audit event responses include links: `self`, `tenant`, `user` (templated to Core)
- [ ] Config entry responses include links: `self`
- [ ] Operation responses include links: `self`, `tenant`, `cluster`
- [ ] All existing response fields unchanged

### US-004: Add HAL link helper module to Query Service
As a developer, I want a reusable HAL link builder in the Query Service so that all QS responses include standardized `_links`.

**Acceptance Criteria:**
- [ ] Create `lib/query_service_ex_web/hal.ex` with `self/1`, `link/2-3` functions
- [ ] Support `templated: true` and `title` options
- [ ] Read `MGMT_PLANE_URL` env var for cross-service links to CP
- [ ] Unit tests cover link building and JSON output format

### US-005: Add HAL links to Query Service event and query responses
As an API consumer, I want event and query responses from QS to include `_links` for stream navigation, schema lookup, and tenant reference.

**Acceptance Criteria:**
- [ ] Single event responses include links: `self`, `stream`, `event_type`, `entity`, `schema`, `tenant` (templated to CP)
- [ ] Query result responses include links: `self`, `next` (pagination), `tenant` (templated)
- [ ] Batch/list responses include `_links.self`
- [ ] Existing response format unchanged (additive)

### US-006: Add HAL links to Query Service projection and schema responses
As an API consumer, I want projection and schema responses from QS to include `_links` for related resources.

**Acceptance Criteria:**
- [ ] Projection responses include links: `self`, `events`, `snapshot`
- [ ] Schema responses include links: `self`
- [ ] Stream list responses include links: `self`, `events`
- [ ] Event type list responses include links: `self`, `events`

---

### Phase 2: Billing Migration

### US-007: Implement LemonSqueezy API client in Control Plane
As a developer, I want a Go HTTP client for the LemonSqueezy API so that CP can manage subscriptions, create checkouts, and report usage.

**Acceptance Criteria:**
- [ ] Create `internal/infrastructure/clients/lemon_squeezy_client.go`
- [ ] Implement methods: `CreateCheckout`, `GetCustomerPortalURL`, `ReportUsage`, `GetSubscription`
- [ ] Read `LEMON_SQUEEZY_API_KEY`, `LEMON_SQUEEZY_STORE_ID`, `LEMON_SQUEEZY_WEBHOOK_SECRET` from env
- [ ] HTTP client uses connection pooling (resty)
- [ ] Unit tests with mock HTTP responses cover success and error paths
- [ ] Support tiered plan variant mapping (env var `LEMON_SQUEEZY_VARIANT_MAP` as JSON: `{"free":"var_1","starter":"var_2","pro":"var_3","enterprise":"var_4"}`)

### US-008: Add subscription metadata to Core tenant entity
As the Control Plane, I want to store billing/subscription data in Core tenant metadata so that subscription state is durable and queryable.

**Acceptance Criteria:**
- [ ] CP writes subscription fields to tenant metadata via existing `UpdateTenantQuotas` or a new `UpdateTenantMetadata` method on CoreClient
- [ ] Metadata includes: `subscription` (LS customer/subscription IDs, status, tier, trial_ends_at, subscription_ends_at), `quotas` (events_quota, queries_quota, usage counters, reset date), `overage` (enabled, rates, counters)
- [ ] Tier-to-quota mapping defined: free (10K events/5K queries), starter (100K/50K), pro (1M/100K), enterprise (unlimited)
- [ ] Existing tenant CRUD operations preserve metadata fields they don't modify

### US-009: Implement billing API endpoints in Control Plane
As an admin user, I want billing endpoints on the Control Plane so I can manage subscriptions without going through QS.

**Acceptance Criteria:**
- [ ] `POST /api/v1/billing/checkout` — creates LemonSqueezy checkout URL, requires tenant_id param
- [ ] `GET /api/v1/billing/portal` — returns customer portal URL for authenticated tenant
- [ ] `GET /api/v1/billing/overage` — returns current overage summary from tenant metadata
- [ ] `POST /api/v1/billing/overage/enable` — enables hybrid pricing for tenant
- [ ] `POST /api/v1/billing/overage/disable` — disables hybrid pricing for tenant
- [ ] `GET /api/v1/billing/projected-charges` — returns projected billing based on current usage rate
- [ ] All endpoints require authentication (PermissionManageTenants or tenant self-service)
- [ ] All responses include HAL `_links`
- [ ] Billing handler registered in router (`main.go`)

### US-010: Implement LemonSqueezy webhook handler in Control Plane
As the platform, I want CP to receive and process LemonSqueezy webhooks so that subscription changes are reflected in tenant metadata.

**Acceptance Criteria:**
- [ ] `POST /api/v1/webhooks/lemonsqueezy` endpoint (public, no JWT auth)
- [ ] HMAC signature verification using `LEMON_SQUEEZY_WEBHOOK_SECRET`
- [ ] Handle events: `subscription_created`, `subscription_updated`, `subscription_cancelled`, `subscription_expired`, `subscription_payment_failed`
- [ ] On subscription change: update tenant metadata in Core (tier, status, quota resets)
- [ ] On subscription cancel/expire: suspend tenant via existing suspend use case
- [ ] Audit log entry for each webhook processed
- [ ] Return 200 on success, 400 on signature failure, 500 on processing error

### US-011: Add hybrid pricing and overage reporting use cases to Control Plane
As the platform, I want CP to calculate and report overage charges so that metered billing works after migration from QS.

**Acceptance Criteria:**
- [ ] Create `internal/application/usecases/billing/calculate_overage.go` — computes overage units from tenant metadata (events_used - events_quota)
- [ ] Create `internal/application/usecases/billing/report_usage.go` — reports overage to LemonSqueezy via metered billing API
- [ ] Tracks reported overage in tenant metadata to prevent double-reporting
- [ ] Scheduler runs overage reporting hourly (add to existing scheduler)
- [ ] Unit tests cover: no overage, events overage only, queries overage only, both, overage disabled

### US-012: Add internal tenant-updated webhook endpoint to Query Service
As the Control Plane, I want to notify QS when tenant state changes so that QS invalidates its cache immediately instead of waiting for TTL.

**Acceptance Criteria:**
- [ ] `POST /internal/tenant-updated` endpoint on QS (internal network only, no public exposure)
- [ ] Accepts JSON: `{"tenant_id": "...", "action": "suspended|activated|updated|deleted"}`
- [ ] Validates request comes from trusted source (shared internal API key via `INTERNAL_API_KEY` env var)
- [ ] On receive: invalidates ETS cache entry for that tenant_id
- [ ] Returns 200 on success, 401 on invalid key, 404 if tenant not in cache (still 200, idempotent)
- [ ] CP configured with `QUERY_SERVICE_INTERNAL_URL` env var to call this endpoint

### US-013: Integration test — billing flow across CP, Core, and QS
As a developer, I want to verify the full billing flow works across services in Docker so that we can be confident in the migration.

**Acceptance Criteria:**
- [ ] Create `docker-compose.test.yml` in monorepo root with CP, QS, Core services
- [ ] Test script: create tenant → set subscription tier via CP → verify QS reads correct quota
- [ ] Test script: simulate usage increment → verify overage calculation in CP
- [ ] Test script: CP sends tenant-updated webhook → verify QS cache invalidation
- [ ] All services start, pass health checks, and communicate successfully
- [ ] Tests can run against existing `docker-compose.allsource.yml` stack as well

---

### Phase 3: QS Auth Simplification

### US-014: Implement API key verification in Query Service via Core
As a developer using API keys, I want QS to verify my API key against Core so that I can authenticate without OAuth.

**Acceptance Criteria:**
- [ ] New plug `QueryServiceExWeb.Plugs.ApiKeyAuth` extracts `X-API-Key` header
- [ ] Verifies key by calling Core's API key validation endpoint (or CP endpoint that proxies to Core)
- [ ] On success: sets `conn.assigns.tenant_id`, `conn.assigns.user_id`, `conn.assigns.auth_method` = `:api_key`
- [ ] On failure: returns 401 with JSON error body
- [ ] Caches verified API keys in ETS with 120-second TTL (same as tenant cache)
- [ ] Key revocation reflected within cache TTL window

### US-015: Implement shared-secret JWT validation in Query Service
As an interactive user with a Core-issued JWT, I want QS to validate my token locally so that authentication is fast with no round-trip.

**Acceptance Criteria:**
- [ ] New plug `QueryServiceExWeb.Plugs.JwtAuth` extracts `Authorization: Bearer <token>` header
- [ ] Validates JWT using `JWT_SECRET` env var (shared with Core and CP) — HMAC HS256
- [ ] Extracts claims: `sub` (user_id), `tenant_id`, `role`, `is_api_key`
- [ ] Sets `conn.assigns.tenant_id`, `conn.assigns.user_id`, `conn.assigns.role`, `conn.assigns.auth_method` = `:jwt`
- [ ] Rejects expired tokens (checks `exp` claim)
- [ ] No external dependency — pure local validation

### US-016: Replace QS auth pipeline with unified token validator
As a developer, I want QS to use a single auth pipeline that supports both API keys and JWTs so that the old Guardian/OAuth pipeline can be removed.

**Acceptance Criteria:**
- [ ] Rewrite `QueryServiceExWeb.Plugs.AuthPipeline` to try: (1) API key header, (2) Bearer JWT, (3) dev mode bypass
- [ ] Remove `guardian` dependency from `mix.exs`
- [ ] Remove `QueryServiceEx.Accounts.Guardian` module
- [ ] Remove `QueryServiceExWeb.Plugs.AuthErrorHandler` (replaced by inline error responses)
- [ ] Dev mode bypass still works when `AUTH_DISABLED=true`
- [ ] All existing authenticated routes work with both API key and JWT auth
- [ ] Existing tests updated to use new auth mechanism

### US-017: Remove OAuth from Query Service
As a developer, I want OAuth removed from QS so that the codebase is simpler and auth responsibility is clear.

**Acceptance Criteria:**
- [ ] Remove `ueberauth`, `ueberauth_google`, `ueberauth_github` dependencies from `mix.exs`
- [ ] Delete `lib/query_service_ex_web/controllers/auth_controller.ex`
- [ ] Remove OAuth routes from `router.ex` (`/api/auth/:provider`, `/api/auth/:provider/callback`)
- [ ] Remove OAuth config blocks from `config.exs`, `dev.exs`, `runtime.exs`
- [ ] Remove OAuth-related test files and helpers
- [ ] Keep `/api/auth/me` route — reimplement to return user info from JWT claims (no DB lookup)
- [ ] `mix compile --warnings-as-errors` passes with no dead code warnings

### US-018: Integration test — auth flows across services
As a developer, I want to verify that API key and JWT auth work end-to-end in Docker so that we're confident OAuth removal is safe.

**Acceptance Criteria:**
- [ ] Test: create API key via CP/Core → use key to call QS event endpoint → success
- [ ] Test: login via Core auth → get JWT → use JWT to call QS event endpoint → success
- [ ] Test: expired JWT → QS returns 401
- [ ] Test: revoked API key → QS returns 401 (within cache TTL)
- [ ] Test: no auth header → QS returns 401
- [ ] Tests run in Docker Compose test environment

---

### Phase 4: QS Tenant Store Removal

### US-019: Implement TenantCache GenServer in Query Service
As a developer, I want QS to cache tenant data from Core in ETS so that tenant lookups are fast without a local database.

**Acceptance Criteria:**
- [ ] Create `lib/query_service_ex/tenant_cache.ex` GenServer
- [ ] ETS table stores tenant data with 120-second TTL per entry
- [ ] Public API: `get_tenant(tenant_id)`, `get_quota(tenant_id)`, `get_tier(tenant_id)`, `invalidate(tenant_id)`
- [ ] Cache miss triggers `GET /api/v1/tenants/{id}` to Core (via RustCoreClient or direct HTTP)
- [ ] Handles Core unavailability gracefully — returns stale data if available, error if not
- [ ] `invalidate/1` called from tenant-updated webhook handler (US-012)
- [ ] GenServer started in application supervision tree

### US-020: Implement async usage increment with retry queue in Query Service
As the platform, I want QS to increment usage counters in Core asynchronously with retries so that event processing isn't blocked by billing updates.

**Acceptance Criteria:**
- [ ] Create `lib/query_service_ex/usage_reporter.ex` GenServer
- [ ] Buffers usage increments in-process and flushes to Core periodically (every 5 seconds or 100 increments, whichever first)
- [ ] Calls `POST /api/v1/tenants/{id}/usage/increment` on Core with batched count
- [ ] Retry on failure: exponential backoff, max 3 retries, 2s/4s/8s delays
- [ ] On persistent failure: log error, increment dropped counter metric, continue processing
- [ ] Prometheus metric: `qs_usage_increments_total`, `qs_usage_increments_failed`
- [ ] Supervised in application tree, graceful shutdown flushes pending increments

### US-021: Replace QS TenantContext plug to use TenantCache instead of Ecto
As a developer, I want the QS tenant context plug to read from TenantCache (ETS/Core) instead of PostgreSQL so that the PG dependency can be removed.

**Acceptance Criteria:**
- [ ] Rewrite `QueryServiceExWeb.Plugs.TenantContext` to call `TenantCache.get_tenant/1` instead of `Repo.get(Tenant, id)`
- [ ] Subscription status check uses tenant metadata from cache (not PG columns)
- [ ] Quota data read from cached tenant metadata (not PG columns)
- [ ] Rate limiting tier read from cached tenant metadata
- [ ] All existing tenant-scoped routes continue to work identically

### US-022: Replace QS UsageEnforcement plug to use cached quota data
As the platform, I want QS usage enforcement to check quotas from Core-backed tenant cache so that billing enforcement works without PG.

**Acceptance Criteria:**
- [ ] Rewrite `QueryServiceExWeb.Plugs.UsageEnforcement` to read quotas from `TenantCache.get_quota/1`
- [ ] Hard limit mode: return 402 when `events_used >= events_quota` and overage disabled
- [ ] Soft limit mode: allow through with overage headers when overage enabled
- [ ] Usage increment calls `UsageReporter.increment/2` (async, from US-020) instead of PG update
- [ ] Rate limit headers still populated correctly

### US-023: Remove Ecto, Postgrex, and PostgreSQL from Query Service
As a developer, I want all PostgreSQL dependencies removed from QS so that it is fully stateless.

**Acceptance Criteria:**
- [ ] Remove `ecto`, `ecto_sql`, `postgrex` from `mix.exs` dependencies
- [ ] Delete `lib/query_service_ex/repo.ex`
- [ ] Delete `lib/query_service_ex/tenants/tenant.ex` (Ecto schema)
- [ ] Delete `lib/query_service_ex/tenants.ex` (context module — replaced by TenantCache)
- [ ] Delete `lib/query_service_ex/accounts/user.ex` (Ecto schema)
- [ ] Delete `lib/query_service_ex/accounts.ex` (context module)
- [ ] Delete or archive `priv/repo/migrations/` directory
- [ ] Remove Ecto/PG config from `config.exs`, `dev.exs`, `test.exs`, `runtime.exs`
- [ ] Remove `QueryServiceEx.Repo` from application supervision tree
- [ ] `mix compile --warnings-as-errors` passes
- [ ] `mix test` passes with no PG-dependent tests

### US-024: Remove API key Ecto schema and context from Query Service
As a developer, I want QS API key management removed since it now delegates to Core for key verification.

**Acceptance Criteria:**
- [ ] Delete `lib/query_service_ex/api_keys/api_key.ex` (Ecto schema)
- [ ] Delete `lib/query_service_ex/api_keys.ex` (context module)
- [ ] Delete `lib/query_service_ex_web/controllers/api_key_controller.ex`
- [ ] Remove API key routes from `router.ex` (these move to CP or are handled by Core)
- [ ] Remove related test files
- [ ] No compilation warnings

### US-025: Integration test — QS stateless operation
As a developer, I want to verify QS runs without PostgreSQL in Docker so that the migration is complete.

**Acceptance Criteria:**
- [ ] Update `docker-compose.test.yml` — QS service has NO postgres dependency
- [ ] Test: QS starts successfully without any PG connection configured
- [ ] Test: create event via QS → verify it reaches Core
- [ ] Test: query events via QS → verify results returned from Core
- [ ] Test: QS enforces quota from Core-cached tenant data
- [ ] Test: CP tenant update → QS cache invalidation → QS reflects new data
- [ ] Test: QS survives Core being temporarily unavailable (serves from stale cache)

---

### Phase 5: Cleanup and Documentation

### US-026: Remove deprecated QS billing endpoints and add redirects
As a client developer, I want deprecated QS billing endpoints to return 301 redirects to CP so that existing integrations gracefully transition.

**Acceptance Criteria:**
- [ ] QS billing routes (`/api/billing/*`) return 301 with `Location` header pointing to CP equivalent
- [ ] Redirect URLs use `MGMT_PLANE_URL` env var for CP base URL
- [ ] Log each redirect with warning level for monitoring migration progress
- [ ] Remove all billing business logic from QS (controllers, contexts, modules)
- [ ] Delete `lib/query_service_ex/billing/` directory entirely
- [ ] Delete `lib/query_service_ex_web/controllers/billing_controller.ex`

### US-027: Update Docker Compose for production topology
As a DevOps engineer, I want Docker Compose updated to reflect the new service topology so that deployment matches the architecture.

**Acceptance Criteria:**
- [ ] Update `docker-compose.allsource.yml` — remove PostgreSQL service dependency from QS
- [ ] QS container no longer has `DATABASE_URL` or Ecto env vars
- [ ] CP container has new env vars: `LEMON_SQUEEZY_API_KEY`, `LEMON_SQUEEZY_STORE_ID`, `LEMON_SQUEEZY_WEBHOOK_SECRET`, `QUERY_SERVICE_INTERNAL_URL`
- [ ] QS container has new env vars: `INTERNAL_API_KEY`, `JWT_SECRET`, `MGMT_PLANE_URL`
- [ ] CP container has `DATA_PLANE_URL` env var
- [ ] All services start and pass health checks
- [ ] `docker-compose.test.yml` kept in sync with production compose

### US-028: Update OpenAPI specs for both services
As an API consumer, I want OpenAPI documentation updated to reflect the new endpoint locations and auth methods.

**Acceptance Criteria:**
- [ ] QS OpenAPI spec removes: OAuth endpoints, billing endpoints, API key management endpoints
- [ ] QS OpenAPI spec updates: auth methods to show API key header and Bearer JWT
- [ ] CP OpenAPI spec adds: billing endpoints, webhook endpoint
- [ ] Both specs include HAL `_links` in response schemas
- [ ] Specs validate with `swagger-cli validate` or equivalent

## Functional Requirements

- FR-1: All CP API responses that reference entities must include `_links` object with at minimum a `self` link
- FR-2: All QS API responses that reference entities must include `_links` object with at minimum a `self` link
- FR-3: Cross-service links must use configurable base URLs from environment variables, not hardcoded values
- FR-4: The CP must be able to create LemonSqueezy checkouts and return checkout URLs
- FR-5: The CP must process LemonSqueezy webhooks and update tenant subscription metadata in Core
- FR-6: The QS must validate both API key and JWT authentication methods
- FR-7: The QS must cache tenant data with a 120-second TTL, refreshed from Core on cache miss
- FR-8: The QS must report usage increments to Core asynchronously with retry (max 3 attempts)
- FR-9: The CP must notify QS of tenant state changes via internal webhook to invalidate cache
- FR-10: After Phase 4, QS must start and operate without any PostgreSQL connection
- FR-11: Tiered billing plans must enforce: free (10K events/5K queries), starter (100K/50K), pro (1M/100K), enterprise (unlimited)
- FR-12: QS auth must fall through: API key header → Bearer JWT → dev mode bypass → 401

## Non-Goals (Out of Scope)

- **Core auth API changes** — Core's existing auth endpoints are consumed as-is; no modifications to Core's user/auth system
- **Asymmetric JWT (RS256)** — Using shared HMAC secret for now; asymmetric keys are a future enhancement
- **Custom OAuth providers** — OAuth is being removed, not migrated to a new provider
- **Real-time tenant cache updates via WebSocket** — Using webhook + TTL; WebSocket subscription is future work
- **Multi-region billing** — Single LemonSqueezy account; multi-region is out of scope
- **QS API key management UI** — API key CRUD moves to CP; no new UI in this phase
- **Core HAL links** — Core responses are internal protocol; only QS and CP add cross-service HAL links
- **PostgreSQL migration tooling** — No data migration from QS PG to Core; tenant data already exists in Core via CP

## Technical Considerations

- **Shared JWT_SECRET:** All three services (Core, CP, QS) must use the same `JWT_SECRET` env var value. Secret rotation requires coordinated restart.
- **ETS cache behavior on QS restart:** Cache starts cold; first requests after restart will hit Core. The 120s TTL means worst case a tenant sees stale data for 2 minutes after CP updates.
- **Usage increment durability:** Async fire-and-forget with retry means up to ~15s of usage data could be lost on QS crash (5s flush interval + retries). Acceptable for billing granularity.
- **LemonSqueezy webhook idempotency:** Webhook handler must be idempotent — LS may retry delivery. Use subscription_id + event type as dedup key.
- **Backwards compatibility during migration:** Phase 2 runs both billing stacks in parallel. Phase 5 adds 301 redirects for graceful deprecation.
- **Internal webhook security:** The `POST /internal/tenant-updated` endpoint must not be publicly routable. Docker network isolation + `INTERNAL_API_KEY` header provides defense in depth.
- **Existing CP patterns:** CP uses clean architecture (domain → application → infrastructure → interfaces). New billing code follows existing patterns: entities in `domain/entities/`, use cases in `application/usecases/`, handlers in `interfaces/http/`, clients in `infrastructure/clients/`.
- **Existing QS patterns:** QS uses Phoenix plugs pipeline. New auth plugs follow existing plug patterns. TenantCache follows GenServer + ETS pattern used elsewhere in Elixir ecosystem.

## Success Metrics

- QS starts and passes all tests with zero PostgreSQL dependencies
- All existing API endpoints continue to work with both API key and JWT auth
- Billing operations (checkout, portal, overage) work via CP endpoints
- LemonSqueezy webhooks processed by CP and reflected in tenant metadata within 5 seconds
- Tenant state changes propagated from CP to QS cache within 1 second (via webhook)
- HAL `_links` present in all entity responses across CP and QS
- Docker Compose test suite passes all cross-service integration tests
- No OAuth dependencies remain in QS `mix.exs`

## Open Questions

1. **API key CRUD migration timing** — Should API key management endpoints move to CP (alongside billing) or stay as Core-only? Currently QS has full CRUD; proposal removes from QS but doesn't explicitly add to CP.
2. **Webhook retry behavior** — If CP fails to notify QS of tenant update, should it retry? Or rely on TTL expiration as fallback?
3. **LemonSqueezy sandbox vs production** — Do we need separate env var sets for sandbox testing during development?
4. **Usage counter atomicity** — Core's tenant metadata update for usage counters: is it atomic (increment) or read-modify-write (race condition under load)?