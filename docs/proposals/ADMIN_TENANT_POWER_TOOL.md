# Admin Tenant Power Tool — manage *and* actively help every tenant

**Status:** DESIGN proposal, 2026-06-24. Extends the **shipped** fleet-health/recovery foundation (`docs/proposals/CONTROL_PLANE_TENANT_HEALTH_RECOVERY.md`, P0–P3 on `main`). No code yet — this is the design + phased-build plan that follow-on `/create-prompt` invocations consume.
**Author:** design pass.
**Scope:** turn `apps/admin` into a power tool for the single system owner to (1) understand any tenant in seconds, (2) run everyday lifecycle/quota/billing ops safely, (3) proactively reach out to tenants who need help, and (4) view the product read-only as a tenant for support — all audited, all on the existing Control-Plane API. Closes **four broken gaps** and builds **four capability pillars + guarded view-as** on top.

---

## 1. Vision & framing

The operator already has a fleet-health rollup and a guarded recovery API (shipped this cycle). What's missing is the **everyday tenant-operations cockpit**: the place you land when a customer emails "my dashboard is empty" or "I want to upgrade" and you need to *see their whole world, act on it safely, and follow up* — without leaving one console and without re-deriving anything from a runbook.

This proposal does **not** redesign the foundation. It composes the shipped pieces (`/api/v1/admin/fleet/health/:id`, `/api/v1/admin/recovery/*`, the BFF, the admin-role gate) into a tenant-centric power tool and adds only what is genuinely missing.

### What already ships (verified — do NOT rebuild)

| Capability | Where | Verified file |
|---|---|---|
| Admin app shell, auth chain, BFF | `apps/admin` (Vercel `allsource-admin`, `admin.all-source.xyz`) | `apps/admin/src/proxy.ts`, `apps/admin/src/app/api/v1/[...path]/route.ts`, `apps/admin/src/lib/auth.ts` |
| Pages: `/tenants`, `/tenants/[id]`, `/fleet`, `/fleet/[id]`, `/monitoring`, `/billing`, `/security`, `/security/alerts` | `apps/admin/src/app/(authenticated)/*` | (directory listing) |
| Admin-role gate (one allowlist, at mint) | `roleForEmail()` → `RoleAdmin`; `AdminAuthMiddleware` enforces `role=="admin"` | `apps/control-plane/auth.go:50`, `admin_middleware.go:69` |
| Per-tenant **health assessment** (all signals + values) | `GET /api/v1/admin/fleet/health/:id` | `fleet_health_handler.go:72`, `fleet_health.go` |
| Fleet rollup (4 tier counts + worst-N) | `GET /api/v1/admin/fleet/health` | `fleet_health_handler.go:44` |
| **Recovery actions, fully guarded** — resync, reconcile-subscription, resolve-dunning (Guarded); rotate-keys, reprovision, restore (Destructive); batch | `POST /api/v1/admin/recovery/*` | `recovery_handler.go`, `recovery.go`, `recovery_guarded.go`, `recovery_destructive.go`, `recovery_batch.go` |
| Diagnose edition-trap + identity divergence (read-only) | `GET …/recovery/diagnose/edition`, `…/recovery/:id/diagnose-identity` | `fleet_health_handler.go:98,109`, `recovery_diagnose.go` |
| Recovery audit → durable Core event | `admin.recovery.*` events under the `admin-recovery` system tenant | `recovery_audit.go:49` |
| Billing admin — revenue (MRR/ARR/churn), invoices, refund, dunning | `GET/POST /api/v1/admin/billing/*` | `admin_billing_handler.go`, `billing/admin_{revenue,list_invoices,refund,dunning}.go` |
| Tenant CRUD — list/detail/usage/quotas/suspend/unsuspend/bulk | `/api/v1/admin/tenants*` | `admin_tenant_handler.go` |
| Real per-tenant **usage metering** in Core | `POST /api/v1/tenants/{id}/usage/increment` (events/queries meters), `GET /api/v1/tenants/{id}/stats` | `apps/core/src/infrastructure/web/api_v1.rs:205,208`, `tenant_api.rs` |
| Email **inbound** webhook + hosted-OAuth inbox connect (Nylas v3) | `POST/GET /api/v1/webhooks/email`, `/api/v1/admin/inbox/connect` | `email_webhook_handler.go`, `inbox_connect_handler.go`, `main.go:604-609` |
| Outbound **SMTP** client (used for billing-warning emails) | `smtpEmailClient.SendEmail` | `internal/infrastructure/clients/email_client.go`, `billing/check_usage_warnings.go` |
| Email-event JSON-Schema contract | `email.{received,sent,triaged,replied,archived,drafted}` | `docs/contracts/email-events/README.md` |
| Resilience primitives: `asList()`, error boundary, edge-safe JWT decode | — | `apps/admin/src/lib/{security,billing}-api.ts`, `app/(authenticated)/error.tsx`, `lib/auth.ts:37` |

### What is net-new in this proposal

- **Gap fixes** (4): kill the demo-litter probe + reap demo tenants + prevent recurrence; wire `/monitoring`; fix per-tenant counts (a field-name bug *and* a stale-source bug); reconcile the plan/tier catalog.
- **Pillar A** — a per-tenant **360** that composes existing reads (no new heavy endpoint; one optional convenience aggregate).
- **Pillar B** — surface the *already-built* recovery actions as **first-class per-tenant operations** in the tenant drill-down (mostly UI + client glue; no new server actions except provision/reactivate aliases).
- **Pillar C** — proactive **comms**: a genuinely-new `/api/v1/admin/notices` + `…/messages` surface (in-app notice events + email send) built on the existing SMTP client + Core event-sourcing + (optionally) the Nylas provider. In-app notices do **not** exist today.
- **Pillar D** — billing/revenue ops UI on the existing endpoints + the catalog reconciliation + the `/monitoring` feed (gap #2 + gap #4).
- **Guarded view-as** — a new read-only, short-TTL, audited impersonation token minted off the existing delegation primitive, plus a banner + teardown.

---

## 2. Architecture invariants this design honors

Non-negotiable per `CLAUDE.md` / `MEMORY.md`. Stated so the build phase cannot regress.

- **Core IS the database.** Per-tenant usage comes from Core metering (`POST …/usage/increment`, `GET …/stats`) — never a new DB. "Tenant reports empty data" is a read-path/identity symptom, never data loss (the central data-visibility lesson).
- **No PostgreSQL for events.** The Control Plane runs with no Postgres; tenants/subscriptions/quotas live as Core tenant-metadata JSON. New comms/notice/view-as state is **Core events** (`admin.notice.*`, `admin.message.*`, `admin.viewas.*`), audited like `admin.recovery.*`.
- **The admin is Bearer-only-via-BFF.** Client pages call **same-origin** `/api/v1/...`; the BFF (`apps/admin/src/app/api/v1/[...path]/route.ts`) reads the httpOnly `admin_token` cookie and attaches `Authorization: Bearer`. The CP ignores cookies (`admin_middleware.go extractBearerToken`). **Never** propose a direct cross-origin `credentials: "include"` call to the CP — that 401s (a real bug this cycle, `CONTROL_PLANE_CORS.md` §4 symptom 6).
- **Admin/system MCP (if any) goes in `apps/mcp-server-elixir`, never `apps/prime-mcp`** (single-tenant by design). This proposal adds no required MCP work; an optional MCP follow-on reuses the shipped `recovery_*` machinery.
- **Reuse the recovery safety model — don't fork it.** Every mutating/comms/view-as action: dry-run where applicable, server-enforced confirmation for destructive ops, Core audit event before reporting success. The shipped `recoveryGuard` (`recovery.go:79`) is the canonical confirmation/blast-radius mechanism; new mutations reuse the same patterns.
- **Deployment topology.** `apps/admin` → Vercel (prebuilt). `apps/control-plane` + `apps/query-service` → Fly. There is no `allsource-web` Fly app and `apps/web` has no admin route group — admin features stay in `apps/admin`.

---

## 3. Gap closure plan

Four gaps that are broken in production *today*. Each: grounded root cause → fix → files/endpoints → acceptance.

### Gap 1 — Demo-user litter (pollutes `/tenants` every status poll) — **highest priority**

**Root cause (grounded).** `apps/web/src/app/api/status/services/route.ts` `getMonitorToken()` (line 48) mints a status-probe session by calling `POST ${cpUrl}/api/v1/demo/start` (line 53). That endpoint is `DemoStartHandler` (`apps/control-plane/onboard.go:109`), which **creates a tenant every call** — registers `demo-<slug>@demo.allsource.dev` (line 112), creates a Core tenant with `name: "Demo User"` (line 114) and `is_demo: true` (line 150), and seeds ~1000 events (`/api/v1/demo/seed`, line 199). The token is cached in a module variable `monitorToken` (line 46) that **does not survive Vercel cold starts / new lambda instances** — and the `/status` page polls this route continuously. Result: roughly one "Demo User" free tenant per cold-start poll → dozens/day in `/tenants`. The probe is a textbook **side-effectful liveness probe**: a health check that mutates production.

**Fix — three parts (all required):**

1. **Stop the side-effectful probe.** Replace `getMonitorToken()` → `DemoStartHandler` with a non-mutating liveness check. The login probe in this same file (`probeLoginAuth`, line 88) exists to prove "a real session round-trips against QS `/api/v1/auth/me`". Two non-mutating options, in order of preference:
   - **(a) Persistent service token.** Provision **one** long-lived service identity (a real API key via `SignAPIKey`, role `serviceaccount`, 365-day TTL — `auth.go:140`) for a dedicated `status-monitor` tenant, store it as a Vercel env `STATUS_MONITOR_TOKEN`, and have `probeLoginAuth` use it directly. No mint call, no tenant creation, ever. This keeps the *end-to-end* "a real token validates" semantics the probe was written for.
   - **(b) Liveness-only probe.** If the round-trip semantics are dispensable, drop the authed probe entirely and assert liveness via the unauthenticated health endpoints (QS `GET /health` — `apps/query-service/lib/query_service_ex/telemetry.ex:147` treats it as a health-check path; CP `GET /health`). This loses the "real session validates" signal but is the simplest no-side-effect option. Recommend (a) so we keep the auth-path coverage that caught the silent-logout incident.
2. **Reap the existing demo tenants.** Demo tenants are deterministically identifiable: `is_demo: true` in Core (`onboard.go:150`; surfaced on `GET /api/v1/tenants/{id}/stats` as `is_demo`, `tenant_api.rs:534`) and email domain `@demo.allsource.dev`. Core already exposes `DELETE /api/v1/tenants/{id}` (`api_v1.rs:223`) and `GET /api/v1/tenants` (list). Add a **CP admin reaper**: `POST /api/v1/admin/tenants/reap-demo` (new, admin-gated) that lists tenants, filters `is_demo == true` (optionally `older_than` / `name == "Demo User"`), and deletes them — with the **recovery dry-run discipline** (`?dry_run=true` → returns the would-delete list + count; apply requires a `confirm_token` echoed from the dry-run, exactly like `recovery_batch`). Surface it as a one-click "Reap demo tenants (N)" action on `/tenants` behind the typed-count confirm. Audit as `admin.recovery.reap_demo` (reuse `RecoveryAuditor.Record`, `recovery_audit.go:49`).
3. **Prevent recurrence (the class of bug).** Make a side-effectful probe *structurally impossible* against `/demo/start`:
   - Gate `DemoStartHandler` behind an explicit `DEMO_ENABLED` flag (default off in prod) so a stray probe gets a 404/403 instead of a new tenant. Demo provisioning becomes an opt-in, never an ambient side effect.
   - Add a CP guard/test asserting **no liveness/status code path calls `/demo/*`** (a `grep` test in CI + a comment banner on `DemoStartHandler`). Document the rule in `MEMORY.md`: *liveness/status probes must be read-only; minting tenants from a health check is a defect.*

**Files/endpoints:** `apps/web/src/app/api/status/services/route.ts` (remove demo mint); `apps/control-plane/onboard.go` (`DEMO_ENABLED` gate); new `apps/control-plane/internal/interfaces/http/admin_tenant_handler.go` `ReapDemo` + use case + route in `main.go` admin group; reuse `recovery_audit.go`, `recoveryGuard`. Admin UI: a reap button on `apps/admin/src/app/(authenticated)/tenants/page.tsx`.

**Acceptance:**
- `/status` polling for 1h creates **zero** new tenants (assert tenant count stable across many polls + cold starts).
- `GET /api/v1/admin/tenants/reap-demo?dry_run=true` returns the exact list of `is_demo` tenants + count and changes nothing; apply without the echoed `confirm_token` returns 400; apply with it deletes them and writes one `admin.recovery.reap_demo` Core event.
- With `DEMO_ENABLED` unset, `POST /api/v1/demo/start` returns 403/404 and creates no tenant.

### Gap 2 — `/monitoring` shows no data

**Root cause (grounded).** `apps/admin/src/lib/metrics-api.ts` calls the **Query Service** — `GET /api/admin/metrics/summary`, `/api/admin/metrics/timeseries`, `/api/cluster/members` (lines 45,58,70) — with `credentials: "include"` against `getApiUrl()` which client-side returns `process.env.NEXT_PUBLIC_API_URL` (line 11). That is **not** the same-origin BFF: it's a direct cross-origin call to QS, with a different auth model than the CP admin JWT, so it never authenticates through the admin chain. `CONTROL_PLANE_CORS.md` §4 explicitly flags this: *"the admin `/monitoring` pages (`metrics-api.ts`) call the Query Service, not the Control Plane, with different auth — they do NOT flow through the BFF."*

**Fix.** Route metrics through the **same-origin BFF**, like every other admin client. Two routing choices:
   - **(a) CP passthrough (preferred).** Add a thin CP proxy/aggregator (`GET /api/v1/admin/metrics/summary`, `…/metrics/timeseries`, `…/cluster/members`) inside the `/api/v1/admin` group that fetches from QS using the CP's own service credential (the CP already talks to backends via the delegation client, `main.go:289`) and returns the existing shapes (`MetricsSummary`, `TimeseriesPoint`, `ClusterMember`). The admin client then hits same-origin `/api/v1/admin/metrics/*` → BFF attaches the admin Bearer → CP authorizes (admin role) → CP fetches QS. One auth model, BFF-consistent.
   - **(b) Dedicated metrics BFF route.** If QS metrics should stay on QS, add a server-side admin route (e.g. `apps/admin/src/app/api/metrics/[...path]/route.ts`) that mints/forwards a QS-acceptable token server-side (never exposing it to the client), mirroring the data-proxy pattern. More surface; only if a CP passthrough is undesirable.
   Change `metrics-api.ts` `getApiUrl()` to return `""` client-side (same-origin) and point the paths at the chosen BFF route. **Apply the resilience rules:** wrap the timeseries/members reads in `asList()` (currently `data.points || data` / `data.members || data` — replace with `asList` so a wrapped/odd response can't crash the chart's `.map`); guard `formatUptime`/numeric fields with `?? 0`.

**Files/endpoints:** `apps/admin/src/lib/metrics-api.ts` (same-origin + `asList`); the QS metrics/cluster routes in `apps/query-service` (confirm shapes); new CP passthrough handler + routes in `apps/control-plane` admin group (option a).

**Acceptance:** `/monitoring` renders summary stat cards + the events/error charts + cluster members with live data after login (no 401/CORS in the console); a wrapped or empty metrics response renders the empty/zero state, never a blank "couldn't load".

### Gap 3 — Per-tenant Events/Members counts are 0

**Root cause (grounded — TWO bugs).**
1. **Field-name mismatch.** The admin list DTO emits `event_count` / `member_count` (singular — `dto/tenant_dto.go:74-75`, `dto/admin_tenant_dto.go:24`), populated by `extractEventCount`/`extractMemberCount` (`list_tenants.go:220-221`). But the frontend `Tenant` type expects **`events_count` / `members_count`** (plural — `apps/admin/src/lib/tenants-api.ts:26-27`). So even a non-zero backend value deserializes to `undefined` and renders 0.
2. **Stale / never-populated source.** `extractEventCount` reads `metadata.quotas.events_used` (`list_tenants.go:255`) — a metadata mirror, not the real Core counter — and `extractMemberCount` reads `metadata.member_count` (`list_tenants.go:285`) which onboarding never sets, so it's always 0. The **real** counters live in Core's usage metering (prompts 027-029): `events_used` / `queries_used` are bumped via `POST /api/v1/tenants/{id}/usage/increment` (`api_v1.rs:208`, meters `Events`/`Queries`, `tenant.rs:185`) and read via `GET /api/v1/tenants/{id}/stats` (`api_v1.rs:205`, returns `usage`/`quotas`/`utilization.events_today`).

**Fix.**
- **Reconcile the contract.** Pick one naming and align both sides. Simplest: change the frontend `Tenant` type to `event_count`/`member_count` to match the live DTO (the CP is the API authority; the `_count` singular is already used by `TenantUsageResponse` too). Update `tenants-api.ts` + `tenants/[id]/page.tsx` consumers.
- **Wire real counters.** Replace the metadata-mirror reads with the **real Core stats**. For the list, the CP enriches each tenant from `GET /api/v1/tenants/{id}/stats` (the same `coreClient.GetTenantStats` the fleet-health signals already call — `fleet_health_helpers.go:239`, `recovery_guarded.go:284`). To avoid N stats calls on a large list, either (a) enrich only the current page, or (b) keep reading `metadata.quotas.events_used` but **ensure it is kept current** by the metering path (the increment handler already writes `events_used` durably — `event_sourced_tenant_repository.rs:459`). For **detail** (`GET /api/v1/admin/tenants/:id`), populate `event_count`/`member_count` from `GetTenantStats` directly (it's a single tenant — cheap). For members, derive `member_count` from the real members list the detail already assembles (`get_admin_tenant_detail.go:40` `extractMemberCount`) rather than the absent metadata field.

**Files/endpoints:** `apps/control-plane/internal/application/usecases/list_tenants.go` (`extractEventCount`/`extractMemberCount` → real stats), `get_admin_tenant_detail.go`; `apps/admin/src/lib/tenants-api.ts` (field names), `app/(authenticated)/tenants/{page,[id]/page}.tsx`. Source of counts: Core `GET /api/v1/tenants/{id}/stats`.

**Acceptance:** the tenant list and detail show non-zero Events and Members for a tenant that has ingested + has members; the numbers match `GET /api/v1/tenants/{id}/stats` `usage`/member list; a tenant with zero usage shows 0 (guarded with `?? 0`, never `undefined`/NaN).

### Gap 4 — Plan list out of sync with LemonSqueezy

**Root cause (grounded — four-way drift).** The canonical tiers are **`free / indie / studio / scale / enterprise`** (`subscription.go:31-45`), with retired aliases `pro/growth/team/starter → indie/studio` (`retiredTierMap`, `subscription.go:50`). But:
- The **admin filter** offers `free / starter / pro / enterprise` (`tenants-api.ts:17` `TenantPlan`; `tenants/page.tsx` `PLAN_OPTIONS`) — **retired 010 names**. Selecting `plan=starter` filters the CP list by exact case-insensitive match against stored canonical tiers (`list_tenants.go` plan filter + `extractPlan`) → matches **nothing**.
- The **web siteConfig** is *correct*: `self-host / indie / studio / scale / enterprise` (`apps/web/src/lib/config.ts` pricing, with the "ONE naming scheme end-to-end" contract comment).
- The **CP catalog** (`get_catalog.go:15` `catalogTiers = ["indie","studio","scale"]`) and **LS variant map** (`LEMON_SQUEEZY_VARIANT_MAP`, keys `"<tier>:<period>"`, `lemon_squeezy_client.go:171`) and the **webhook** variant→tier resolver (`webhook_lemonsqueezy.go:279` authoritative by variant id, `resolveTierByName` fallback) are all on canonical names. The **only** drifted surface is the **admin filter**.

**Fix.**
- **Single source of truth = the CP canonical tiers** (`subscription.go`). Update the admin `TenantPlan` type + `PLAN_OPTIONS` to `free / indie / studio / scale / enterprise` (drop `starter`/`pro`). The admin filter then matches `extractPlan`'s stored values.
- **Make the catalog self-describing so the admin doesn't hardcode tiers.** Have the admin plan filter populate its options from `GET /api/v1/billing/catalog` (the catalog already returns the canonical paid tiers) plus the static `free`/`enterprise` ends — so the filter can never drift from the catalog again. The catalog's variant ids come from `LEMON_SQUEEZY_VARIANT_MAP`; the existing `verify_billing_config.go` already asserts the required tiers are configured — extend its check to flag any admin-known tier missing a variant.

**Files/endpoints:** `apps/admin/src/lib/tenants-api.ts`, `app/(authenticated)/tenants/page.tsx` (tier strings, optionally catalog-driven); reference `GET /api/v1/billing/catalog` (`get_catalog.go`); `subscription.go` is the authority; `verify_billing_config.go` for the guard.

**Acceptance:** the admin plan filter offers exactly the canonical tiers; filtering by `indie`/`studio`/`scale` returns the matching tenants; a tenant on a retired tier (`pro`) still resolves via `MapRetiredTier` and is reachable by its canonical filter; the filter options match `GET /api/v1/billing/catalog`.

---

## 4. Capability pillars

Each pillar: the `apps/admin` surface (reusing the existing client/component vocabulary), the CP/QS endpoints (reuse first; new ones marked **NEW** with shapes consistent with existing admin handlers), and how it composes with the shipped fleet-health/recovery.

### Pillar A — Deep per-tenant 360

**Goal.** One drill-down where the operator sees the tenant's entire world. This **subsumes gap #3** and is mostly a *composition* of reads that already exist — the only genuinely-new server piece is an optional convenience aggregate.

**Surface (`apps/admin`).** Extend `app/(authenticated)/tenants/[id]/page.tsx` into a tabbed/sectioned 360 (reuse `StatCard`, `usage-bar.tsx`, `metrics-chart.tsx`, the `health-chip.tsx` colour vocabulary, the `tenants` table styles). A "View health" deep-link already exists conceptually via the fleet drill-down — surface the health panel inline here too.

**360 data model — each datum and where it's read from (all existing reads):**

| Section | Datum | Source (existing) |
|---|---|---|
| Identity | id, name, status, plan/tier, created_at, description, home_region | `GET /api/v1/admin/tenants/:id` (`admin_tenant_handler.go:208`) |
| Members | member list (email, role, joined_at), **member_count** | detail response `members` (`get_admin_tenant_detail.go:40`) |
| API keys | key names/ids, role string (assert `serviceaccount`) | Core list-keys via fleet `api_key_validity` signal / `recovery rotate-keys` dry-run (`recovery_destructive.go:28`) |
| **Usage trends** | events/queries counts + over-time | Core `GET /api/v1/tenants/{id}/stats` (`api_v1.rs:205`: `usage`, `utilization.events_today`); admin `GET …/tenants/:id/usage` (`TenantUsageResponse.DailyUsage`) |
| **Health signals** | tier + every signal + observed value + reasons | `GET /api/v1/admin/fleet/health/:id` (`fleet_health_handler.go:72`) |
| Subscription/billing | tier, status, renewal/expiry, dunning state, overage, grandfather | detail `Subscription` (`admin_tenant_dto.go:33`) + `GET /api/v1/admin/billing/dunning`; invoices via `…/billing/invoices?tenant_id=` |
| Recent errors / empty-read symptoms | `empty_read_symptom_rate`, identity divergence | health signals + `GET …/recovery/:id/diagnose-identity` (read-only) |
| Audit timeline | tenant audit + recovery events targeting this tenant | detail `audit` link (`/api/v1/tenants/:id/audit`); recovery events `entity_id=recovery:<tenant_id>` in the `admin-recovery` tenant (`recovery_audit.go:71`) |

**NEW (optional) — `GET /api/v1/admin/tenants/:id/overview`.** A single aggregate that fans out the above server-side and returns `{ identity, members, api_keys, usage, health, subscription, recent_symptoms, audit }`, so the 360 is one round-trip instead of 6 client calls. Shape mirrors `adminTenantDetailHALResponse` (HAL `_links` + payload). This is convenience only — the page works by composing the existing endpoints if the aggregate isn't built first.

**Client (`src/lib/tenants-api.ts`).** Add `fetchTenantOverview(id)` (or compose existing `fetchTenantDetail`/`fetchTenantUsage` + a new `fetchTenantHealth(id)` hitting `/api/v1/admin/fleet/health/:id`). All same-origin via the BFF; every list field through `asList`; every number `?? 0`.

### Pillar B — Lifecycle + quota ops

**Goal.** Make the *already-built* recovery actions first-class per-tenant operations, plus the everyday provision/suspend/reactivate/quota/plan controls — all behind the shipped guards.

**Mapping to what exists (reuse, do not rebuild server-side):**

| Operation | Server action | Status |
|---|---|---|
| Suspend / reactivate | `POST /api/v1/admin/tenants/:id/{suspend,unsuspend}` (`admin_tenant_handler.go:160,174`) | exists; `suspend-dialog.tsx` already wired |
| Adjust quotas | `PUT /api/v1/admin/tenants/:id/quotas` (`admin_tenant_handler.go:134`) | exists; `edit-quotas-dialog.tsx` wired |
| Adjust plan / reconcile entitlements | `POST /api/v1/admin/recovery/:id/reconcile-subscription` (Guarded) (`recovery_handler.go:95`) | exists |
| Force re-sync | `POST /api/v1/admin/recovery/:id/resync` (Guarded) | exists |
| Rotate keys | `POST /api/v1/admin/recovery/:id/rotate-keys` (Destructive, confirm_token) (`recovery_destructive.go:21`) | exists |
| Reprovision | `POST /api/v1/admin/recovery/:id/reprovision` (Destructive, confirm_tenant_id) (`recovery_destructive.go:86`) | exists |
| Restore / replay | `POST /api/v1/admin/recovery/:id/restore` (Destructive) (`recovery_destructive.go:144`) | exists |
| Resolve dunning | `POST /api/v1/admin/recovery/:id/resolve-dunning` (Guarded) | exists |
| **Provision** (create tenant) | onboarding flow (`CreateTenantUC`) — surface a guarded admin "create tenant" | mostly exists; thin admin wrapper |

**Surface (`apps/admin`).** Add a **"Operations" panel** to `tenants/[id]/page.tsx` that renders these as buttons, each opening the **existing `components/fleet/recovery-dialog.tsx`** (it already renders the dry-run→preview→typed-confirm/echoed-count→Apply guard UI for recovery). Reuse `recovery-dialog.tsx` verbatim for the destructive ops; reuse `suspend-dialog.tsx`/`edit-quotas-dialog.tsx` for those. New client functions go in `tenants-api.ts` (or a small `recovery-api.ts`) hitting the `/api/v1/admin/recovery/*` paths same-origin; pass `dry_run` + confirmation params straight through. **No new server guards** — the use-case layer already enforces them (`recovery.go:123` `enforceConfirmation`, blast-radius caps, `recovery_audit.go` audit).

**Integration with fleet-health.** Each operation's enabling/recommendation is driven by the tenant's health signals (e.g. surface "Rotate keys" prominently when `api_key_validity` is Critical; surface "Resolve dunning" when `subscription_state` is Degraded). The drill-down reads `…/fleet/health/:id` and lights up the relevant operation.

### Pillar C — Proactive help + comms

**Goal.** Message/email a tenant (or cohort), post in-app notices, run at-risk outreach driven by health tiers, onboarding nudges, and per-tenant support notes — channelled, opt-out-aware, rate-limited, audited. This is the pillar with the **most net-new** code, but it **integrates** the existing email stack rather than reinventing it.

**What exists vs net-new (grounded).**
- **EXISTS:** an SMTP client (`email_client.go` `smtpEmailClient.SendEmail`, env `SMTP_{HOST,PORT,USERNAME,PASSWORD,FROM}`) already used to send billing-warning emails (`billing/check_usage_warnings.go`); a Nylas provider with a `Send()` capability and inbound webhook (`email_webhook_handler.go`, `emailprovider/nylas`); the `email.*` event contract (`docs/contracts/email-events/README.md`); Core event-sourcing for durable audit.
- **DOES NOT EXIST:** any in-app **notice/announcement/broadcast** surface (confirmed: only `alert_rule` operational alerts + a web `use-notification-preferences.ts` user-pref hook). No operator→tenant message channel. No `inbox_*` MCP tools (proposed only).

**Channel design.**

1. **In-app notices (NEW, primary).** Operator posts a notice to a tenant (or a cohort selected by health tier / plan / filter). Persisted as a **Core event** `admin.notice.created` (entity `notice:<tenant_id>`, payload `{title, body, severity, audience, expires_at, actor}`) under a dedicated `admin-comms` system tenant (mirroring `admin-recovery`). The tenant **dashboard** (`apps/web`) reads its notices via a tenant-scoped read and renders a banner; dismissal writes `admin.notice.dismissed`. Event-sourced, durable, auditable, no new DB.
2. **Email (reuse).** For outbound email use the **existing SMTP client** for transactional operator→tenant mail (e.g. "your workspace is at 90% quota", "we noticed your sync stalled — here's how to fix it"), recipient from tenant metadata `email`. For conversational reply-as-mailbox, the Nylas `Send()` path exists but is gated by per-tenant grants and is **out of scope for v1 comms** (that's the AI-inbox product). Emails that correspond to a notice also emit `admin.message.sent` to Core for audit.
3. **Per-tenant support notes (NEW).** Free-text operator notes attached to a tenant, persisted as `admin.note.created` Core events (entity `note:<tenant_id>`), shown on the 360 audit timeline. Internal-only; never sent to the tenant.

**NEW endpoints (CP, admin-gated, shapes consistent with existing admin handlers):**

```
POST /api/v1/admin/notices            # create an in-app notice for a tenant or cohort
  body: { audience: {tenant_id} | {tier|plan|health_tier|filter}, title, body,
          severity: "info"|"warning"|"critical", expires_at?, dry_run?, confirm_token? }
  dry_run:true → { would: { recipient_tenant_ids:[…], count }, confirm_token }  # cohort blast-radius preview
GET  /api/v1/admin/notices?tenant_id=  # list notices (admin view)
POST /api/v1/admin/messages            # send an operator→tenant email (SMTP), audited
  body: { tenant_id, template|subject+body, dry_run? }
POST /api/v1/admin/tenants/:id/notes   # add a support note
GET  /api/v1/admin/tenants/:id/notes   # list support notes
```

Plus a **tenant-facing read** (QS or CP, tenant-scoped, BFF/normal auth) `GET /api/v1/notices` for the dashboard banner, and `POST /api/v1/notices/:id/dismiss`.

**Templates.** A small set of operator templates keyed to health/lifecycle events: `at_risk_outreach` (driven by `tier ∈ {At-Risk, Critical}`), `quota_warning` (reuse the existing 80/100% logic from `check_usage_warnings.go`), `onboarding_nudge` (tenant created but `events_count == 0` after N days), `dunning_reminder`. Templates render from tenant + health context; the operator reviews before send.

**Opt-out, rate limits, audit (server-enforced):**
- **Opt-out.** Honor a per-tenant `comms_opt_out` flag (and per-category, e.g. marketing vs operational — operational/critical can be exempt). Reuse the web `use-notification-preferences` shape as the tenant-facing control; the CP checks the flag before sending and records `skipped_opt_out` in the audit.
- **Rate limits.** Cohort sends go through the **recovery blast-radius pattern**: a `max_recipients` cap + dry-run preview + echoed `confirm_token` (reuse `recoveryGuard.mintConfirmToken`/`validateConfirmToken`, `recovery.go:91`). Per-tenant per-category cooldown (e.g. no more than one `quota_warning` per day) enforced by reading the last `admin.message.sent` event for that tenant+template.
- **Audit.** Every notice/message/note write emits a Core event (`admin.notice.*`, `admin.message.sent`, `admin.note.created`) via `IngestEvent` (the same pattern as `recovery_audit.go:49`). "Who messaged whom, when, why, opted-out?" is a durable, event-sourced query.

**Surface (`apps/admin`).** A "Communicate" panel on the 360 (send notice / send email / add note) using a new `recovery-dialog.tsx`-style guarded dialog for cohort sends (dry-run preview of recipients + typed count). A fleet-level "At-risk outreach" view that lists `tier ∈ {At-Risk, Critical}` tenants (from `…/fleet/health?tier=at_risk`) with a one-click templated outreach. New client file `src/lib/comms-api.ts` (same-origin BFF, `asList`, guards).

### Pillar D — Billing + revenue ops

**Goal.** Plan↔LemonSqueezy sync (gap #4), invoices, refunds, dunning resolution, MRR/ARR/churn analytics, and the `/monitoring` metrics feed (gap #2) — surfaced on the existing endpoints.

**Reuse (already shipped):**
- Revenue/MRR/ARR/churn: `GET /api/v1/admin/billing/revenue?range=` (`admin_billing_handler.go`, `billing/admin_revenue.go`; client `billing-api.ts fetchRevenue`).
- Invoices: `GET /api/v1/admin/billing/invoices` (`admin_list_invoices.go`; `fetchInvoices` with `asList`).
- Refund: `POST /api/v1/admin/billing/refund` (`admin_refund.go`; `processRefund`).
- Dunning: `GET /api/v1/admin/billing/dunning` (`admin_dunning.go`; `fetchDunning`) → resolve via `POST /api/v1/admin/recovery/:id/resolve-dunning` (Guarded).
- Catalog: `GET /api/v1/billing/catalog` (`get_catalog.go`) — drive the plan filter (gap #4).

**Surface (`apps/admin`).** The existing `/billing` page already consumes most of this. Add: a **per-tenant billing tab** in the 360 (invoices filtered by `tenant_id`, current subscription/renewal, one-click "resolve dunning" via the recovery dialog), and the **catalog-sync status** (does every catalog tier have a configured LS variant? — surface `verify_billing_config` results). The `/monitoring` feed is fixed per gap #2 and lives on `/monitoring` (not strictly billing, but the prompt groups the metrics feed under D — wire it via the CP passthrough so it's BFF-consistent).

**Integration with fleet-health.** `subscription_state`, `grandfather_window`, `events_quota_pct` are already health signals; the billing tab links to the health drill-down and vice versa. Plan changes go through `reconcile-subscription` so quotas re-derive from `QuotasForTier` (no manual drift).

---

## 5. Guarded view-as (read-only impersonation)

A time-boxed, fully-audited "view the product as this tenant sees it" mode for support/debugging. **Read-only: it never writes on the tenant's behalf.** Every guard below has a stated reason.

### 5.1 The token mint — scoped, short-TTL, distinct from the tenant's real session

Reuse the **existing delegation primitive** as the base, with a tighter, read-only variant. The current `SignDelegationJWT(userID, tenantID, role)` (`auth.go:121`) mints a **60-second** backend-forwarding token with claims `{sub, tenant_id, role, is_api_key, exp(+60s), iat, iss}`. A view-as token is a deliberate sibling:

```
SignViewAsJWT(adminUserID, targetTenantID) → JWT, distinct from any real session:
  claims: {
    sub:        adminUserID,         // WHO is impersonating — never the tenant's real user id
    tenant_id:  targetTenantID,      // the tenant whose data is being viewed
    role:       "readonly",          // entities.RoleReadOnly — NOT serviceaccount, NOT admin, NOT the tenant's role
    view_as:    true,                // marks this as an impersonation token (enforced downstream)
    act_as:     adminUserID,         // audit trail: the real actor behind the view
    exp:        now + 15m,           // SHORT time-box (see WHY)
    iat, iss
  }
```

- **WHY a distinct token (not the tenant's real session):** we must never possess or replay the tenant's actual credentials; minting our own scoped token means the tenant's real session is untouched, the token is attributable to the admin (`act_as`), and it can be revoked/expired independently.
- **WHY `role: "readonly"`:** `RoleReadOnly` already exists (`roles.go`). Read endpoints accept it; **write endpoints reject it** (no `events:write`, no admin perms). This is the primary read-only enforcement — the token simply *cannot* authorize a mutation.
- **WHY `view_as: true`:** a defense-in-depth marker so any mutating handler (and the BFF) can hard-refuse a `view_as` token even if a write route ever accidentally accepted `readonly`. Belt and suspenders, because the cost of a write on a customer's behalf is unacceptable.
- **WHY short TTL (15m, not the 7-day session):** a support session is minutes; a long-lived impersonation token is a standing credential-theft risk. Short TTL bounds the blast radius if it leaks. Auto-expiry is the backstop for a forgotten "exit".

### 5.2 Read-only enforcement (layered)

1. **Role.** `readonly` can't reach write endpoints (Core/QS reject non-`PermissionWrite` roles on mutating routes).
2. **`view_as` refusal on writes.** The CP (and any QS write path) rejects `view_as: true` tokens on any mutating method — independent of role.
3. **The view-as surface is read-only by construction.** View-as renders the **tenant's product** (the `apps/web` dashboard) in a read-only frame; the admin app mints the token server-side and proxies reads only. No admin action buttons are exposed in view-as mode.

### 5.3 Mint/teardown flow + banner

- **Start.** Operator clicks "View as tenant" on the 360. The admin app's BFF (server-side, holds the admin Bearer) calls **NEW** `POST /api/v1/admin/tenants/:id/view-as` (admin-gated) → CP mints `SignViewAsJWT` and writes a Core audit event `admin.viewas.started` (entity `viewas:<tenant_id>`, payload `{actor, tenant_id, exp}`) **before** returning the token. The token is set in a separate, scoped, short-TTL httpOnly cookie (e.g. `viewas_token`, distinct from `admin_token`) and the operator is dropped into the read-only product frame.
- **Banner.** A **persistent, unmissable** "You are viewing as `<tenant name>` — read-only — [Exit]" bar renders for the entire session (top of every view-as page). **WHY:** the operator must never forget they're impersonating; an accidental belief that they're in their own account is how mistakes happen. The banner also shows the auto-expiry countdown.
- **Exit / teardown.** One-click "Exit" clears the `viewas_token` cookie and writes `admin.viewas.stopped`. **Auto-expiry:** when the 15-minute token lapses, the frame is dead (every read 401s) and the operator is returned to the admin app; a scheduled/lazy check emits `admin.viewas.stopped` (reason `expired`) so start/stop always pair in the audit.

### 5.4 Audit — every start/stop is a Core event

Both `admin.viewas.started` and `admin.viewas.stopped` are durable Core events under the `admin-comms`/`admin-recovery`-style system tenant, written via the same `IngestEvent` pattern (`recovery_audit.go:49`). "Who viewed whose data, when, for how long?" is a queryable, immutable record. **Explicitly: there is no `admin.viewas.wrote` event because view-as never writes** — and a write attempt is itself an alarmable audit (`view_as token rejected on write`).

### 5.5 Hard constraints (restated)

- **NO writes on the tenant's behalf — ever.** Read-only by role, by `view_as` refusal, and by surface.
- Distinct, short-TTL, scoped token; the tenant's real session is never touched.
- Visible banner + one-click exit + auto-expiry.
- Start and stop are both Core audit events.

---

## 6. Resilience & quality standards (codified — page "definition of done")

These are the lessons this cycle paid for, turned into rules every new admin page MUST follow. Grounded in the existing code that already does it right.

1. **List clients always return arrays.** Every list-returning function in `src/lib/*-api.ts` runs its response through `asList()` (`security-api.ts:26`, `billing-api.ts:84`) before returning. A wrapped (`{rules:[…]}`), `{items}`, `{data}`, or `undefined` response reaching a `.map` crashes the whole route ("x.map is not a function"). New clients copy `asList` and pass the expected key(s).
2. **Every page sits under the error boundary.** `app/(authenticated)/error.tsx` exists and renders `error.message` (never a blank "couldn't load"). Keep it; do not catch-and-swallow into a silent empty state — surface the real message so a missing/odd API field is diagnosable.
3. **Guard missing numeric/date fields.** Never call `.toLocaleString()`/`.toFixed()`/`new Date()` on a possibly-undefined API field. Coalesce with `?? 0` (numbers) / a null-guard (dates) — the error boundary's own doc comment names `.toLocaleString()` on undefined as the canonical crash.
4. **Client calls go through the same-origin BFF.** Client code calls `/api/v1/...` with `getApiUrl()` returning `""` client-side (`tenants-api.ts:8`); the BFF (`app/api/v1/[...path]/route.ts`) attaches the admin Bearer. **Never** a direct cross-origin `credentials: "include"` to the CP — the CP is **Bearer-only** and ignores cookies (this is exactly what broke `/monitoring`, gap #2). Metrics/cluster reads must be migrated onto the BFF.
5. **Edge-middleware code must not use Node `Buffer`.** `proxy.ts` runs in the Edge runtime where `Buffer` is unavailable; JWT decode uses `atob`+`TextDecoder` (`auth.ts:37`). Using `Buffer` there crashed the middleware for every authenticated request once a token cookie was present. New middleware/edge code uses web-standard APIs only. (Note: the status-probe route in `apps/web` uses `Buffer.from(...,"base64url")` — that runs in the Node server runtime, not Edge, so it's fine there; the rule is Edge-specific.)
6. **Mutations are guarded + audited.** Any new mutating action reuses the recovery guard discipline: `dry_run` preview where applicable, server-enforced confirmation for destructive/cohort ops (typed id or echoed `confirm_token`), and a Core audit event before reporting success. Client-side confirmation alone is insufficient (a raw curl/MCP must hit the same guard).

**Definition of done for an admin page:** (a) every list client returns an array via `asList`; (b) the page renders under the error boundary and shows real error text; (c) every number/date from the API is guarded; (d) all data calls are same-origin BFF (no cross-origin cookie calls); (e) any edge/middleware code is Buffer-free; (f) mutations are dry-runnable + confirmed + audited; (g) `data-testid` hooks on the key elements for proofshot.

---

## 7. Auth, safety & audit model

Reconciled with the shipped recovery safety model — **not forked**.

- **One admin-role gate, applied once at mint.** `ADMIN_EMAILS` (comma-separated, case-insensitive) → `RoleAdmin` at token-mint in `roleForEmail()` (`auth.go:50`); `AdminAuthMiddleware` enforces only the resulting `role == "admin"` claim (`admin_middleware.go:69`). Every new admin endpoint (reap-demo, notices, messages, notes, view-as, metrics passthrough, tenant/overview) sits inside the existing `/api/v1/admin` group and inherits this with **zero new auth code**. There is no second allowlist.
- **BFF Bearer pattern everywhere.** Client → same-origin `/api/v1/...` → BFF reads `admin_token` httpOnly cookie → forwards to CP with `Authorization: Bearer`. The CP is Bearer-only. This is the single client→server path for *all* admin data, including the migrated metrics feed.
- **Every mutating / comms / view-as action is a durable Core audit event.** Reuse `IngestEvent` (`recovery_audit.go:49`): `admin.recovery.*` (existing), `admin.recovery.reap_demo` (gap #1), `admin.notice.*` / `admin.message.sent` / `admin.note.created` (pillar C), `admin.viewas.{started,stopped}` (view-as). System tenants `admin-recovery` / `admin-comms` keep these out of per-customer queries. Never a fire-and-forget log line only.
- **Dry-run + confirmation for destructive/cohort ops.** Reuse `recoveryGuard` (`recovery.go:79`): `dry_run` returns a `would` preview; destructive single-tenant ops require typed `confirm_tenant_id`; token-gated ops (rotate-keys, batch, reap-demo, cohort comms) require an echoed `confirm_token` from a prior dry-run; blast-radius caps (`BatchDefaultMaxTenants=25`, `BatchAbsoluteCeiling=100`, `recovery.go:33`) apply to any cohort action. Enforced **server-side** in the use case, so the UI, a raw curl, and a future MCP tool are all bound by the same guard.
- **Comms opt-out + rate limits** are server-enforced (per-tenant `comms_opt_out`, per-category cooldown, cohort `max_recipients` cap) — see §4 Pillar C.
- **View-as is read-only** by role (`readonly`), by `view_as` refusal on writes, and by surface; short-TTL distinct token; banner + exit + auto-expiry; start/stop audited — see §5.

---

## 8. Gap analysis table

`exists` = shipped & working · `partial` = some pieces shipped, gap remains · `missing` = net-new.

| # | Capability | State | Where it lives / will live |
|---|---|---|---|
| G1 | Demo-litter: stop probe | **partial** (bug live) | `apps/web/.../status/services/route.ts` (mint to remove); `onboard.go` `DEMO_ENABLED` |
| G1 | Demo-litter: reap demo tenants | **missing** | NEW `POST /api/v1/admin/tenants/reap-demo` (CP) + reuse Core `DELETE /api/v1/tenants/{id}` + `RecoveryAuditor` |
| G1 | Demo-litter: recurrence guard | **missing** | `onboard.go` gate + CI grep test + `MEMORY.md` rule |
| G2 | `/monitoring` data feed | **partial** (wrong auth) | `metrics-api.ts` → same-origin BFF; NEW CP `…/admin/metrics/*` passthrough; QS metrics routes |
| G3 | Per-tenant counts (field-name) | **partial** (bug) | `tenants-api.ts:26-27` vs `dto/tenant_dto.go:74-75` |
| G3 | Per-tenant counts (real source) | **partial** | `list_tenants.go` `extractEventCount/Member` → Core `GET …/tenants/{id}/stats` |
| G4 | Plan/tier catalog reconcile | **partial** (admin drift) | `tenants-api.ts`/`tenants/page.tsx` → canonical `subscription.go` tiers; catalog-driven via `GET /api/v1/billing/catalog` |
| A | Per-tenant 360 (compose reads) | **partial** | `tenants/[id]/page.tsx`; reads all exist (`fleet/health/:id`, `stats`, billing, audit) |
| A | 360 convenience aggregate | **missing** (optional) | NEW `GET /api/v1/admin/tenants/:id/overview` |
| B | Lifecycle/quota ops (server) | **exists** | `/api/v1/admin/{tenants,recovery}/*` — full recovery API shipped |
| B | Lifecycle/quota ops (UI surfacing) | **missing** | "Operations" panel on 360 reusing `recovery-dialog.tsx`/`suspend`/`edit-quotas` |
| C | Outbound email (transactional) | **exists** | `email_client.go` `SendEmail` (billing-warnings today) |
| C | In-app notices / broadcast | **missing** | NEW `POST /api/v1/admin/notices` + `admin.notice.*` Core events + dashboard banner |
| C | Operator→tenant message (email, audited) | **missing** | NEW `POST /api/v1/admin/messages` over existing SMTP client |
| C | Per-tenant support notes | **missing** | NEW `…/tenants/:id/notes` + `admin.note.*` events |
| C | At-risk outreach (health-driven) | **missing** | NEW fleet view over `…/fleet/health?tier=at_risk` + templated send |
| D | Revenue/invoices/refund/dunning | **exists** | `/api/v1/admin/billing/*` shipped |
| D | Catalog-sync status surface | **partial** | reuse `verify_billing_config.go`; surface in `/billing` |
| V | Guarded view-as (read-only) | **missing** | NEW `POST /api/v1/admin/tenants/:id/view-as` (`SignViewAsJWT`) + banner + `admin.viewas.*` events |

**Counts:** exists 4, partial 8, missing 9. Everything new completes a partial or fills a missing — nothing re-implements a shipped capability.

---

## 9. Phased build plan

Ordered so the **broken gaps land first** (especially demo-litter, which pollutes prod every poll), then the pillars. Each phase names the apps/files it touches and verifiable acceptance.

### Phase 0 — Stop the bleeding: demo-litter (gap #1) — **do first**
- **Apps:** `apps/web` (remove demo mint), `apps/control-plane` (reaper + `DEMO_ENABLED` gate).
- **Scope:** swap `status/services/route.ts` to a non-mutating probe (persistent `STATUS_MONITOR_TOKEN` or `/livez`); NEW `POST /api/v1/admin/tenants/reap-demo` (dry-run + `confirm_token`, `is_demo` filter, Core `DELETE`); `DEMO_ENABLED` gate on `DemoStartHandler`; CI grep test "no status path calls `/demo/*`"; `MEMORY.md` rule.
- **Acceptance:** 1h of `/status` polling creates 0 tenants; reap dry-run lists `is_demo` tenants + count and mutates nothing; apply (with echoed token) deletes them + writes `admin.recovery.reap_demo`; `/demo/start` is 403/404 with `DEMO_ENABLED` unset.

### Phase 1 — Per-tenant counts (gap #3) — Control Plane + admin
- **Apps:** `apps/control-plane` (Go), `apps/admin` (TS).
- **Scope:** align the count field names (`tenants-api.ts` ↔ DTO); wire `extractEventCount`/`extractMemberCount` (list) and detail to real Core `GET …/tenants/{id}/stats`; member_count from the real members list.
- **Acceptance:** list + detail show non-zero, stats-matching Events/Members; zero-usage tenant shows guarded 0.

### Phase 2 — Plan/tier reconcile (gap #4) — admin (+ CP guard)
- **Apps:** `apps/admin` (TS), `apps/control-plane` (extend `verify_billing_config`).
- **Scope:** `TenantPlan`/`PLAN_OPTIONS` → canonical tiers; optionally drive options from `GET /api/v1/billing/catalog`; extend `verify_billing_config` to flag missing variants.
- **Acceptance:** filter offers canonical tiers; `indie/studio/scale` filters return matching tenants; retired-tier tenants reachable by canonical filter; options match the catalog.

### Phase 3 — `/monitoring` feed (gap #2) — CP passthrough + admin
- **Apps:** `apps/control-plane` (NEW `…/admin/metrics/*` passthrough), `apps/admin` (`metrics-api.ts` → same-origin + `asList`).
- **Scope:** CP fetches QS metrics/cluster with its service credential, returns existing shapes; admin client same-origin BFF; `asList` + `?? 0` guards.
- **Acceptance:** `/monitoring` renders live summary + charts + cluster members, no 401/CORS; wrapped/empty response → zero/empty state, not a crash.

### Phase 4 — Per-tenant 360 (pillar A) — admin (+ optional CP aggregate)
- **Apps:** `apps/admin` (compose existing reads), optional `apps/control-plane` (`GET …/tenants/:id/overview`).
- **Scope:** expand `tenants/[id]/page.tsx` into the 360 (identity/members/keys/usage/health/billing/symptoms/audit), each section reading its existing endpoint; add `fetchTenantHealth(id)` (`…/fleet/health/:id`); optionally the overview aggregate.
- **Acceptance:** the 360 shows all eight sections with live data per the §4-A source table; health panel matches `…/fleet/health/:id`; every list via `asList`, numbers guarded.

### Phase 5 — Lifecycle/quota ops surfacing (pillar B) — admin
- **Apps:** `apps/admin` only (server actions already shipped).
- **Scope:** "Operations" panel on the 360 reusing `recovery-dialog.tsx` (destructive), `suspend-dialog.tsx`, `edit-quotas-dialog.tsx`; client funcs hitting `/api/v1/admin/recovery/*` + `…/tenants/:id/{suspend,unsuspend,quotas}` same-origin; health-driven prominence.
- **Acceptance:** each op opens the correct guarded dialog; destructive ops keep Apply disabled until dry-run preview + typed/echoed confirm; a successful apply writes the matching `admin.recovery.*` event (assert via events query); no new server guard added.

### Phase 6 — Proactive comms (pillar C) — CP + admin (+ web banner)
- **Apps:** `apps/control-plane` (NEW notices/messages/notes + opt-out + rate-limit + audit), `apps/admin` (comms panel + at-risk view), `apps/web` (tenant notice banner read).
- **Scope:** `admin.notice.*`/`admin.message.sent`/`admin.note.*` Core events; `/api/v1/admin/{notices,messages}` + `…/tenants/:id/notes`; cohort sends through the recovery blast-radius guard; SMTP reuse for email; tenant-facing `GET /api/v1/notices` + dismiss; templates (`at_risk_outreach`, `quota_warning`, `onboarding_nudge`, `dunning_reminder`).
- **Acceptance:** a notice to a tenant renders in the dashboard banner and is dismissible (both audited); a cohort send dry-run previews recipients + count and requires the echoed token; an opted-out tenant is skipped with `skipped_opt_out` in the audit; a second `quota_warning` within the cooldown is rate-limited; an operator→tenant email sends via SMTP and emits `admin.message.sent`.

### Phase 7 — Guarded view-as (read-only impersonation) — CP + admin
- **Apps:** `apps/control-plane` (`SignViewAsJWT` + `…/tenants/:id/view-as` + write-refusal of `view_as` + audit), `apps/admin` (server-side mint via BFF, `viewas_token` cookie, banner, exit, auto-expiry).
- **Scope:** mint a `readonly`+`view_as` 15-min token distinct from the session; enforce read-only (role + `view_as` refusal + surface); persistent banner + one-click exit; `admin.viewas.{started,stopped}` events.
- **Acceptance:** starting view-as drops into the read-only product frame with the persistent banner + countdown; any write attempt is rejected (and alarmed) — assert no write succeeds; exit + auto-expiry both clear the cookie and write `admin.viewas.stopped`; start always has a paired stop in the audit; the tenant's real session is never touched.

### Phase 8 — Optional MCP parity (mcp-server-elixir)
- **Apps:** `apps/mcp-server-elixir` only. **Explicitly NOT `prime-mcp`.**
- **Scope:** expose the 360 + lifecycle ops + comms as MCP tools reusing the shipped `recovery_*`/fleet machinery (the foundation already added `fleet_health_summary`/`tenant_health_assessment`/`recovery_*`). Add `tenant_overview`, `tenant_notice`, gated by `control_plane_enabled` (read) / `ALLSOURCE_SYSTEM_ADMIN` (mutating), per the foundation's gate model.
- **Acceptance:** tools appear only when the gates are set; mutating tools enforce the same dry-run/confirm guards; absent from `prime-mcp` (grep proves it).

---

## 10. Recommended packaging — follow-on `/create-prompt` invocations

**Group by app + language** so each build prompt is single-app, single-language with crisp acceptance — the monorepo isolation rule forbids cross-app coupling, and cross-app changes are weakly verifiable. Gap fixes front-loaded. Order respects dependency edges (server endpoints before the admin UI that consumes them).

1. `/create-prompt` — **"Control Plane: kill demo-litter — reaper + DEMO_ENABLED gate (Go) and the web status-probe fix (TS)"** — *split into two* if cross-app is undesirable: (1a) Go CP `reap-demo` endpoint + `DEMO_ENABLED` gate + CI grep test; (1b) `apps/web` `status/services/route.ts` non-mutating probe. Acceptance: Phase 0.
2. `/create-prompt` — **"Control Plane: real per-tenant counts + catalog-verify (Go)"** — wire list/detail counts to Core `/stats`; extend `verify_billing_config`. Acceptance: Phase 1 (CP half) + Phase 2 (guard).
3. `/create-prompt` — **"Control Plane: admin metrics passthrough (Go)"** — `…/admin/metrics/*` + `…/cluster/members` proxy. Acceptance: Phase 3 (CP half).
4. `/create-prompt` — **"Admin web: gap-fix client pass (TS)"** — count field names, plan-tier options (catalog-driven), `metrics-api.ts` → same-origin BFF + `asList`. Acceptance: Phase 1/2/3 (admin halves). Depends on prompts 2 & 3.
5. `/create-prompt` — **"Admin web: per-tenant 360 + operations panel (TS)"** — the 360 composition + the recovery/suspend/quota dialogs surfacing. Acceptance: Phase 4 + Phase 5. (Optional CP `…/tenants/:id/overview` as a tiny Go side-prompt.)
6. `/create-prompt` — **"Control Plane: proactive comms — notices/messages/notes (Go)"** — events, endpoints, opt-out, rate-limit, audit, SMTP reuse. Acceptance: Phase 6 (CP half).
7. `/create-prompt` — **"Admin web + web: comms surfaces (TS)"** — admin comms panel + at-risk view (`apps/admin`) and tenant notice banner (`apps/web`). *Split per app* (isolation). Acceptance: Phase 6 (UI halves). Depends on prompt 6.
8. `/create-prompt` — **"Control Plane: guarded view-as token + write-refusal + audit (Go)"** — `SignViewAsJWT`, `…/view-as`, `view_as` enforcement. Acceptance: Phase 7 (CP half).
9. `/create-prompt` — **"Admin web: view-as frame, banner, exit, auto-expiry (TS)"** — server-side mint via BFF, `viewas_token`, read-only frame. Acceptance: Phase 7 (admin half). Depends on prompt 8.
10. *(optional)* `/create-prompt` — **"Elixir MCP: tenant 360 + comms tools (Elixir)"** — Phase 8, mcp-server-elixir only.

---

## Appendix — grounding (every existing-code claim cites a file actually opened)

- Demo-litter probe: `apps/web/src/app/api/status/services/route.ts:46-79` (`getMonitorToken` → `POST /api/v1/demo/start`); `apps/control-plane/onboard.go:109-222` (`DemoStartHandler`, `name:"Demo User"`, `is_demo:true`, `demo-…@demo.allsource.dev`).
- Core tenant ops: `apps/core/src/infrastructure/web/api_v1.rs:201-223` (`POST /tenants`, `GET /tenants`, `GET /tenants/{id}/stats`, `POST /tenants/{id}/usage/increment`, `DELETE /tenants/{id}`); meters `tenant.rs:185` (`events_used`/`queries_used`); durable increment `event_sourced_tenant_repository.rs:459`; `is_demo` on stats `tenant_api.rs:534`.
- Monitoring auth bug: `apps/admin/src/lib/metrics-api.ts:11,45,58,70` (QS, `credentials:"include"`, not BFF); `docs/runbooks/CONTROL_PLANE_CORS.md` §4 ("metrics-api.ts call the Query Service … do NOT flow through the BFF").
- Per-tenant counts: frontend `apps/admin/src/lib/tenants-api.ts:26-27` (`events_count`/`members_count`); DTO `apps/control-plane/internal/application/dto/tenant_dto.go:74-75` + `admin_tenant_dto.go:24` (`event_count`/`member_count`); `list_tenants.go:220-221,255,285` (`extractEventCount` reads `metadata.quotas.events_used`; `extractMemberCount` reads absent `metadata.member_count`).
- Tier source of truth: `apps/control-plane/internal/domain/entities/subscription.go:31-45` (canonical tiers), `:50` (`retiredTierMap`), `:189` (`HighestActiveTier`), `:252-275` (`TierQuotaMap`), `:298` (`QuotasForTier`); admin drift `apps/admin/src/lib/tenants-api.ts:17` + `tenants/page.tsx` `PLAN_OPTIONS`; catalog `internal/application/usecases/get_catalog.go:15`; LS map `internal/infrastructure/clients/lemon_squeezy_client.go:171`; webhook resolver `webhook_lemonsqueezy.go:279`; web config `apps/web/src/lib/config.ts` pricing.
- Shipped fleet/recovery: `apps/control-plane/internal/interfaces/http/fleet_health_handler.go:44,72,98,109`, `recovery_handler.go` (all actions), `recovery.go:33,79,91,123` (guards/blast-radius/confirm-token), `recovery_destructive.go:21,86,144` (rotate/reprovision/restore), `recovery_audit.go:15,49,71` (Core audit event); routes `apps/control-plane/main.go:643-660`.
- Auth/mint primitives: `apps/control-plane/auth.go:24-36` (Claims), `:50` (`roleForEmail`→`RoleAdmin`/`RoleDeveloper`, `ADMIN_EMAILS`), `:121` (`SignDelegationJWT`, 60s), `:140` (`SignAPIKey`, role `serviceaccount`, 365d), `:668` (`SessionHandler`); roles `internal/domain/entities/roles.go` (`RoleReadOnly`, `RoleServiceAccount`); gate `admin_middleware.go:69`.
- Admin app patterns: BFF `apps/admin/src/app/api/v1/[...path]/route.ts`; edge gate `apps/admin/src/proxy.ts`; auth + edge-safe decode `apps/admin/src/lib/auth.ts:37,72`; `asList` `apps/admin/src/lib/security-api.ts:26`, `billing-api.ts:84`; error boundary `apps/admin/src/app/(authenticated)/error.tsx`; dialogs `apps/admin/src/components/tenants/suspend-dialog.tsx`, `components/fleet/recovery-dialog.tsx`; sidebar `components/sidebar.tsx:25`.
- Comms stack: SMTP `apps/control-plane/internal/infrastructure/clients/email_client.go` (`SendEmail`), used by `internal/application/usecases/billing/check_usage_warnings.go`; inbound `email_webhook_handler.go`, `inbox_connect_handler.go`, routes `main.go:604-609`; event contract `docs/contracts/email-events/README.md` (`email.{received,sent,triaged,replied,archived,drafted}`); AI-inbox design `docs/proposals/AI_INBOX_ON_ALLSOURCE.md`. **In-app notices do not exist today** (searched; only `alert_rule` + web `use-notification-preferences`).
