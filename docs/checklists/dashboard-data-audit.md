# Dashboard Data Audit — hosted AllSource dashboard

**Tenant audited:** `decebal-dobrica-at-gmail-com` (tier `studio`, status `active`)
**Data backend:** Query Service (`allsource-query.fly.dev`, v0.22.0) — the dashboard's real read path
**Method:** each card probed live with the Debug service-account JWT as `Authorization: Bearer …`
against the QS (the same transport the browser proxy uses). Date: 2026-06-22.

Legend: ✅ real · ⚠️ wrong-source (reads the 0 billing meter) · 🅳 demo/placeholder · ❌ endpoint
missing/empty · 🟡 honest-empty (real endpoint, empty for this tenant).

---

## Ground-truth backend values (this tenant, via QS Debug Bearer)

| Endpoint | Result | Tenant-scoped? |
|---|---|---|
| `GET /api/tenant` | `{tier: studio, events_used: 0, events_quota: 5_000_000, queries_used: 0, queries_quota: 500_000}` | yes |
| `GET /api/tenant/usage` | `events.used: 0`, `queries.used: 0` | yes — **the broken meter** |
| `GET /api/streams` | `total: 7735` | yes ✅ |
| `GET /api/event-types` | `total: 144` | yes ✅ |
| `GET /api/projections` | `count: 2` (both `running`) | yes ✅ |
| `GET /api/events?limit=N` | real rows; `count` = page size only (no tenant total) | yes |
| `GET /api/tenants/me/analytics?range=30d` | real `ingestion_rate[]`, `event_type_distribution[]`, `top_entity_ids[]` | yes ✅ |
| `GET /api/tenant/audit-logs` | real `entries[]` | yes ✅ |
| `GET /api/team/members` | `{members:[owner], seat_limit:1, seats_used:1}` | yes ✅ |
| `GET /api/api-keys` | `{count: 5, keys:[…]}` (field is `keys`, not `data`) | yes ✅ |
| `GET /api/replay` | `{count:0, total:0, data:[]}` | yes 🟡 (empty) |
| `GET /api/webhooks` | `{count:0, total:0, data:[]}` | yes 🟡 (empty) |
| `GET /api/v1/prime/graph` | real `{nodes, edges, stats}` | yes ✅ |
| `GET /api/metrics` → `.backend` | **Prometheus TEXT blob**, not JSON. Contains `allsource_storage_events_total` (GLOBAL, all tenants) and event-type counters. **No p99/latency percentile, no per-tenant total.** | ❌ not tenant-scoped |
| `GET /api/billing/status` | **HTTP 500** (Core proxy error) | broken endpoint |
| `GET /api/schemas` | **HTTP 500** | broken endpoint |
| `GET /api/analytics/summary` | `econnrefused` (Core analytics down) | broken endpoint |

**Critical fact:** there is **no tenant-scoped total-event-count endpoint**. `allsource_storage_events_total`
(869,992) in `/api/metrics` is the whole Core process across **all** tenants — using it as one tenant's
"Total Events" would be dishonest. The truthful tenant-scoped magnitudes available are **Streams (7,735)**,
**Event Types (144)**, and the real **30-day ingestion series**.

---

## Per-view truth table

### Overview — `/dashboard` (`apps/web/src/app/dashboard/page.tsx`)

| Card / element | Source (before) | Rendered (before) | Real value | Verdict → Fix |
|---|---|---|---|---|
| "Total Events" stat card (`stats-cards.tsx:99`) | `stats.events.used` ← `/api/tenant/usage` | **0** of 5,000,000 | store is full | ⚠️ → reads real tenant-scoped activity (streams/event-types/ingestion), label clarified |
| "Queries Executed" stat card (`stats-cards.tsx:106`) | `stats.queries.used` ← usage meter | 0 | no query metering exists | ⚠️ → honest: meter not yet wired (see Phase B note) |
| "Active Projections" (`stats-cards.tsx:112`) | `stats.projections.active` ← `/api/projections` | 2 | 2 | ✅ already real |
| "p99 Latency" (`stats-cards.tsx:119`) | `backend.p99_latency_us` ← metrics blob (a string) | "—" | no real source exists | ✅ honest "—" (kept; no fake number) |
| "Current Plan" Events bar (`page.tsx:147`) | `stats.events.used / quota` ← usage meter | 0 / 5.0M | quota bar (period usage) | ✅ correct source for a *quota* bar; reads real after Phase B backfill |
| "Current Plan" Queries bar (`page.tsx:171`) | usage meter | 0 / 500K | quota bar | ✅ correct source; honest-empty until query metering |
| "Event Ingestion (30 days)" chart (`page.tsx:199`) | `UsageChart` with **no `history`** | "No usage data available" | real ingestion series exists | ⚠️/🅳 → plots real daily series from `/api/tenants/me/analytics` |
| "Query Usage (30 days)" chart (`page.tsx:200`) | `UsageChart` no `history` | "No usage data available" | no query time-series exists | 🅳 → explicit "query metering starts soon" honest empty |
| "Live Performance" events/sec, p99, throughput (`live-metrics.tsx`) | `backend.events_per_second` etc. ← metrics blob string | "—" | not in the blob as JSON | ✅ honest "—" (kept) |
| "Your Event Store" banner: total events (`page.tsx:265`) | `stats.events.used` | **0** | store is full | ⚠️ → real tenant-scoped event count |
| "Your Event Store" banner: storage (`page.tsx:278`) | `backend.storage_bytes` ← blob | "—" | `allsource_storage_size_bytes 0` (replica reports 0) | ✅ honest "—" (kept) |

### Other tabs

| Route | Source | Verdict | Note |
|---|---|---|---|
| `/dashboard/events` | `useEvents` → `/api/events` | ✅ real | honest "No events yet" empty state; unwrap applied |
| `/dashboard/analytics` | `useUsageAnalytics` → `/api/tenants/me/analytics` | ✅ real | ingestion/type-dist/top-entities all real, honest empties |
| `/dashboard/memory` | `useEvents(prime.*)` + `useGraph` → `/api/v1/prime/graph` | ✅ real | honest "sync not configured" empty; unwrap applied |
| `/dashboard/pipelines` | `listProjections` → `/api/projections` | ✅ real | maps 2 real projections; `eventsProcessed` is hardcoded `0` (no per-projection count in API) — left, not fabricated |
| `/dashboard/settings` | event-sourced prefs + auth store | ✅ real | notification prefs genuinely event-sourced; tenant id falls back to JWT claim |
| `/dashboard/team` | `useTeamMembers` → `/api/team/members` | ✅ real | reads `{members, seat_limit, seats_used}` |
| `/dashboard/billing` | `useDashboardStats` + `/api/billing/catalog` | ✅ real | does NOT hit the 500ing `/api/billing/status`; quota bars read meter (honest-empty until Phase B) |
| `/dashboard/api-keys` | `useApiKeys` → `/api/api-keys` | ✅ real | special-cases the `keys` field; 5 real keys |
| `/dashboard/tools/replay` | `useReplays` → `/api/replay` | 🟡 honest-empty | unwrap applied; "No replays yet" |
| `/dashboard/demo` | real seed + `Math.random` simulations | 🅳 by design | explicit Demo Zone; labeled simulations, gated until seeded |

### Banners

| Banner | File | Verdict |
|---|---|---|
| "Early Access — … Some features use demo data." | `early-access-banner.tsx:36` | ⚠️ stale copy → drop the "demo data" clause (production surfaces are real now) |
| "Demo Account — Data resets daily." | `demo-banner.tsx:10` | ✅ correctly gated on `tenant?.is_demo` |

---

## Root cause of the zeros (frontend)

The Overview was the only surface reading the **billing usage meter** (`/api/tenant/usage` →
`events_used`/`queries_used`) for labels that read like **store totals**. That meter is `0` for this
tenant because its data was synced via Prime → Control-Plane → Core, bypassing the Query Service write
path that increments usage. The fix points the *total/activity* numbers at the real, tenant-scoped event
store (streams/event-types/ingestion via the QS), and keeps the *quota bars* on the meter (which is the
correct source for "used this billing period").

## Root cause of the zeros (backend) — see Phase B

`events_used`/`queries_used` live in **Core's per-tenant `metadata.quotas` JSON**, read by the QS via
`RustCoreClient.get_tenant`. Forward metering is wired on the QS side
(`EventController.create` → `UsageReporter.record` → `POST /api/v1/tenants/{id}/usage/increment`) **but
Core has no `…/usage/increment` route** (`apps/core/.../api_v1.rs:201-219`) → the increment 404s and is
dropped. So even QS-path writes don't move the meter today. Verified live: writing a probe event through
the QS left `events_used` at 0 after the flush window.

## Phase B — what shipped (Control Plane)

1. **Stop zeroing usage on every tier apply.** `UpdateSubscriptionMetadataUseCase.applyLocked`
   (`apps/control-plane/internal/application/usecases/update_subscription_metadata.go`) used to rebuild
   the `quotas` map from tier limits alone, silently resetting `events_used`/`queries_used`/`x402_used`/
   `reset_date` to 0 on every webhook, change-plan, and 15-min scheduler tick. It now **carries the usage
   counters forward** when refreshing the tier limits. Regression test:
   `update_subscription_metadata_test.go` → "preserves usage counters across a tier apply".

2. **Backfill `events_used` from the real store.** New `BackfillEventsUsedUseCase`
   (`apps/control-plane/internal/application/usecases/billing/backfill_events_used.go`) counts a tenant's
   real events in Core (tenant-scoped `QueryEvents`, paged) and writes the count into
   `metadata.quotas.events_used` — the field the dashboard reads. Idempotent, `dry_run` supported, page-
   capped with an honest `Capped`/lower-bound flag, audit-logged (`billing.events_used.backfilled`).
   Exposed at **`POST /api/v1/admin/billing/backfill-usage`** (admin-scoped;
   `apps/control-plane/internal/interfaces/http/backfill_usage_handler.go`,
   `apps/control-plane/main.go`). Tests in `backfill_events_used_test.go`.

**Run after deploy** (single tenant, dry-run first):

```
curl -X POST https://api.all-source.xyz/api/v1/admin/billing/backfill-usage \
  -H "Authorization: Bearer <ADMIN_JWT>" -H 'Content-Type: application/json' \
  -d '{"tenant_id":"decebal-dobrica-at-gmail-com","dry_run":true}'
# then re-run with "dry_run":false
```

### Remaining limitation (honest, surfaced)

True **forward** metering still needs a Core change: the QS `UsageReporter` already POSTs to
`POST /api/v1/tenants/{id}/usage/increment`, but **Core does not register that route** (`api_v1.rs`), so
those increments 404 and drop. Until Core adds the increment handler (a separate, Core-side change), the
honest stance is: the dashboard's **Total Events / Streams / Event Types / 30-day ingestion** read the
**real event store** (Phase A — always truthful, never the meter); the **quota bars** read
`events_used`, which is now (a) backfillable to the real count and (b) no longer zeroed on tier changes,
but is not yet auto-incrementing on new ingest. The Overview no longer shows 0 where the store is full,
and never invents a number.
