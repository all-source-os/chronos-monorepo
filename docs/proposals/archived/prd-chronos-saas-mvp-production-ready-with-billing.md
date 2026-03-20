# PRD: AllSource SaaS MVP - Production Ready with Billing

## Overview
Transform the AllSource event sourcing platform into a production-ready SaaS offering. This PRD focuses on making the Query Service (Elixir) production-ready first, then adding billing via LemonSqueezy with hybrid pricing (base subscription + usage overage), and OAuth-only authentication. The deployment target is Fly.io with a design-for-scale architecture starting small.

## Goals
- Deploy Query Service to Fly.io with production-grade configuration
- Implement OAuth authentication (Google, GitHub) - no passwords
- Integrate LemonSqueezy for billing with hybrid pricing model
- Add usage metering for events ingested/queried
- Create customer onboarding flow
- Ensure all services pass quality gates before deployment

## Quality Gates

These commands must pass for each user story based on the service being modified:

**Query Service (Elixir):**
```bash
cd apps/query-service && mix deps.get && mix format --check-formatted && mix compile --warnings-as-errors && mix test
```

**Control Plane (Go):**
```bash
cd apps/control-plane && go mod download && gofmt -l . && go vet ./... && go test -v -race ./...
```

**Core (Rust):**
```bash
cd apps/core && cargo fmt --check && cargo clippy --locked --all-targets --all-features -- -D warnings && cargo test --locked --lib --all-features
```

**Web App (TypeScript):**
```bash
cd apps/web && bun install && bun run typecheck && bun run lint
```

## User Stories

### US-001: Configure Query Service for Fly.io Deployment
As a platform operator, I want the Query Service deployed on Fly.io so that it can serve production traffic reliably.

**Acceptance Criteria:**
- [ ] Create `fly.toml` configuration in `apps/query-service/`
- [ ] Configure health check endpoint at `/health`
- [ ] Set up environment variables for production (DATABASE_URL, SECRET_KEY_BASE)
- [ ] Configure auto-scaling rules (min 1, max 10 instances)
- [ ] Add Fly.io secrets management for sensitive config
- [ ] Document deployment process in README

### US-002: Add Production Error Handling to Query Service
As a platform operator, I want robust error handling so that transient failures don't crash the service.

**Acceptance Criteria:**
- [ ] Implement retry logic with exponential backoff for database connections
- [ ] Add circuit breaker pattern for external service calls
- [ ] Replace `fatal` exits with graceful degradation where possible
- [ ] Add structured JSON logging with request correlation IDs
- [ ] Implement graceful shutdown handling for Fly.io deploys

### US-003: Implement OAuth Authentication (Google)
As a user, I want to sign in with my Google account so that I don't need to manage another password.

**Acceptance Criteria:**
- [ ] Add `ueberauth` and `ueberauth_google` dependencies
- [ ] Create OAuth callback controller at `/auth/google/callback`
- [ ] Store user profile (email, name, avatar) on first login
- [ ] Create or link tenant on first OAuth login
- [ ] Generate JWT token after successful OAuth
- [ ] Handle OAuth errors gracefully with user-friendly messages

### US-004: Implement OAuth Authentication (GitHub)
As a developer, I want to sign in with my GitHub account so that I can quickly onboard.

**Acceptance Criteria:**
- [ ] Add `ueberauth_github` dependency
- [ ] Create OAuth callback controller at `/auth/github/callback`
- [ ] Extract GitHub username and email for profile
- [ ] Link to existing account if email matches
- [ ] Generate JWT token after successful OAuth

### US-005: Create LemonSqueezy Billing Integration
As a platform operator, I want to integrate LemonSqueezy so that I can charge customers.

**Acceptance Criteria:**
- [ ] Create `AllSource.Billing` context module
- [ ] Add LemonSqueezy API client for subscription management
- [ ] Create webhook endpoint at `/webhooks/lemonsqueezy`
- [ ] Handle `subscription_created`, `subscription_updated`, `subscription_cancelled` events
- [ ] Store subscription status in `tenants` table
- [ ] Implement signature verification for webhooks

### US-006: Implement Hybrid Pricing Model
As a platform operator, I want to charge a base subscription plus usage overage so that pricing scales with customer usage.

**Acceptance Criteria:**
- [ ] Create pricing tiers: Free (1K events/mo), Pro ($29/mo, 100K events), Enterprise (custom)
- [ ] Track events ingested per tenant per billing period
- [ ] Track events queried per tenant per billing period
- [ ] Calculate overage charges ($0.001 per event over limit)
- [ ] Create usage summary endpoint for customers
- [ ] Integrate usage data with LemonSqueezy metered billing

### US-007: Create Usage Metering System
As a platform operator, I want to meter tenant usage so that I can bill accurately.

**Acceptance Criteria:**
- [ ] Add `tenant_usage` table with period-based aggregation
- [ ] Increment counters on event ingestion (async, non-blocking)
- [ ] Increment counters on query execution (async, non-blocking)
- [ ] Create daily rollup job for usage aggregation
- [ ] Expose usage API endpoint `/api/v1/tenants/:id/usage`
- [ ] Add usage dashboard data to tenant context

### US-008: Implement Subscription Enforcement
As a platform operator, I want to enforce subscription limits so that free users don't exceed quotas.

**Acceptance Criteria:**
- [ ] Check subscription status on API requests
- [ ] Return 402 Payment Required when quota exceeded
- [ ] Allow 10% grace period before hard cutoff
- [ ] Send warning at 80% and 100% usage via webhook
- [ ] Cache subscription status for performance (1 min TTL)

### US-009: Create Customer Onboarding Flow
As a new user, I want a guided onboarding experience so that I can start using AllSource quickly.

**Acceptance Criteria:**
- [ ] Create onboarding API endpoints for step tracking
- [ ] Auto-create first stream on tenant creation
- [ ] Generate initial API key during onboarding
- [ ] Return onboarding checklist status in tenant response
- [ ] Mark onboarding complete after first event ingested

### US-010: Add Structured Logging and Observability
As a platform operator, I want structured logs so that I can debug production issues.

**Acceptance Criteria:**
- [ ] Configure Logger to output JSON in production
- [ ] Add request_id to all log entries
- [ ] Add tenant_id to all log entries where available
- [ ] Log all API requests with method, path, status, duration
- [ ] Integrate with Fly.io log aggregation

### US-011: Create Fly.io Health Monitoring
As a platform operator, I want health checks so that Fly.io can manage instance lifecycle.

**Acceptance Criteria:**
- [ ] Implement `/health` endpoint returning 200 when healthy
- [ ] Implement `/ready` endpoint checking database connectivity
- [ ] Add memory usage to health response
- [ ] Add active connection count to health response
- [ ] Configure Fly.io to use health checks for routing

### US-012: Implement Tenant Isolation Security Review
As a platform operator, I want verified tenant isolation so that customer data is secure.

**Acceptance Criteria:**
- [ ] Audit all queries for tenant_id filtering
- [ ] Add database-level row security policies
- [ ] Create integration tests verifying cross-tenant isolation
- [ ] Document tenant isolation architecture
- [ ] Add tenant_id validation middleware

### US-013: Create API Rate Limiting
As a platform operator, I want rate limiting so that no single tenant can overwhelm the system.

**Acceptance Criteria:**
- [ ] Implement token bucket rate limiting per tenant
- [ ] Configure limits by subscription tier (Free: 10 req/s, Pro: 100 req/s)
- [ ] Return 429 Too Many Requests with Retry-After header
- [ ] Add rate limit headers to all responses (X-RateLimit-*)
- [ ] Store rate limit state in ETS for performance

### US-014: Set Up Production Database on Fly.io
As a platform operator, I want a production database so that data persists reliably.

**Acceptance Criteria:**
- [ ] Create Fly Postgres cluster with 2 replicas
- [ ] Configure connection pooling (PgBouncer)
- [ ] Set up automated daily backups
- [ ] Configure SSL for all database connections
- [ ] Document disaster recovery procedures

### US-015: Create Customer Self-Service API Key Management
As a customer, I want to manage my API keys so that I can rotate them without support.

**Acceptance Criteria:**
- [ ] Add `/api/v1/api-keys` endpoint for CRUD operations
- [ ] Support multiple API keys per tenant (max 5)
- [ ] Add key expiration date support
- [ ] Log all API key operations for audit
- [ ] Return masked key on list (show last 4 chars only)

## Functional Requirements

- FR-1: The system must authenticate users exclusively via OAuth (Google, GitHub)
- FR-2: The system must create a tenant and default workspace on first OAuth login
- FR-3: The system must enforce subscription limits based on LemonSqueezy subscription status
- FR-4: The system must meter all events ingested and queried per tenant
- FR-5: The system must report usage to LemonSqueezy for metered billing
- FR-6: The system must return 402 Payment Required when subscription is inactive or quota exceeded
- FR-7: The system must isolate tenant data at the application and database level
- FR-8: The system must rate limit API requests per tenant based on subscription tier
- FR-9: The system must emit structured JSON logs with request and tenant correlation
- FR-10: The system must handle Fly.io deployment lifecycle (health checks, graceful shutdown)

## Non-Goals

- Custom SSO/SAML integration (Enterprise feature, post-MVP)
- Admin dashboard UI (API-first for MVP, UI in v2)
- Multi-region deployment (single region for MVP)
- Real-time usage dashboards (async batch reporting sufficient)
- Custom branding per tenant
- Data export/import tools
- SLA commitments or uptime guarantees

## Technical Considerations

- **Fly.io Specifics:** Use Fly Postgres, configure `fly.toml` for Phoenix apps, leverage Fly's built-in metrics
- **LemonSqueezy:** Use REST API, implement webhook handlers with signature verification
- **OAuth:** Use `ueberauth` library with Google and GitHub strategies
- **Rate Limiting:** Use ETS-based token bucket for sub-millisecond checks
- **Usage Metering:** Async GenServer to avoid blocking request path
- **Database:** Add indexes on `tenant_id` for all tenant-scoped tables

## Success Metrics

- Query Service deployed and serving traffic on Fly.io
- OAuth login working for Google and GitHub
- LemonSqueezy webhooks processing successfully
- Usage metering accurate within 1% of actual events
- API rate limiting enforced per tier
- Zero cross-tenant data leakage (verified by tests)
- 99% uptime during business hours (tracked via Fly metrics)

## Open Questions

1. Should we support GitHub OAuth with private email addresses (requires additional scope)?
2. What is the grace period before hard cutoff on quota exceeded - 24 hours or 7 days?
3. Should free tier require credit card on file?
4. Do we need GDPR data export functionality for MVP?