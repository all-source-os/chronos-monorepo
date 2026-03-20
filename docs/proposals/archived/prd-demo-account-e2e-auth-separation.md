# PRD: Demo Account & E2E Auth Separation

## Overview
Separate authentication e2e testing from production demo functionality. Add an `is_demo` flag to tenants so demo accounts get full platform access without billing. Auth e2e tests run against a staging deployment with real OAuth — eliminating dependency on `AUTH_DISABLED=true` and the `dev-token` endpoint. Demo tenant data is pre-seeded and resets daily.

## Goals
- Demo accounts have full feature access but are excluded from billing enforcement
- Auth e2e tests authenticate via real OAuth against the staging environment (no `dev-token`, no `AUTH_DISABLED`)
- Demo tenant data is pre-seeded on first login and resets on a daily schedule
- `dev-token` endpoint and `AUTH_DISABLED` flag are compile-time guarded so they cannot exist in production builds
- Clear separation between test infrastructure and production demo flow

## Quality Gates

### Epic-Level (run once on epic completion)
General codebase checks that run ONCE when all stories are done:
- `make ci` — full CI pipeline (Rust, Go, Elixir, e2e)
- `cd tooling/e2e && bunx playwright test` — all e2e specs pass

### Story-Level (checked per story)
- **Backend stories:** curl/test the specific endpoint and verify response
- **E2e test stories:** run the relevant Playwright spec file

## User Stories

### US-001: Add `is_demo` field to tenant metadata in Core [Backend]
As a platform operator, I want tenants to have an `is_demo` flag so that the system can identify demo accounts and skip billing.

**Acceptance Criteria:**
- [ ] Core accepts `is_demo` (boolean, default `false`) in tenant metadata when creating/updating tenants via `POST /api/v1/tenants` and `PUT /api/v1/tenants/{id}`
- [ ] `GET /api/v1/tenants/{id}` returns `is_demo` in the metadata object
- [ ] Existing tenants without `is_demo` default to `false`
- [ ] `cargo test` passes in `apps/core/`

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Skip billing enforcement for demo tenants in Query Service [Backend]
As a demo user, I want to use the platform without hitting quota limits or billing gates so that I can explore all features freely.

**Acceptance Criteria:**
- [ ] `UsageEnforcement` plug in `apps/query-service/lib/query_service_ex_web/plugs/usage_enforcement.ex` checks `is_demo` on the tenant
- [ ] When `is_demo == true`, the plug skips quota enforcement entirely (no 402 responses)
- [ ] Usage is still tracked (events_used/queries_used increment) for analytics — just not enforced
- [ ] `mix test` passes in `apps/query-service/`
- [ ] Verify: `curl -H "Authorization: Bearer <demo-token>" <staging>/api/v1/events` returns 200 even when quotas would be exceeded

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: Demo tenant creation via Control Plane OAuth flow [Backend]
As a prospect, I want to log in with Google/GitHub OAuth and land in a demo tenant so that I can try the platform immediately.

**Acceptance Criteria:**
- [ ] A route or parameter (e.g., `/demo/start` or query param `?demo=true` on the OAuth callback) signals the Control Plane to provision/assign the user to the demo tenant
- [ ] If the demo tenant doesn't exist yet, it is created with `is_demo: true`, `subscription_status: "active"`, `subscription_tier: "enterprise"`, unlimited quotas (`events_quota: -1`, `queries_quota: -1`)
- [ ] If the demo tenant already exists, the user is added as a member
- [ ] The returned JWT contains the demo tenant's `tenant_id`
- [ ] Verify: complete the OAuth flow via the demo entry point and confirm `/api/auth/me` returns the demo tenant

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Pre-seed demo tenant data on first login [Backend]
As a demo user, I want sample data already loaded when I first log in so that I can immediately explore the platform's capabilities.

**Acceptance Criteria:**
- [ ] After demo tenant assignment (US-003), check if the tenant has events; if empty, call Core's `POST /api/v1/demo/seed` scoped to the demo tenant
- [ ] Seeded data includes sample events across 3-4 streams (reuse existing `seed_demo` logic in Core)
- [ ] Seeding is idempotent — calling it when data already exists is a no-op or skips
- [ ] Verify: log in as a new demo user, confirm `/api/v1/events/query` returns pre-seeded events

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: Daily demo data reset [Backend]
As a platform operator, I want the demo tenant's data to reset daily so that it stays clean for new prospects.

**Acceptance Criteria:**
- [ ] A scheduled task (Elixir `GenServer` with `:timer` or Quantum job in Query Service) runs once per day
- [ ] The task deletes all events for the demo tenant via Core API (e.g., `DELETE /api/v1/tenants/{demo_tenant_id}/events` or equivalent)
- [ ] After deletion, re-seeds the demo tenant with fresh sample data via `POST /api/v1/demo/seed`
- [ ] The reset time is configurable via env var `DEMO_RESET_HOUR` (default: `04` for 4 AM UTC)
- [ ] `mix test` passes in `apps/query-service/`

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: Guard `dev-token` endpoint and `AUTH_DISABLED` from production [Backend]
As a security engineer, I want the `dev-token` endpoint and `AUTH_DISABLED` bypass to be impossible to enable in production so that they can never be exploited.

**Acceptance Criteria:**
- [ ] Add a compile-time or startup guard: if `MIX_ENV=prod` (or `RELEASE_MODE=true`), the `dev-token` route is not registered in the router and `AUTH_DISABLED` env var is ignored
- [ ] In `apps/query-service/lib/query_service_ex_web/router.ex`, the dev-token route is wrapped in a `if Mix.env() != :prod` guard or equivalent runtime check
- [ ] `apps/query-service/lib/query_service_ex/dev_mode.ex` — `auth_disabled?/0` always returns `false` in prod regardless of env var
- [ ] Verify: start Query Service in prod mode, confirm `GET /api/auth/dev-token` returns 404
- [ ] `mix test` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: Demo entry point on the web app [UI]
As a prospect, I want a clear "Try Demo" button on the landing/login page that starts the demo OAuth flow so that I can try the platform without a full signup.

**Acceptance Criteria:**
- [ ] Add a "Try Demo" button/link on `apps/web/src/app/(auth)/login/page.tsx`
- [ ] Button navigates to the demo OAuth entry point (from US-003) — e.g., `{CONTROL_PLANE_URL}/api/v1/auth/oauth/google?demo=true`
- [ ] After OAuth callback, user lands on `/dashboard` with demo tenant context
- [ ] The demo tenant dashboard shows a persistent banner: "You're using a demo account — data resets daily"
- [ ] Verify: click "Try Demo", complete OAuth, confirm dashboard loads with demo banner and pre-seeded data

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: Auth e2e tests against staging with real OAuth [Integration]
As a developer, I want auth e2e tests that use real OAuth against the staging environment so that tests validate the actual production auth flow.

**Acceptance Criteria:**
- [ ] Create `tooling/e2e/tests/smoke/auth-staging.spec.ts` (or rename/replace existing `auth.spec.ts`)
- [ ] Tests use Playwright's browser automation to complete the real OAuth flow (Google or GitHub) using a test account
- [ ] Test account credentials sourced from env vars: `E2E_OAUTH_EMAIL`, `E2E_OAUTH_PASSWORD` (never hardcoded)
- [ ] Test scenarios: login via OAuth → verify session → verify dashboard loads → logout → verify session cleared
- [ ] `playwright.config.ts` updated: staging base URL from `STAGING_URL` env var (falls back to `http://localhost:3000`)
- [ ] Old `auth.spec.ts` that depends on `dev-token` is removed or moved to a `local-only/` directory
- [ ] Run: `cd tooling/e2e && bunx playwright test tests/smoke/auth-staging.spec.ts` passes against staging

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: Demo zone e2e tests use demo account flow [Integration]
As a developer, I want demo zone e2e tests to authenticate via the demo OAuth flow so that tests reflect the real user experience.

**Acceptance Criteria:**
- [ ] Update `tooling/e2e/tests/smoke/demo-zone.spec.ts` to authenticate via the demo OAuth flow (US-003/US-007) instead of dev-token
- [ ] Test scenarios: start demo → OAuth login → verify pre-seeded data visible → interact with Live Fire view → interact with MCP Showdown view
- [ ] Tests are tagged/grouped so they can run independently: `bunx playwright test --grep @demo`
- [ ] Run: `cd tooling/e2e && bunx playwright test tests/smoke/demo-zone.spec.ts` passes against staging

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements
- FR-1: Tenants with `is_demo: true` in metadata bypass billing enforcement but still track usage
- FR-2: The demo OAuth entry point provisions users into the shared demo tenant with enterprise-tier access
- FR-3: Demo tenant data is pre-seeded on first login and resets daily at a configurable hour
- FR-4: The `dev-token` endpoint and `AUTH_DISABLED` flag are unreachable in production builds
- FR-5: Auth e2e tests authenticate via real OAuth using test account credentials from env vars
- FR-6: Demo zone e2e tests use the demo OAuth flow, not dev-token

## Non-Goals (Out of Scope)
- Custom demo data per prospect (all demo users share one tenant with the same sample data)
- Per-user demo tenants (single shared demo tenant for now)
- Demo account time limits or auto-expiry
- Demo-specific analytics dashboard for sales team
- Migrating demo user data to a paid tenant on conversion
- Multi-provider OAuth testing (pick one provider for e2e — Google or GitHub, not both)

## Technical Considerations
- **Tenant metadata is stored in Core**, not PostgreSQL. The `is_demo` flag goes in Core's tenant metadata structure (same as `subscription`, `quotas`).
- **TenantCache (ETS)** in Query Service caches tenant data with 5-min TTL. After setting `is_demo`, the cache must be invalidated (Control Plane webhook `POST /internal/tenant-updated` already handles this).
- **OAuth test account**: Use a dedicated Google/GitHub test account. Store credentials in CI secrets, never in code. Consider Google's "test user" concept for OAuth apps in testing mode.
- **Daily reset scheduler**: Elixir's `Process.send_after` or a library like Quantum. Keep it simple — a GenServer with a daily timer is sufficient.
- **Core event deletion**: Need to verify Core supports deleting events by tenant. If not, US-005 may need a Core-side endpoint addition.

## Success Metrics
- `dev-token` endpoint returns 404 in production deployment
- Demo users can log in, see pre-seeded data, and use all features without billing errors
- Auth e2e tests pass in CI against staging using real OAuth (no `AUTH_DISABLED`)
- Demo data resets daily without manual intervention
- Zero security incidents from dev-mode endpoints in production

## Open Questions
- Does Core currently support deleting all events for a specific tenant? If not, US-005 needs a Core endpoint addition.
- Which OAuth provider (Google or GitHub) should the e2e test account use? Recommendation: GitHub — easier to create test accounts without phone verification.
- Should the demo tenant be auto-created on first deployment, or manually provisioned once?
- What sample data streams should be seeded? (Reuse existing `seed_demo` logic or define new streams?)