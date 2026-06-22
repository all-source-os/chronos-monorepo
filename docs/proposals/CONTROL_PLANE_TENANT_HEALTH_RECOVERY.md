# Control Plane — Fleet-Wide Tenant Health & Recovery

**Status:** P0–P3 **shipped to `main`** (not yet deployed) — Control Plane API `a02667e`, admin UI `e233bee`, MCP tools `b6e3c88`. **P4 (docs + small polish) in progress.**
**Author:** design pass, 2026-06-22; build pass, 2026-06-22.
**Scope:** one unified, fleet-wide "is every tenant healthy, and if not, fix it" capability for the single system owner, surfaced through (a) the Go Control Plane admin API, (b) the standalone Next.js admin app, and (c) the Elixir MCP server.

---

## TL;DR — the bet

The operator already has scattered admin primitives across three apps, but no single answer to *"is every tenant in the fleet healthy right now, and if one isn't, can I fix it from here?"* Today that question is answered by hand from runbooks — `docs/runbooks/DIAGNOSING_PIPELINE_DATA_VISIBILITY.md` (the edition trap, wrong-tenant provision, double-unwrap, 404-proxy) and `docs/runbooks/PRICING_BILLING_CUTOVER.md` (dunning, grandfather windows, overage). Every recurrence costs hours.

This proposal adds exactly three things on top of what already ships:

1. A **computable tenant-health model** (Healthy / Degraded / At-Risk / Critical) derived from named signals read from the systems that already hold them.
2. A small set of **new Control Plane endpoints** — a fleet-health aggregate plus guarded recovery actions — that reuse the existing `AdminAuthMiddleware`, Clean-Architecture use-case pattern, and HAL response shape.
3. **Admin-UI pages** and **Elixir MCP tools** that consume those endpoints, reusing the existing `src/lib/*-api.ts` client + `components/monitoring|tenants` vocabulary and the `mcp_tools.ex` tool/handler/gate pattern.

No new app. No PostgreSQL for events. No fleet tools in `prime-mcp` (it is single-tenant by design). Every Destructive recovery action ships with a designed guard (dry-run, typed-name confirmation, blast-radius cap) and a mandatory audit event sourced into Core.

---

## Architecture invariants this design honors (and why)

These are non-negotiable per `CLAUDE.md` and `MEMORY.md`. Prior sessions violated them; this section makes them explicit so the build phase cannot regress.

- **Core IS the database.** Full durability — WAL (CRC32, fsync), Parquet (Snappy), DashMap (in-memory reads). Event data survives restarts. The health model treats Core as the durable source of truth; "tenant reports empty data" is classified as a *read-path/identity* symptom, **never** as data loss (this is the central lesson of the data-visibility runbook).
- **No PostgreSQL for events.** The Control Plane runs with **no PostgreSQL at all** — `main.go` logs `"Persistence: core (no PostgreSQL)"` and `internal.NewContainerWithConfig` is built from a `CoreClient`, not a DB pool. Tenants, subscriptions, quotas, and overage all live as Core tenant-metadata JSON (verified below). New health/recovery state follows the same rule: read from Core/QS, write audit + recovery events into Core.
- **Admin/system MCP tools go in `apps/mcp-server-elixir`, never `apps/prime-mcp`.** `prime-mcp` is single-tenant by design (`apps/prime-mcp/src/projection_registry.rs:22` — *"Single-tenant for now … each process is one tenant"*; `apps/prime-mcp/src/http.rs:294,379` — *"prime-mcp is local + single-store, so there is no tenant boundary here"*). A fleet/cross-tenant tool in a single-tenant server is both an architecture violation and a security footgun. The Elixir server already reaches the Control Plane (`ControlPlaneClient`) and already gates tenant tools behind `control_plane_enabled` — it is the correct home.
- **Reuse existing surfaces.** Extend `apps/control-plane` admin API, `apps/admin` web app, and `apps/mcp-server-elixir`. Do **not** push admin features into `apps/web` (Vercel, user-facing, no admin route group by design).
- **Deployment topology.** `apps/control-plane` and `apps/mcp-server-elixir` are **Fly** backends. `apps/admin` deploys as its **own standalone app** (it is NOT the Vercel web frontend). Do not mix these up.

---

## 1. Gap analysis — capability × surface

The spine of the doc. Each capability is scored against the three surfaces and given a verdict. Verified file paths in the "Where it lives today" column.

| Capability | Control Plane API | Admin UI (`apps/admin`) | MCP (`mcp-server-elixir`) | Verdict | Where it lives today |
|---|---|---|---|---|---|
| List tenants + status | ✅ `GET /api/v1/admin/tenants` | ✅ `/tenants` page | ⚠️ none (no `tenant_list` tool) | **partial** (MCP gap) | `main.go:601`, `admin_tenant_handler.go:91`, `tenants/page.tsx`, `tenants-api.ts:fetchTenants` |
| Tenant detail + usage | ✅ `GET …/tenants/:id`, `…/:id/usage` | ✅ `/tenants/[id]` page | ⚠️ `tenant_usage` only (no detail) | **partial** | `main.go:603-604`, `admin_tenant_handler.go:208,228`, `tenants/[id]/page.tsx`, `mcp_tools.ex:5999`(tenant tools) |
| Subscription state (tier/status/renewal) | ✅ in `AdminTenantDetailResponse.Subscription` | ✅ on detail page | ❌ not surfaced by any tool | **partial** | `admin_tenant_dto.go:33` (`SubscriptionInfo`), `subscription.go:68` (`SubscriptionMetadata`) |
| **Cross-tenant health aggregate / scoring** | ❌ nothing | ❌ no fleet overview page | ❌ no tool | **missing** | — (this proposal) |
| Dunning / past-due list | ✅ `GET …/billing/dunning` | ✅ `/billing` page | ❌ no tool | **partial** | `main.go:637`, `admin_dunning.go:27`, `billing-api.ts:fetchDunning` |
| **Recovery actions** (fix edition, re-sync, reactivate, resolve dunning) | ⚠️ partial: suspend/unsuspend + quotas + refund only | ⚠️ suspend + edit-quotas dialogs only | ⚠️ `tenant_suspend`/`tenant_update` only | **partial** | `main.go:606-607,636`, `suspend-dialog.tsx`, `mcp_tools.ex:6237` |
| **High-risk recovery** (re-provision, restore, rotate keys, batch) | ⚠️ Core ops exist (`/operations/replay`, `/backup`) but no tenant-scoped guarded wrapper; bulk = suspend/activate only | ❌ no recovery console | ⚠️ `backup_create/restore` exist but no tenant-recovery tool | **partial/missing** | `main.go:504-508,602` (`BulkAction`), `mcp_tools.ex:5200`(backup) |
| **Incident / cascade detection** (edition trap, wrong-tenant, JWT mismatch, role-string drift) | ❌ none codified | ❌ none | ❌ none | **missing** | runbook §5/§7/§8 only (manual) |
| Admin-action audit | ✅ audit logger + Core event-sourcing | ⚠️ audit_log shown on tenant detail link only | ⚠️ `audit_log` tool reads, none write fleet actions | **partial** | `audit.go:196` (`writeEvent`), `heartbeat.go:243` (`IngestEvent`), `audit_event.go:30` |

**Verdict counts: ship-ready 0, partial 7, missing 3.** The platform is rich on per-tenant CRUD and billing, but has **zero** fleet-wide rollup, **zero** codified incident detection, and only fragmentary recovery (suspend/quotas/refund). Everything new this proposal adds is in the "missing" or "complete-the-partial" column — nothing re-implements an existing capability.

---

## 2. Tenant health model

A concrete, computable classification. Each signal names its source backend and a threshold; the rollup rule converts signals to one of four tiers.

### 2.1 Signals (name → source → threshold)

| Signal | Read from | How | Threshold / values |
|---|---|---|---|
| `last_event_age` | **Core** (the durable store) | `coreClient.GetTenantStats(id)` for counts + a `GET /api/v1/events/query?tenant_id=<id>&limit=1&order=desc` probe for the newest event's `timestamp` (CLAUDE.md "Core API"). | OK < 24h · Degraded 24h–7d · At-Risk > 7d with a paid tier · n/a for tenants that never ingested |
| `last_sync_age` | **Core** | same desc-probe but filtered to the tenant's sync source (`event_type_prefix=prime.` or the agent's stream). Mirrors the runbook Step-1 probe. | OK < 1h · Degraded 1–24h · At-Risk > 24h |
| `events_quota_pct` | **Core tenant metadata** | `QuotaMetadata{EventsUsed, EventsQuota}` (`subscription.go:101`). `-1` quota = unlimited → 0%. | OK < 80% · Degraded 80–99% · At-Risk ≥ 100% (over quota; overage must be enabled) |
| `subscription_state` | **Core tenant metadata** (LemonSqueezy-mirrored) | `SubscriptionMetadata.Status` + `HighestActiveTier(subs)` (`subscription.go:69,189`). | OK `active`/`on_trial` · Degraded `past_due` (in grace) · At-Risk `expiring ≤ 7d` (renewal countdown from `SubscriptionEndsAt`) · Critical `canceled`/`unpaid`/`expired` (and not grandfathered) |
| `grandfather_window` | **Core tenant metadata** | `SubscriptionMetadata.IsGrandfathered(now)` (`subscription.go:88`). | informational; suppresses the `subscription_state=Critical` downgrade while inside the window, flags `expiring soon` when `GrandfatherUntil ≤ 14d` |
| `edition_trap` | **Query Service** (cross-service probe) | The edition lives on QS, not CP — `apps/query-service/lib/query_service_ex/edition.ex:10` reads `ALLSOURCE_EDITION` (default `:community`). Probe QS `/health` (returns version + edition) and compare the tenant's stored `tenant_id` against what a QS read resolves. | **Critical** if QS edition is `community` while the fleet has ≥ 2 non-`community` tenants with data (the exact failure in runbook §5 #5) |
| `durability` | **Core** (`health_deep`) | The MCP `health_deep` backend call already returns `durable`, `wal_enabled/wal_entries`, `parquet_enabled/parquet_files`, `warnings` (`mcp_tools.ex:5687`). CP can read the same via `/health/core`. | Critical if `durable=false` or any `warnings` non-empty |
| `replication_lag` | **Core followers** | `cluster/status` / `cluster/members` already exposes `role` + `lag_ms` (`metrics-api.ts:ClusterMember`, `main.go:491-492`). | OK < 1s · Degraded 1–10s · At-Risk > 10s or any follower `unreachable` |
| `api_key_validity` | **Core** (auth) | Probe a representative key; **critically, assert the role string is `serviceaccount` (no underscore)** — the `service_account` drift silently 403s every key (`MEMORY.md` API-key role contract; runbook §5 #1). | Critical if the canonical key 403s or carries a malformed role string |
| `empty_read_symptom_rate` | **Query Service / proxy** | rate of authenticated reads returning 0 rows or `404 page not found` (the QS-vs-CP 404 fingerprint, runbook §8). Surfaced as a *symptom class*, never auto-classified as data loss. | Degraded if symptom rate > 0 for a tenant that has Core data (points at read-path/identity, per runbook) |

> **Why `last_event_age` / `last_sync_age` are probes, not stored counters:** `get_tenant_usage.go:54` notes per-day metrics are not yet exposed by Core (today's totals only). So recency must come from the documented `?limit=1&order=desc` query, not a precomputed field. The build phase should NOT invent a stored `last_event_at` column — it should issue the probe.

### 2.2 Rollup rule (explicit)

Compute a per-tenant tier by **worst-wins severity**, with a grandfather override:

```
tier(tenant) =
  Critical  if ANY signal is Critical
            AND NOT (the only Critical signal is subscription_state
                     AND grandfather_window is active)
  At-Risk   else if ANY signal is At-Risk
  Degraded  else if ANY signal is Degraded
  Healthy   else
```

Each tenant result carries the **list of contributing signals** (signal name + observed value + tier it triggered) so the UI/MCP can show *why* a tenant is non-green, not just the colour. Fleet rollup = counts per tier + the ordered list of the worst N tenants:

```
fleet.health = {
  total, healthy, degraded, at_risk, critical,
  worst: [ {tenant_id, tier, reasons:[{signal, value, tier}]} … ]  // sorted Critical→Degraded
}
```

This is deterministic and cheap: every signal is a read the platform already performs or a single documented probe.

---

## 3. Subscription state surface

What the operator sees per tenant and fleet-wide, mapped to its data source. All subscription fields are **Core tenant-metadata JSON** mirrored from LemonSqueezy via the webhook (`main.go:572` → `WebhookHandler.LemonSqueezy`); there is no separate billing DB.

| Field shown | Source field | Source backend |
|---|---|---|
| Tier (effective) | `HighestActiveTier(subs)` over `TenantBillingMetadata.Subscriptions` | Core metadata (`subscription.go:189`) — *highest active wins*, so a duplicate subscription bubbles up |
| Status | `SubscriptionMetadata.Status` (`active`/`past_due`/`canceled`/`trialing`/`expired`) | Core metadata (`subscription.go:73`) |
| Renewal / expiry countdown | `SubscriptionEndsAt`, `TrialEndsAt` | Core metadata (`subscription.go:77-78`); LemonSqueezy `renews_at` mirrored into `SubscriptionInfo.RenewsAt` (`admin_tenant_dto.go:40`) |
| Past-due days + dunning retry state | `Status` + `classifyRetryStatus` (`pending_retry`/`exhausted`/`manual_review`) | Core metadata, surfaced by `AdminDunningUseCase` (`admin_dunning.go:68`); next-retry/lockout timing is LemonSqueezy-driven (LS auto-retries `past_due`) |
| Overage status | `OverageMetadata{Enabled, EventsOverage, QueriesOverage, EventRate}` | Core metadata (`subscription.go:124`); x402 allowance via `QuotaMetadata.X402Allowance/X402Used` (`subscription.go:112`) |
| Plan-vs-quota mismatch | compare stored `QuotaMetadata.EventsQuota` against `QuotasForTier(tier)` (incl. the no-downgrade floor) | computed from Core metadata + `subscription.go:298` (`QuotasForTier`) — surfaces a tenant whose stored quota drifted from its tier's entitlement |
| Grandfather window | `GrandfatherUntil` + `IsGrandfathered(now)` | Core metadata (`subscription.go:83,88`) |

Fleet-wide, the aggregate adds the existing revenue/dunning rollups already computed by `AdminRevenueUseCase` (MRR/ARR/churn — `admin_billing_dto.go:38`) and `AdminDunningUseCase`. The health surface **links to** these rather than re-deriving them.

---

## 4. Recovery playbook (deep recovery, with guards)

Each action: failure mode it remediates → risk tier → preconditions → confirmation → dry-run → audit → blast-radius limit. Risk tiers: **Safe** (read-only or trivially reversible), **Guarded** (mutates one live tenant, reversible), **Destructive** (irreversible or wide blast radius).

Every mutating action writes a recovery audit event into Core using the **existing** `coreClient.IngestEvent(IngestEventRequest{EventType, EntityID, TenantID, Payload})` pattern (`heartbeat.go:243`) — event type `admin.recovery.<action>`, entity `recovery:<tenant_id>`, tenant a dedicated `admin-recovery` system tenant (mirroring `heartbeatTenant`). This keeps the recovery log durable and event-sourced, consistent with the "event-source everything" rule.

### 4.1 Safe / Guarded actions

| Action | Remediates | Risk | Preconditions | Confirm | Dry-run | Audit | Blast radius |
|---|---|---|---|---|---|---|---|
| `assess_tenant` / `fleet_health` | (diagnosis) | Safe | none | none | n/a (read) | none (read) | none |
| `force_resync` (re-trigger ingestion / reconcile) | stale `last_sync_age`; x402 counter drift | Guarded | tenant exists; a sync source configured | single click | yes — show what *would* be re-pulled (count + range) | `admin.recovery.force_resync` | one tenant |
| `reactivate` (unsuspend) | over-aggressive suspension; dunning resolved | Guarded | status `suspended` | single click | n/a | reuses existing `…/:id/unsuspend` (`main.go:607`) | one tenant |
| `suspend` | abuse / non-payment / policy | Guarded | status `active` | single click + reason | n/a | reuses `…/:id/suspend`; `tenant_suspend` already logs reason (`mcp_tools.ex:6038`) | one tenant |
| `edit_quotas` | plan-vs-quota mismatch | Guarded | tenant exists; ≥ 1 field | single click | preview old→new | reuses `PUT …/:id/quotas` (`main.go:605`) | one tenant |
| `reconcile_subscription` (re-apply `QuotasForTier`) | quota drift; retired-tier alias not mapped | Guarded | active subscription present | single click | preview computed entitlements | `admin.recovery.reconcile_subscription` | one tenant |
| `resolve_dunning` (re-issue checkout / mark for manual review / extend grace) | `past_due`/`unpaid`/`expired` drift | Guarded | tenant in dunning list | single click | preview action taken | `admin.recovery.resolve_dunning` | one tenant |

### 4.2 The headline incident fixes (codified from the runbooks)

| Action | Remediates | Risk | Guard design |
|---|---|---|---|
| `fix_edition_trap` | **edition=community trap** (runbook §5 #5) | **Destructive** (changes a fleet-wide QS setting that pins *every* request's tenant) | **Diagnose-only by default**: the action *detects* the trap (QS edition `community` + non-`community` tenants with data) and emits the exact remediation — set `ALLSOURCE_EDITION=enterprise` on `allsource-query` and confirm a **new** Fly release (runbook trap #4: a `fly deploy` that creates no release is a no-op). The actual env mutation is **operator-executed** (`fly secrets set` / deploy), not done blindly by the API — because it is a single switch that re-routes the whole fleet. The tool returns a copy-paste command + a post-change verification probe. Audit `admin.recovery.fix_edition_trap` records the detection + the recommended command. |
| `diagnose_tenant_identity` | **wrong-tenant silent auto-provision**, **JWT tenant_id mismatch** (runbook §7) | Safe (read-only) | Compares (a) the tenant the caller's JWT claims, (b) the tenant the data is stored under (Core probe), (c) what QS resolves. Returns the divergence + the fix (re-login with the account whose `TenantSlug(email)` equals the data tenant — `tenant.go:177`). **No mutation** — auto-merging tenants is explicitly out of scope (too dangerous; the runbook fix is a re-login, not a data move). |
| `check_key_role` | **API-key role-string drift** (`service_account` vs `serviceaccount`) | Safe (read) → Guarded (rotate) | Read step asserts the role string; if drifted, the Guarded `rotate_keys` action (below) re-mints with the canonical `serviceaccount`. |

### 4.3 High-risk set (Destructive) — power **with** guards

These mutate live production tenants; a fat-finger damages paying customers. Each guard is justified.

| Action | Remediates | Preconditions | Guard (and why) | Dry-run | Audit | Blast radius |
|---|---|---|---|---|---|---|
| `reprovision_tenant` | corrupt/incomplete tenant metadata; failed onboarding | tenant exists; **not** `active` with recent events | **Typed-name confirmation** — caller must type the exact `tenant_id`. *Why:* re-provisioning rewrites tenant metadata; a wrong id would clobber a healthy paying tenant. **Dry-run mandatory first** (returns the metadata diff). **Max blast radius = 1**, hard-rejected if the tenant has ingested in the last 24h (use `force_resync` instead). | required | `admin.recovery.reprovision_tenant` | 1 tenant, never `active`+recent |
| `restore_from_backup` / `replay` | data corruption; bad projection | a backup/snapshot exists (`/operations/snapshots`, `/backup` — `main.go:500-508`) | **Typed-name confirmation + dry-run** showing the snapshot id, age, and event count to be replayed. *Why:* a replay/restore can overwrite newer events; the operator must see exactly which snapshot and how far back. Wraps the **existing** Core ops (`StartReplay`, `backupHandler`) in a tenant-scoped guarded use case — does not add a new Core capability. | required | `admin.recovery.restore` | 1 tenant; targets one snapshot |
| `rotate_keys` | role-string drift; suspected key compromise | tenant exists | **Confirmation token** (a short-lived token returned by a preceding dry-run that the apply call must echo) + always re-mints the canonical `serviceaccount` role. *Why:* rotation invalidates the tenant's current keys — confirming via an echoed token proves the operator saw the dry-run's "these N keys will stop working" warning. | required | `admin.recovery.rotate_keys` | 1 tenant's keys |
| `batch_recovery` | apply one Guarded action (e.g. `reconcile_subscription`, `force_resync`) across many tenants (e.g. all retired-tier tenants — the runbook's "retired-tier backfill") | a tenant **filter** (status/tier/health-tier) | **Hard `max_tenants` cap (default 25, absolute ceiling 100)** + **dry-run mandatory** returning the full affected-tenant list + per-tenant preview, + **confirmation token** echoing the exact count. *Why:* a batch is the single most dangerous surface — an unbounded "recover everything" could mutate the whole customer base. The cap and the count-echo make the blast radius impossible to fat-finger. **Destructive sub-actions (reprovision/restore/rotate) are forbidden in batch.** | required | one `admin.recovery.batch` event + one per affected tenant | ≤ `max_tenants`, Guarded-only |

> **Guard summary rule (for the build phase):** any action that is not read-only MUST take a `dry_run: bool` and return a preview when true; any **Destructive** action MUST additionally require either a typed `confirm_tenant_id` (matching the target) or an echoed `confirm_token` from a prior dry-run, and MUST write a Core audit event before returning success. A recovery surface without these is a regression, not a feature.

---

## 5. Surface design — three backends, one model

The health model and playbook live **once** in the Control Plane (Go use cases). The Admin UI and MCP are thin consumers — no parallel scoring logic.

### 5.1 Control Plane API — only the NEW endpoints

All under the existing admin group `cp.router.Group("/api/v1/admin")` with `AdminAuthMiddleware(cp.authClient.jwtSecret)` (`main.go:592-593`). Responses use the existing HAL wrapper pattern (`HALResource{Links: …}` + payload, as in `admin_tenant_handler.go:221`). New handlers follow the Clean-Architecture shape: a `Fleet*Handler` / `Recovery*Handler` wrapping use cases, registered in the container exactly like `AdminTenantHandler`.

```
# --- Health (read) ---
GET  /api/v1/admin/fleet/health
       ?tier=critical|at_risk|degraded   (optional filter)
       &limit=N                          (worst-N, default 25)
  → 200 {
      "_links": { "self": …, "tenant_template": "/api/v1/admin/fleet/health/{id}" },
      "summary": { "total": N, "healthy": N, "degraded": N, "at_risk": N, "critical": N },
      "worst":   [ { "tenant_id", "name", "tier", "reasons":[{"signal","value","tier"}] } … ]
    }

GET  /api/v1/admin/fleet/health/:id      # single-tenant assessment (all signals + values)
  → 200 { "tenant_id", "tier", "signals":[{"signal","value","tier","source"}], "subscription": {…} }

# --- Recovery (mutating; every one supports ?dry_run=true) ---
POST /api/v1/admin/recovery/:id/resync                 # Guarded
POST /api/v1/admin/recovery/:id/reconcile-subscription # Guarded
POST /api/v1/admin/recovery/:id/resolve-dunning        # Guarded
POST /api/v1/admin/recovery/:id/rotate-keys            # Destructive (confirm_token)
POST /api/v1/admin/recovery/:id/reprovision            # Destructive (confirm_tenant_id)
POST /api/v1/admin/recovery/:id/restore                # Destructive (confirm_tenant_id + snapshot_id)
POST /api/v1/admin/recovery/batch                      # Destructive-bounded (filter + max_tenants + confirm_token)
GET  /api/v1/admin/recovery/diagnose/edition           # Safe — detect edition trap, return command
GET  /api/v1/admin/recovery/:id/diagnose-identity      # Safe — tenant/JWT/store divergence

# request body shape (mutating), consistent across actions:
{ "dry_run": false, "reason": "…", "confirm_tenant_id": "…" | "confirm_token": "…", … action-specific … }
# dry_run:true → 200 { "dry_run": true, "would": { … preview … }, "confirm_token": "…" (for token-gated actions) }
```

Reuse, not rebuild: `reactivate`/`suspend`/`edit_quotas` keep using the **existing** `…/admin/tenants/:id/{suspend,unsuspend,quotas}` routes (`main.go:605-607`) — the recovery console calls them directly; only the *new* capabilities get new endpoints. `restore`/`replay` wrap the **existing** `/api/v1/operations/*` + `/api/v1/operations/backup` ops (`main.go:500-508`) in a tenant-scoped guarded use case.

### 5.2 Admin web UI (`apps/admin`)

New pages slot into the existing `(authenticated)` route group; a new sidebar entry is added to `components/sidebar.tsx` (`navigation` array, `main.go`→`sidebar.tsx:24`). New API client file `src/lib/fleet-api.ts` follows the **exact** existing pattern (`fetch(url, { credentials: "include" })`, typed response interfaces, `getApiUrl()` from `NEXT_PUBLIC_API_URL`) seen in `tenants-api.ts`/`billing-api.ts`.

| New page / component | Route | Reuses |
|---|---|---|
| **Fleet Health overview** — 4 stat cards (Healthy/Degraded/At-Risk/Critical counts) + worst-N table of per-tenant health chips | `app/(authenticated)/fleet/page.tsx` | `StatCard` (`components/monitoring/stat-card.tsx`) for the counts; the `monitoring/page.tsx` auto-refresh + `RefreshCw` pattern verbatim; `ClusterHealth`'s green/yellow/red dot convention for the health chip |
| **Per-tenant health drill-down** — all signals with observed values + the contributing-reason list | `app/(authenticated)/fleet/[id]/page.tsx` | `usage-bar.tsx` for quota %, the `tenants/[id]/page.tsx` layout; links back to `/tenants/[id]` and `/billing` |
| **Recovery console** — per action, a dialog rendering the guard | a panel on the drill-down page + a `RecoveryDialog` component under `components/fleet/` | extends `suspend-dialog.tsx` (already has the `variant="destructive"` confirm button + `isSubmitting` state). Destructive dialogs add a **typed-name input** ("type the tenant id to confirm") and a **dry-run preview pane** (render the `would` payload before enabling the real Apply button) |
| **Health chip** | `components/fleet/health-chip.tsx` | the exact dot+colour vocabulary from `cluster-health.tsx:57-63` (`bg-green-500`/`bg-yellow-500`/`bg-red-500`) so it reads consistently with monitoring |

The recovery console renders the guards as **real UI**: dry-run is a two-step ("Preview" → shows diff → "Apply" enabled only after preview); typed-name confirmation is an input compared client-side to the target id; batch shows the affected-tenant list and the count the operator must confirm. This mirrors the existing `SuspendDialog` UX, scaled up for risk.

### 5.3 Admin MCP tools (`apps/mcp-server-elixir`)

**These go in the Elixir server, NOT `prime-mcp`** — `prime-mcp` is single-tenant (`projection_registry.rs:22`, `http.rs:294,379`); a fleet tool there would cross the tenant boundary it explicitly does not have. The Elixir server already composes from `state.control_client` (a `ControlPlaneClient`) and `state.backend` (a `CoreClient`), and already gates tenant tools behind `control_plane_enabled` — the new tools extend that exact machinery.

**Exact insertion points in `mcp_tools.ex`:**

1. **Tool definition fns** — add `defp tool_fleet_health_summary/0`, `defp tool_tenant_health_assessment/0`, and `defp tool_recovery_<action>/0` alongside the existing `defp tool_tenant_suspend` (`mcp_tools.ex:5999`). Same `%{name, description, inputSchema}` shape.
2. **Phase list in `list_tools/1`** — add a new gated block after the Phase-8 tenant block (`mcp_tools.ex:102-115`):
   ```elixir
   # Phase 8b: Fleet health & recovery — gated by control_plane_enabled (read)
   #           + a NEW system_admin distinction for mutating recovery tools.
   fleet_tools =
     if config[:control_plane_enabled] do
       base = [tool_fleet_health_summary(), tool_tenant_health_assessment()]
       if config[:system_admin] do
         base ++ [tool_recovery_resync(), tool_recovery_reconcile_subscription(),
                  tool_recovery_resolve_dunning(), tool_recovery_rotate_keys(),
                  tool_recovery_reprovision(), tool_recovery_restore(),
                  tool_recovery_batch(), tool_recovery_diagnose_edition()]
       else
         base
       end
     else
       []
     end
   ```
   …then append `fleet_tools` to the concatenation at `mcp_tools.ex:146-157`.
3. **Handler map** — add a `@fleet_tool_handlers %{ "fleet_health_summary" => :handle_fleet_health_summary, … "recovery_restore" => :handle_recovery_restore }` next to `@tenant_tool_handlers` (`mcp_tools.ex:224`), and add its `Map.get(@fleet_tool_handlers, …)` clause to `dispatch_tool/4` (`mcp_tools.ex:333`).
4. **Gate** — extend the `@control_plane_gated_tools` MapSet (`mcp_tools.ex:286`) with the two read tools, and add a **new** `@system_admin_gated_tools` MapSet for the mutating `recovery_*` tools with a matching `cond` arm in `call_tool/3` (`mcp_tools.ex:300`) that returns the "system-admin not enabled" isError when `not Map.get(state, :system_admin, false)`. `system_admin` is set in `server.ex` init (`server.ex:62-68`) from a new `ALLSOURCE_SYSTEM_ADMIN` env (off by default) and threaded into the `tools/list` config (`server.ex:134-138`).
5. **Handler impls** — `def handle_fleet_health_summary(args, state, format)` etc., composing existing client calls:
   - read tools call a **new** `ControlPlaneClient.fleet_health/2` + `fleet_tenant_health/3` (added to `control_plane_client.ex` next to `tenant_usage` — same Tesla `get("/api/v1/admin/fleet/health…")` + status-match pattern, `control_plane_client.ex:86`). **Note:** the existing tenant tools call the **non-admin** `/api/v1/tenants/*` group; the new fleet tools must call the **`/api/v1/admin/*`** group, so `ControlPlaneClient.new/0` must attach the admin JWT — a small, explicit addition.
   - mutating tools call new `ControlPlaneClient.recovery_*` fns hitting the `POST /api/v1/admin/recovery/*` endpoints, passing `dry_run` + confirmation params straight through.
   - format output via the existing `ToonEncoder.format_response(data, format)` (`mcp_tools.ex:6247`).

**inputSchema sketches** (matching the existing `tool_tenant_suspend` style at `mcp_tools.ex:6030`):

```elixir
# fleet_health_summary — read, no destructive params
inputSchema: %{ type: "object",
  properties: %{ "tier" => %{type: "string", enum: ["critical","at_risk","degraded","healthy"]},
                 "limit" => %{type: "integer", description: "worst-N, default 25"} },
  required: [] }

# tenant_health_assessment — read
inputSchema: %{ type: "object",
  properties: %{ "tenant_id" => %{type: "string"} }, required: ["tenant_id"] }

# recovery_reprovision — DESTRUCTIVE → dry_run + typed confirmation
inputSchema: %{ type: "object",
  properties: %{ "tenant_id" => %{type: "string"},
                 "dry_run" => %{type: "boolean", description: "preview only; default true"},
                 "confirm_tenant_id" => %{type: "string",
                    description: "must exactly equal tenant_id to execute (omit for dry-run)"},
                 "reason" => %{type: "string"} },
  required: ["tenant_id", "reason"] }

# recovery_batch — DESTRUCTIVE-bounded
inputSchema: %{ type: "object",
  properties: %{ "filter" => %{type: "object", description: "status/tier/health-tier selector"},
                 "action" => %{type: "string", enum: ["resync","reconcile_subscription","resolve_dunning"]},
                 "max_tenants" => %{type: "integer", description: "hard cap ≤ 100, default 25"},
                 "dry_run" => %{type: "boolean", description: "default true"},
                 "confirm_token" => %{type: "string", description: "echo the token from the dry-run"} },
  required: ["filter", "action"] }
```

The description strings should carry the same loud `**ADMIN ONLY**` / `**SAFETY WARNING**` blocks the existing `tool_tenant_suspend` uses (`mcp_tools.ex:6006-6028`), plus an explicit "dry-run runs by default; you must pass confirmation to mutate" note for Destructive tools.

---

## 6. Auth & safety model for "admin of the whole system"

### 6.1 Who the admin is, on each surface

The platform is owned by a single operator. Authorization is established by a JWT carrying `role: "admin"`, validated identically everywhere:

- **Control Plane API.** `AdminAuthMiddleware` (`admin_middleware.go:44`) extracts the Bearer token, validates the JWT against `JWT_SECRET`, and **requires `claims.Role == entities.RoleAdmin`** (`admin_middleware.go:69`) — 401 on bad token, 403 on non-admin. The fleet/recovery endpoints sit inside the same `/api/v1/admin` group, so they inherit this with zero new auth code.
- **Admin web app.** Gated by a Control-Plane JWT with `role == "admin"` in an `admin_token` cookie; `validateAdminToken` calls CP `/api/auth/me` and asserts `isAdminRole(payload)` (`apps/admin/src/lib/auth.ts:73,57`). Same role claim.
- **MCP.** Read fleet tools gated by the existing `control_plane_enabled` (env `ALLSOURCE_CONTROL_URL`); **mutating recovery tools gated by a NEW `system_admin` flag** (env `ALLSOURCE_SYSTEM_ADMIN`, off by default). This is the requested "system-admin distinction": an MCP client merely *connected* to the control plane can read health, but cannot run a Destructive recovery unless the operator explicitly enabled system-admin mode on that server instance.

### 6.2 The ADMIN_EMAILS reconciliation (a real-code note)

`MEMORY.md` (`project_auth_architecture`) states the Control Plane uses an **`ADMIN_EMAILS` allowlist** to grant the admin role, and that "Control Plane owns auth, not the Rust auth service." The middleware enforces the **role claim** (`role == "admin"`), not the email directly — i.e. the allowlist is applied **upstream at login/token-mint** (where the JWT's `role` is set), and the middleware is the **downstream enforcement** of the resulting claim.

**CONFIRMED (build pass):** the mint site is **`apps/control-plane/auth.go` `roleForEmail()`** — it reads `ADMIN_EMAILS` (comma-separated, matched **case-insensitively**) and maps a matching email to **`RoleAdmin`** at token-mint; every non-matching email mints as a non-admin role. The admin middleware (`admin_middleware.go:69`) then enforces **only** the resulting `role == "admin"` claim — it never re-reads `ADMIN_EMAILS`. So there is exactly one allowlist, applied once at mint: whoever `ADMIN_EMAILS` admits is precisely who passes the `/api/v1/admin` gate (and therefore who can run fleet/recovery), with **no separate allowlist to maintain** on the fleet surface.

> *Contradiction-with-critical_context note (resolved):* the critical_context asked to "reconcile with any ADMIN_EMAILS allowlist you found." There is **no** `ADMIN_EMAILS` read in `admin_middleware.go` — the gate there is purely the role claim — and that is correct by design: the allowlist lives upstream at `auth.go roleForEmail()` (the mint site, now confirmed), not in the middleware. This is consistent, not conflicting: one allowlist, applied at mint, enforced as a claim downstream.

### 6.3 Cross-cutting safety rules (apply to all three surfaces)

1. **Every mutating recovery action is audited as a Core event** via the existing `IngestEvent` pattern (`heartbeat.go:243`) — durable, event-sourced, queryable. Never a fire-and-forget log line only.
2. **Dry-run is universal.** Every non-read action accepts `dry_run` and returns a preview; the default for Destructive actions is dry-run **on**.
3. **Destructive ops require explicit confirmation** — typed-name (`confirm_tenant_id`) or echoed `confirm_token` from a preceding dry-run — enforced server-side in the use case (not just client-side), so the MCP and a raw curl are bound by the same guard the UI renders.
4. **Blast-radius caps are server-enforced** — `batch` rejects > `max_tenants`; single-tenant Destructive actions hard-reject when preconditions (e.g. "active + ingested in last 24h" for reprovision) aren't met.

---

## 7. Phased build plan

Ordered by dependency: the Control Plane API is the spine; UI and MCP parallelize once it exists.

### P0 — Health model + read API (Control Plane)
- **Scope:** the signal-computation use cases + `GET /api/v1/admin/fleet/health` and `…/fleet/health/:id`; the edition/identity diagnose endpoints (read-only).
- **Files:** new `internal/application/usecases/fleet_health.go` (+ a `signals` sub-package); new `internal/interfaces/http/fleet_health_handler.go`; wire handler in the container + `main.go` admin group (after line 607); reuse `coreClient.GetTenantStats`, `tenantRepo.FindAll`, the `events/query?order=desc` probe, and a QS `/health` probe for edition.
- **Acceptance:** `GET /api/v1/admin/fleet/health` returns the four counts + worst-N with per-tenant `reasons`; a tenant with `EventsUsed ≥ EventsQuota` shows `events_quota_pct` At-Risk; with QS on `community` + ≥2 non-`community` data tenants, the diagnose endpoint flags the edition trap and returns the `ALLSOURCE_EDITION=enterprise` command; all endpoints 403 without an admin JWT (proves `AdminAuthMiddleware` reuse).

### P1 — Recovery API with guards (Control Plane)
- **Scope:** Guarded actions (`resync`, `reconcile-subscription`, `resolve-dunning`) + Destructive (`rotate-keys`, `reprovision`, `restore`, `batch`); `dry_run`, `confirm_tenant_id`/`confirm_token`, blast-radius caps; Core audit events.
- **Files:** new `internal/application/usecases/recovery_*.go`; new `internal/interfaces/http/recovery_handler.go`; a small `recovery_audit.go` wrapping `IngestEvent`; register routes in `main.go` admin group; `restore` wraps existing `OperationsHandler.StartReplay` + `backupHandler`.
- **Acceptance:** every mutating endpoint with `?dry_run=true` returns a `would` preview and **no** state change + (for token-gated) a `confirm_token`; `reprovision` without a matching `confirm_tenant_id` returns 400 and does nothing; `reprovision` on an `active` tenant with recent events is hard-rejected; `batch` with > 100 tenants in the filter is capped/rejected; each successful apply writes one `admin.recovery.*` event to Core (assert via an events query).

### P2 — Admin web UI (parallel with P3)
- **Scope:** Fleet overview page, per-tenant drill-down, recovery console with rendered guards; new `src/lib/fleet-api.ts`; sidebar entry.
- **Files:** `app/(authenticated)/fleet/page.tsx`, `app/(authenticated)/fleet/[id]/page.tsx`, `src/lib/fleet-api.ts`, `components/fleet/health-chip.tsx`, `components/fleet/recovery-dialog.tsx`; edit `components/sidebar.tsx` `navigation`.
- **Acceptance:** overview renders the four `StatCard` counts + a worst-N table with red/yellow/green chips matching `cluster-health.tsx` colours; drill-down lists every signal + value; a Destructive recovery dialog keeps Apply disabled until the typed tenant id matches AND a dry-run preview has been shown; the client uses `credentials: "include"` + `NEXT_PUBLIC_API_URL` exactly like `tenants-api.ts`.

### P3 — Admin MCP tools (parallel with P2)
- **Scope:** `fleet_health_summary`, `tenant_health_assessment` (read) + `recovery_*` (mutating, system-admin-gated); `ControlPlaneClient` additions hitting `/api/v1/admin/*` with the admin JWT; the `system_admin` gate + `ALLSOURCE_SYSTEM_ADMIN` env.
- **Files:** `lib/mcp_server_elixir/protocol/mcp_tools.ex` (tool fns + handler map + gate + handlers at the insertion points in §5.3), `lib/mcp_server_elixir/infrastructure/control_plane_client.ex` (new `fleet_*`/`recovery_*` fns + admin-JWT attach), `lib/mcp_server_elixir/server.ex` (read `ALLSOURCE_SYSTEM_ADMIN`, thread `system_admin` into the `tools/list` config).
- **Acceptance:** with `ALLSOURCE_CONTROL_URL` set but `ALLSOURCE_SYSTEM_ADMIN` unset, `tools/list` shows the two read tools and **omits** every `recovery_*`; calling a `recovery_*` tool returns the "system-admin not enabled" isError; with `ALLSOURCE_SYSTEM_ADMIN=true`, `recovery_reprovision` without `confirm_tenant_id` returns the dry-run preview and does not mutate; the tools are **defined in mcp-server-elixir and absent from prime-mcp** (grep proves it).

### P4 — Polish & runbook closure (in progress)
- **Scope:** wire the `fleet/health` summary into the existing admin auto-refresh cadence; add a `data-testid` surface for proofshot; cross-link the two runbooks to the new endpoints/tools; document the `ADMIN_EMAILS` → `role` mint site once confirmed.
- **Acceptance:** `DIAGNOSING_PIPELINE_DATA_VISIBILITY.md` §5/§7/§8 incidents each map to a named tool/endpoint; `PRICING_BILLING_CUTOVER.md` "retired-tier backfill" maps to `recovery_batch`+`reconcile_subscription`.
- **Status:** auto-refresh landed in P2 (30s cadence, `monitoring/page.tsx` pattern verbatim — verified, not re-added); `data-testid` surface present on the fleet pages + recovery dialog; both runbooks cross-linked (§5/§7/§8 + retired-tier); the mint site is **confirmed** (§6.2 — `auth.go roleForEmail()`); operator surface catalog at [`docs/runbooks/FLEET_HEALTH_RECOVERY.md`](../runbooks/FLEET_HEALTH_RECOVERY.md). **Remaining:** deploy control-plane + MCP (Fly) + the admin app, then remove the fixture fallback once the endpoints answer live.

---

## 8. Recommended packaging

**Split into three sequential-then-parallel build prompts, not one combined prompt.** Rationale: the three surfaces are in three languages (Go / TypeScript / Elixir), touch isolated apps (the monorepo's isolation rule forbids cross-app coupling anyway), and have a hard dependency edge (UI + MCP both need the API first). One mega-prompt would be a 3-language, 3-app change with weak verifiability; the split gives each prompt a single language, a single app, and crisp acceptance criteria. P0 and P1 can be one prompt (same app, same language, P1 depends on P0) or two; P2 and P3 are independent and can run concurrently after the API lands.

**Follow-on `/create-prompt` invocations to run after approving this design:**

1. `/create-prompt` — *"Control Plane fleet health + recovery API (P0+P1)"* — build the Go use cases, handlers, routes, guards, and Core audit events per §2/§4/§5.1 and the P0/P1 acceptance criteria. Source of truth: this doc.
2. `/create-prompt` — *"Admin web fleet health + recovery console (P2)"* — build the `apps/admin` pages, `fleet-api.ts`, and guarded dialogs per §5.2 and P2 acceptance criteria. Depends on prompt 1's endpoints.
3. `/create-prompt` — *"Elixir MCP fleet health + recovery tools (P3)"* — build the `mcp_tools.ex` tools/handlers/gate, `control_plane_client.ex` fns, and `server.ex` `system_admin` wiring per §5.3 and P3 acceptance criteria. Depends on prompt 1's endpoints. **Explicitly: mcp-server-elixir, not prime-mcp.**

(Optionally a 4th `/create-prompt` for P4 polish + runbook cross-linking once 1–3 land.)

---

## Appendix — grounding (every existing-code claim cites a file actually opened)

- Admin route group + middleware wiring: `apps/control-plane/main.go:592-639`.
- Admin gate (role claim): `apps/control-plane/internal/interfaces/http/admin_middleware.go:44,69`.
- Tenant entity (status enum, quotas, slug): `apps/control-plane/internal/domain/entities/tenant.go:31,70,177`.
- Subscription/quota/overage metadata + tier resolution + grandfather: `apps/control-plane/internal/domain/entities/subscription.go:68,88,101,124,189,298`.
- Tenants live in Core, no dual-write: `apps/control-plane/internal/infrastructure/persistence/core_tenant_repository.go:12-14`.
- Admin tenant handlers (list/detail/usage/quotas/suspend/bulk): `apps/control-plane/internal/interfaces/http/admin_tenant_handler.go:91,134,160,208,228,248`.
- Admin billing/dunning: `apps/control-plane/internal/interfaces/http/admin_billing_handler.go:190`; `apps/control-plane/internal/application/usecases/billing/admin_dunning.go:27,68`.
- Usage event-count source + no per-day metrics yet: `apps/control-plane/internal/application/usecases/get_tenant_usage.go:42,54`.
- Core event-sourcing pattern for audit/recovery writes: `apps/control-plane/heartbeat.go:243`; `apps/control-plane/audit.go:196`; `apps/control-plane/internal/domain/entities/audit_event.go:30`.
- Admin web client pattern + auth: `apps/admin/src/lib/tenants-api.ts`, `billing-api.ts`, `metrics-api.ts`, `auth.ts:57,73`.
- Admin web components/pages: `apps/admin/src/app/(authenticated)/monitoring/page.tsx`, `components/monitoring/{stat-card,cluster-health,metrics-chart}.tsx`, `components/tenants/{suspend-dialog,usage-bar,edit-quotas-dialog}.tsx`, `components/sidebar.tsx:24`.
- MCP tool/handler/gate machinery + insertion points: `apps/mcp-server-elixir/lib/mcp_server_elixir/protocol/mcp_tools.ex:22,102,160,224,286,300,333,5687,5999,6237`.
- MCP control-plane client (tenant calls hit `/api/v1/tenants/*`, not admin): `apps/mcp-server-elixir/lib/mcp_server_elixir/infrastructure/control_plane_client.ex:18,58,86,124`.
- MCP state init (`control_plane_enabled` from `ALLSOURCE_CONTROL_URL`): `apps/mcp-server-elixir/lib/mcp_server_elixir/server.ex:44-68,134-138`.
- Edition trap is a Query Service setting (default community): `apps/query-service/lib/query_service_ex/edition.ex:10`.
- prime-mcp is single-tenant by design (excluded as fleet-tool host): `apps/prime-mcp/src/projection_registry.rs:22`; `apps/prime-mcp/src/http.rs:294,379`.
