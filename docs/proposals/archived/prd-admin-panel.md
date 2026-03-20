[PRD]
# PRD: Admin Panel & Subscription Management

## Overview

Build an internal admin panel for AllSource Chronos that gives the internal team full visibility and control over tenants, monitoring, billing, and security. The frontend is a separate Next.js app (`apps/admin/`) using the shared `packages/ui` component library. The backend leverages the existing Go Control Plane (`apps/control-plane`), extending it with new admin-specific endpoints where needed. The Control Plane already has tenant CRUD, billing (LemonSqueezy), audit logging, policy/RBAC, and clean architecture with use cases — we build on top of that foundation.

Admin users authenticate via the same OAuth flow as the web app but require an `admin` role claim in their JWT. The admin app is internal-only — not exposed to tenants.

## Goals

- Provide internal team with full tenant lifecycle management (list, view, edit quotas, suspend, archive)
- Replace hardcoded uptime/status numbers with real metrics from Query Service (which aggregates from Core `/metrics`)
- Surface billing data (LemonSqueezy): invoices, revenue tracking, refund processing
- Provide security controls: IP allowlists, API token audit trails, RBAC policy management
- Keep admin app fully isolated (`apps/admin/`) per monorepo rules — no cross-app imports

## Quality Gates

### Epic-Level (run once on epic completion)
General codebase checks that run ONCE when all stories are done:
- `task ci` — full CI pipeline (typecheck, lint, test across all apps)

### Story-Level (checked per story)
- **UI stories:** Playwright e2e test for that story
- **Backend stories:** Verify endpoint returns expected response via curl/test

## User Stories

### US-001: Admin app scaffolding and auth [Integration]
**Description:** As an internal admin, I want a separate Next.js app with OAuth login restricted to admin-role users so that only authorized team members can access the admin panel.

**Acceptance Criteria:**
- [ ] `apps/admin/` exists as a Next.js app with `output: "standalone"`
- [ ] `apps/admin/package.json` depends on `@chronos/ui` (shared component library from `packages/ui`)
- [ ] Tailwind CSS configured, importing shared `packages/ui` preset/theme
- [ ] OAuth login page exists at `/login` using the same provider as `apps/web`
- [ ] JWT validation middleware checks for `role: "admin"` claim — non-admin users see 403
- [ ] Authenticated layout with sidebar navigation (Tenants, Monitoring, Billing, Security)
- [ ] Root `/` redirects to `/tenants` when authenticated
- [ ] `apps/admin/Dockerfile` exists, builds standalone, references only its own source + `packages/ui`
- [ ] Playwright config exists at `apps/admin/playwright.config.ts`
- [ ] E2e test: unauthenticated user is redirected to `/login`

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Control Plane admin auth middleware [Backend]
**Description:** As the admin app, I need the Control Plane to verify admin-role JWTs so that admin API endpoints are protected.

**Acceptance Criteria:**
- [ ] New Gin middleware `AdminAuthMiddleware` in `apps/control-plane/internal/interfaces/http/` that validates JWT and checks `role == "admin"`
- [ ] Returns 401 for missing/invalid token, 403 for non-admin role
- [ ] New route group `/api/v1/admin/*` using this middleware in the Control Plane router
- [ ] Existing tenant-scoped routes are NOT affected
- [ ] Unit test: valid admin JWT passes, non-admin JWT returns 403, missing JWT returns 401
- [ ] Verify via curl: `curl -H "Authorization: Bearer <admin-jwt>" localhost:3901/api/v1/admin/tenants` returns 200

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: List all tenants endpoint [Backend]
**Description:** As the Control Plane, I need an admin endpoint to list all tenants with search/filter/pagination so the admin UI can display them.

**Acceptance Criteria:**
- [ ] `GET /api/v1/admin/tenants` endpoint exists in Control Plane
- [ ] Supports query params: `?search=`, `?plan=`, `?status=`, `?page=`, `?per_page=`
- [ ] Response includes tenant list with: id, name, plan, status, created_at, event_count, member_count
- [ ] Response includes pagination metadata: `total`, `page`, `per_page`, `total_pages`
- [ ] Leverages existing `list_tenants` use case, extended with filtering
- [ ] Unit test: returns filtered results correctly
- [ ] Verify via curl: endpoint returns paginated JSON with HAL links

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Tenant detail and usage endpoint [Backend]
**Description:** As the Control Plane, I need an admin endpoint to view full tenant details including usage breakdown.

**Acceptance Criteria:**
- [ ] `GET /api/v1/admin/tenants/:id` returns full tenant detail (plan, quotas, members, subscription metadata)
- [ ] `GET /api/v1/admin/tenants/:id/usage` returns usage breakdown (events ingested, queries run, storage used, per-day breakdown)
- [ ] Usage data is fetched from Core via Query Service metrics aggregation
- [ ] HAL links include: self, usage, billing, audit, suspend, events
- [ ] Unit test: returns correct detail for existing tenant, 404 for nonexistent
- [ ] Verify via curl: both endpoints return expected JSON structure

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: Tenant quota editing and suspension [Backend]
**Description:** As the Control Plane, I need admin endpoints to edit tenant quotas and suspend/unsuspend tenants.

**Acceptance Criteria:**
- [ ] `PUT /api/v1/admin/tenants/:id/quotas` accepts JSON body with `event_limit`, `query_limit`, `storage_limit_mb`
- [ ] Quota changes are persisted and take effect immediately (Query Service respects updated limits)
- [ ] `POST /api/v1/admin/tenants/:id/suspend` suspends tenant (leverages existing `suspend_tenant` use case)
- [ ] `POST /api/v1/admin/tenants/:id/unsuspend` reactivates tenant
- [ ] Both actions create audit log entries via existing audit infrastructure
- [ ] Unit tests for quota update validation (negative values rejected, etc.)
- [ ] Verify via curl: quota update returns updated tenant, suspend/unsuspend toggles status

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: Bulk tenant operations [Backend]
**Description:** As the Control Plane, I need an admin endpoint for bulk tenant operations (suspend, archive, export).

**Acceptance Criteria:**
- [ ] `POST /api/v1/admin/tenants/bulk` accepts `{"action": "suspend|archive|export", "tenant_ids": [...]}`
- [ ] Suspend: suspends all listed tenants, returns success/failure per tenant
- [ ] Archive: marks tenants as archived (new status), excludes from default listing
- [ ] Export: returns download URL for tenant data export (async job, returns operation ID)
- [ ] Each bulk action creates audit log entries
- [ ] Validation: max 100 tenants per bulk operation
- [ ] Unit test: bulk suspend with mixed valid/invalid IDs returns partial results
- [ ] Verify via curl: bulk suspend returns per-tenant results

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: Tenants list page [UI]
**Description:** As an internal admin, I want to see all tenants in a searchable, filterable table so I can find and manage any tenant quickly.

**Acceptance Criteria:**
- [ ] `/tenants` page renders a data table with columns: Name, Plan, Status, Events, Members, Created
- [ ] Search input filters tenants by name (debounced, calls API)
- [ ] Filter dropdowns for Plan (free, starter, pro, enterprise) and Status (active, suspended, archived)
- [ ] Pagination controls (page size selector: 10/25/50, prev/next)
- [ ] Click on tenant row navigates to `/tenants/[id]`
- [ ] Loading skeleton while data fetches
- [ ] Uses `packages/ui` Table, Input, Select, Button components
- [ ] Playwright e2e test: renders table, search filters results, pagination works

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: Tenant detail page [UI]
**Description:** As an internal admin, I want to view a tenant's full details, usage, and perform actions (edit quotas, suspend) from a single page.

**Acceptance Criteria:**
- [ ] `/tenants/[id]` page shows: tenant info card, usage stats, members list, recent audit log
- [ ] Usage section shows: events ingested (with daily sparkline chart), queries run, storage used, quota % bars
- [ ] "Edit Quotas" button opens modal with form fields for event_limit, query_limit, storage_limit_mb
- [ ] "Suspend/Unsuspend" button with confirmation dialog
- [ ] Actions refresh the page data after completion
- [ ] Breadcrumb navigation: Tenants > [Tenant Name]
- [ ] Uses `packages/ui` Card, Dialog, Chart components
- [ ] Playwright e2e test: detail page loads, edit quotas modal submits, suspend toggles status

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: Metrics aggregation endpoint in Query Service [Backend]
**Description:** As the admin app, I need the Query Service to expose aggregated metrics from Core's Prometheus endpoint so the monitoring dashboard has real data.

**Acceptance Criteria:**
- [ ] New `GET /api/admin/metrics/summary` endpoint in Query Service (behind admin auth)
- [ ] Fetches and parses Core's `/metrics` Prometheus endpoint
- [ ] Returns JSON: `{ uptime_seconds, events_total, events_per_second, query_latency_p99_ms, error_rate_percent, active_tenants }`
- [ ] `GET /api/admin/metrics/timeseries?metric=events_per_second&range=1h|24h|7d` returns time-bucketed data
- [ ] Caches metrics for 15 seconds to avoid hammering Core
- [ ] Unit test: parses sample Prometheus output correctly
- [ ] Verify via curl: endpoint returns valid JSON with real numbers (not hardcoded)

Mark each item [x] as you complete it. Only close when all are checked.

### US-010: Monitoring dashboard page [UI]
**Description:** As an internal admin, I want a real-time monitoring dashboard showing system health, throughput, and error rates so I can spot issues immediately.

**Acceptance Criteria:**
- [ ] `/monitoring` page with stat cards: Uptime, Events/sec, Query Latency p99, Error Rate, Active Tenants
- [ ] Stat cards show real numbers from the metrics API (not hardcoded)
- [ ] Throughput line chart (events/sec over time) with range selector (1h, 24h, 7d)
- [ ] Error rate line chart with same range selector
- [ ] Auto-refresh every 30 seconds with visual indicator
- [ ] Service health section showing Core leader + followers status (from `/api/cluster/members`)
- [ ] Uses `packages/ui` Card and Recharts (already in web app dependencies)
- [ ] Playwright e2e test: dashboard loads with real data, charts render, auto-refresh works

Mark each item [x] as you complete it. Only close when all are checked.

### US-011: Alert rules and SLO configuration [Backend]
**Description:** As the Control Plane, I need endpoints to manage alert rules and SLO definitions so admins can configure monitoring thresholds.

**Acceptance Criteria:**
- [ ] New `AlertRule` entity in Control Plane domain: metric, operator (>, <, ==), threshold, duration, notification_channel
- [ ] `POST /api/v1/admin/alerts` creates alert rule
- [ ] `GET /api/v1/admin/alerts` lists all alert rules
- [ ] `PUT /api/v1/admin/alerts/:id` updates alert rule
- [ ] `DELETE /api/v1/admin/alerts/:id` deletes alert rule
- [ ] New `SLO` entity: metric, target_percent, window (7d, 30d)
- [ ] `GET /api/v1/admin/slos` lists SLOs with current compliance calculated from metrics
- [ ] `POST /api/v1/admin/slos` creates SLO definition
- [ ] Alert rules and SLOs persisted via config repository (existing infrastructure)
- [ ] Unit tests for CRUD operations and SLO compliance calculation
- [ ] Verify via curl: create alert, list alerts, create SLO, get SLO with compliance %

Mark each item [x] as you complete it. Only close when all are checked.

### US-012: Alerts and SLO management page [UI]
**Description:** As an internal admin, I want to configure alert rules and view SLO compliance so I can define and track operational targets.

**Acceptance Criteria:**
- [ ] `/monitoring/alerts` page lists all alert rules in a table
- [ ] "Create Alert" button opens form: metric (dropdown), operator, threshold, duration, channel (email/slack)
- [ ] Edit/Delete actions on each alert row
- [ ] `/monitoring/slos` page shows SLO cards with compliance gauge (green >99%, yellow >95%, red <95%)
- [ ] "Create SLO" button opens form: metric, target %, window
- [ ] SLO cards show current compliance, error budget remaining, trend sparkline
- [ ] Playwright e2e test: create alert rule, verify it appears in list; view SLO compliance

Mark each item [x] as you complete it. Only close when all are checked.

### US-013: Billing administration endpoints [Backend]
**Description:** As the Control Plane, I need admin endpoints for billing data so the admin panel can show invoices, revenue, and handle disputes.

**Acceptance Criteria:**
- [ ] `GET /api/v1/admin/billing/invoices?tenant_id=&status=&page=` lists invoices from LemonSqueezy API
- [ ] `GET /api/v1/admin/billing/revenue?range=30d|90d|1y` returns MRR, ARR, growth rate, churn rate
- [ ] Revenue calculation uses LemonSqueezy subscription data aggregated by period
- [ ] `POST /api/v1/admin/billing/refund` processes refund via LemonSqueezy API for a given invoice
- [ ] `GET /api/v1/admin/billing/dunning` lists tenants with failed payments and retry status
- [ ] All billing admin actions create audit log entries
- [ ] Unit tests for revenue calculation logic
- [ ] Verify via curl: invoices endpoint returns paginated list, revenue endpoint returns MRR/ARR

Mark each item [x] as you complete it. Only close when all are checked.

### US-014: Billing dashboard page [UI]
**Description:** As an internal admin, I want to see revenue metrics, invoice history, and manage billing issues from a dashboard.

**Acceptance Criteria:**
- [ ] `/billing` page with stat cards: MRR, ARR, Growth Rate, Churn Rate
- [ ] Revenue trend chart (MRR over time) with range selector (30d, 90d, 1y)
- [ ] Invoices table with columns: Tenant, Amount, Status (paid/pending/failed), Date, Actions
- [ ] Filter invoices by status and tenant
- [ ] "Refund" action button on paid invoices with confirmation dialog
- [ ] Dunning section: list of tenants with failed payments, retry count, last attempt date
- [ ] Uses `packages/ui` Table, Card, Chart, Dialog components
- [ ] Playwright e2e test: dashboard loads revenue stats, invoices table filters, refund dialog works

Mark each item [x] as you complete it. Only close when all are checked.

### US-015: IP allowlist/blocklist endpoints [Backend]
**Description:** As the Control Plane, I need admin endpoints to manage IP allowlists and blocklists so admins can control access.

**Acceptance Criteria:**
- [ ] New `IPRule` entity: cidr, type (allow|block), description, created_at, created_by
- [ ] `GET /api/v1/admin/security/ip-rules` lists all IP rules
- [ ] `POST /api/v1/admin/security/ip-rules` creates IP rule (validates CIDR format)
- [ ] `DELETE /api/v1/admin/security/ip-rules/:id` removes IP rule
- [ ] IP rules persisted via config repository
- [ ] All changes create audit log entries
- [ ] Unit tests: CIDR validation, create/list/delete
- [ ] Verify via curl: create IP rule, list rules, delete rule

Mark each item [x] as you complete it. Only close when all are checked.

### US-016: API token audit endpoint [Backend]
**Description:** As the Control Plane, I need an admin endpoint to audit API token usage across all tenants.

**Acceptance Criteria:**
- [ ] `GET /api/v1/admin/security/token-audit?tenant_id=&from=&to=&page=` returns token usage logs
- [ ] Each entry shows: tenant_id, key_prefix (first 8 chars), endpoint, timestamp, response_status, ip_address
- [ ] Aggregation view: `GET /api/v1/admin/security/token-audit/summary` returns per-tenant usage counts
- [ ] Data sourced from Query Service audit logs (forwarded to Control Plane)
- [ ] Unit test: pagination and filtering work correctly
- [ ] Verify via curl: token audit returns usage entries, summary returns aggregated counts

Mark each item [x] as you complete it. Only close when all are checked.

### US-017: Security dashboard page [UI]
**Description:** As an internal admin, I want a security dashboard to manage IP rules, audit API token usage, and manage RBAC policies.

**Acceptance Criteria:**
- [ ] `/security` page with three tabs: IP Rules, Token Audit, RBAC Policies
- [ ] IP Rules tab: table of rules (CIDR, Type, Description, Created), "Add Rule" button with CIDR input + type toggle
- [ ] Delete action on each rule with confirmation
- [ ] Token Audit tab: table of recent token usage with tenant filter and date range picker
- [ ] Token Audit summary view: bar chart of API calls per tenant
- [ ] RBAC Policies tab: lists existing policies from Control Plane's policy system (read-only for now, leverages existing `policy_handler.go`)
- [ ] Uses `packages/ui` Tabs, Table, Dialog, Input components
- [ ] Playwright e2e test: add IP rule, verify in list, delete it; token audit table loads and filters

Mark each item [x] as you complete it. Only close when all are checked.

### US-018: Suspicious activity detection endpoint [Backend]
**Description:** As the Control Plane, I need an endpoint that flags suspicious activity patterns so admins can investigate anomalies.

**Acceptance Criteria:**
- [ ] `GET /api/v1/admin/security/suspicious-activity` returns flagged events
- [ ] Detection rules (hardcoded v1): >100 failed auth in 1h, >10 API keys created in 1h, requests from >20 unique IPs for same tenant in 5min
- [ ] Each alert shows: type, tenant_id, description, severity (low/medium/high), timestamp, details
- [ ] Alerts are computed on-demand from audit log data (no background job in v1)
- [ ] Unit test: generates alert when threshold exceeded in test data
- [ ] Verify via curl: endpoint returns list of suspicious activity alerts (empty if none)

Mark each item [x] as you complete it. Only close when all are checked.

### US-019: Suspicious activity UI and notifications [UI]
**Description:** As an internal admin, I want to see suspicious activity alerts prominently so I can investigate security issues quickly.

**Acceptance Criteria:**
- [ ] `/security/alerts` page shows suspicious activity alerts in severity-colored cards
- [ ] Each card shows: type icon, tenant name (linked to tenant detail), description, timestamp
- [ ] Filter by severity (low/medium/high) and type
- [ ] Bell icon in sidebar navigation shows count badge of high-severity alerts
- [ ] Alert count refreshes every 60 seconds
- [ ] Clicking alert card navigates to relevant tenant detail or token audit (contextual)
- [ ] Playwright e2e test: alerts page loads, severity filter works, badge count displays

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements

- FR-1: The admin app must be a standalone Next.js deployment at `apps/admin/`, isolated per monorepo rules
- FR-2: Authentication must use the same OAuth provider as `apps/web` but require an `admin` role claim
- FR-3: All admin API endpoints must live under `/api/v1/admin/*` in the Control Plane (port 3901)
- FR-4: Metrics displayed in the monitoring dashboard must come from real Core/Query Service data, never hardcoded
- FR-5: The Query Service must expose an admin metrics aggregation endpoint that parses Core's Prometheus output
- FR-6: All admin actions (quota changes, suspensions, IP rule changes, refunds) must create audit log entries
- FR-7: Billing data must be sourced from the LemonSqueezy API via the existing Control Plane billing infrastructure
- FR-8: IP rules must validate CIDR format before persisting
- FR-9: The admin panel must use `packages/ui` shared components — no duplicated component code
- FR-10: Bulk operations must be limited to 100 tenants per request

## Non-Goals (Out of Scope)

- SSO configuration UI (future — requires IdP integration work beyond admin panel)
- Fine-grained RBAC policy editor (v1 shows policies read-only; editing is a separate effort)
- Real-time WebSocket streaming in admin dashboard (polling every 30s is sufficient for v1)
- Multi-region admin views (single-cluster only for now)
- Custom alert notification channels beyond email/Slack
- Background alert evaluation jobs (v1 computes on-demand)
- Dunning automation (v1 shows status only, manual retry)
- Tenant data export implementation (v1 returns operation ID, actual export pipeline is separate)

## Technical Considerations

- **Control Plane** (`apps/control-plane`) is the backend — Go, Gin, clean architecture with use cases/entities/repositories
- **Existing infrastructure to leverage:** `list_tenants`, `suspend_tenant`, `create_tenant`, `update_tenant`, `delete_tenant` use cases; `billing_handler` with LemonSqueezy; `audit_handler` and `core_audit_repository`; `policy_handler` for RBAC
- **Query Service metrics endpoint** parses Core's Prometheus `/metrics` output — needs a new controller and caching layer
- **Admin app** uses Next.js App Router, `packages/ui` components, Recharts for charts (already used in `apps/web`)
- **Docker:** `apps/admin/Dockerfile` must only COPY `apps/admin/` and `packages/ui/` — no cross-app contamination
- **Ports:** Admin app on a new port (suggest 3905), Control Plane stays on 3901
- **Auth:** JWT with `role` claim — Control Plane validates via `AdminAuthMiddleware`, admin app validates client-side + server-side

## Success Metrics

- Internal team can list, search, and manage all tenants without direct database access
- Monitoring dashboard shows real uptime, throughput, and error rates (zero hardcoded values)
- Billing dashboard shows accurate MRR/ARR from LemonSqueezy data
- Security alerts surface anomalies within 60 seconds of page load
- All admin actions are fully auditable via audit log

## Open Questions

- Should admin app port be 3905 or something else? (Need to check for conflicts in docker-compose)
- What email/Slack integration should alerts use for notifications? (v1 may just show in-app)
- Should tenant data export produce a downloadable archive or push to S3/GCS?
- Do we need a separate admin OAuth app registration, or can we reuse the same one with role-based access?
[/PRD]
