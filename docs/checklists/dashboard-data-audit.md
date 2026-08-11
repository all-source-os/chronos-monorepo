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
| `GET /api/metrics` → `.backend` | **Now a STRUCTURED map** (was a raw Prometheus TEXT blob). The QS controller parses Core's exposition into `{p99_latency_us, p99_latency_ms, storage_bytes, storage_events_total, parquet_files_total, wal_segments_total, scope:"platform", raw}`. p99 is `histogram_quantile(0.99)` over `allsource_query_duration_seconds` aggregated across all `query_type`s. **All values are PLATFORM-wide** (all tenants, reset-on-restart) — labelled `scope: "platform"`, never presented as tenant numbers. (029) | ✅ platform-scoped (labelled) |
| `GET /api/billing/status` | ~~**HTTP 500** (Core proxy error)~~ → real derived state. The 500 was NOT a Core proxy error: the route sat on the `:api` pipeline, so `assigns.tenant_id` was always nil and the Core client's `is_binary(tenant_id)` guard raised. Now on `:authenticated`, with the action failing closed (401) when there is no tenant context. | ✅ (guarded by `billing_status_route_test.exs`) |
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
| "p99 Latency" (`stats-cards.tsx:119`) | `backend.p99_latency_us` ← metrics blob (a string) | "—" | real, in Core's query histogram | ✅ **REAL (029)** — QS parses Core's `allsource_query_duration_seconds` histogram and returns `backend.p99_latency_us = histogram_quantile(0.99)` aggregated across all `query_type`s (≈22.9 ms live). **PLATFORM** metric (all tenants, reset-on-restart) — card labelled "Platform query latency", banner labelled "p99 latency (platform)", never "your". Honest "—" only when no queries observed yet. |
| "Current Plan" Events bar (`page.tsx:147`) | `stats.events.used / quota` ← usage meter | 0 / 5.0M | quota bar (period usage) | ✅ correct source for a *quota* bar; reads real after Phase B backfill |
| "Current Plan" Queries bar (`page.tsx:171`) | usage meter | 0 / 500K | quota bar | ✅ correct source; honest-empty until query metering |
| "Event Ingestion (30 days)" chart (`page.tsx:199`) | `UsageChart` with **no `history`** | "No usage data available" | real ingestion series exists | ⚠️/🅳 → plots real daily series from `/api/tenants/me/analytics` |
| "Query Usage (30 days)" chart (`page.tsx:200`) | `UsageChart` no `history` | "No usage data available" | no per-tenant query *time-series* source exists | ✅ **wired REAL (029)** — chart now consumes a per-tenant `query_rate` daily series from `/api/tenants/me/analytics` (mirrors `ingestion_rate`). **TENANT**-scoped. Reads are not event-sourced today (a query only bumps the monotonic `queries_used` counter — no timestamp), so the series is **honestly empty** now and the chart shows "No query activity recorded in the last 30 days" instead of a fake trend; it fills in automatically once query events are recorded under `query.`/`audit.query`/`read.` (see `@query_event_prefixes`). The chart's scalar `used` stays the tenant `queries_used` (028). |
| "Live Performance" events/sec, p99, throughput (`live-metrics.tsx`) | `backend.events_per_second` etc. ← metrics blob string | "—" | partially in the structured backend now | 🟡 out of 029 scope — 029 structured `backend.p99_latency_us`/`storage_bytes`, but this component reads other keys (`events_per_second`, throughput) not yet emitted; honest "—" kept for those. p99 here can be pointed at the new field in a follow-up. |
| "Your Event Store" banner: total events (`page.tsx:265`) | `stats.events.used` | **0** | store is full | ⚠️ → real tenant-scoped event count |
| "Your Event Store" banner: storage (`page.tsx:278`) | `backend.storage_bytes` ← blob | "—" | real on-disk bytes (Core gauge) | ✅ **REAL (029, Phase B)** — Core now populates `allsource_storage_size_bytes` from real on-disk bytes (Parquet file sizes + WAL segment bytes, refreshed at boot + every checkpoint), surfaced via QS as `backend.storage_bytes`. **PLATFORM** figure (whole data dir, all tenants) — banner labelled "storage on disk (platform)", never this tenant's. Honest "—" until the Core deploy ships the gauge population. |

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

---

## 029 — last not-real Overview metrics (p99 / 30-day queries / storage)

Finishing pass. Every Overview card/chart now renders real data or an honest empty state, with
platform-vs-tenant metrics labelled explicitly. (029's "Phase A" = frontend + Query Service, ships via
Vercel + a QS Fly deploy; "Phase B" = Core, ships via a Core Fly deploy — distinct from the 028
"Phase B" Control-Plane work above.)

### Phase A — Query Service + frontend

1. **p99 Latency — REAL, PLATFORM.** The QS `/api/metrics` controller
   (`apps/query-service/lib/query_service_ex_web/controllers/metrics_controller.ex`) now fetches Core's
   **raw** Prometheus text (`RustCoreClient.get_metrics_raw/0`, not the JSON-mangling `get_metrics/0`)
   and parses it into a typed `backend` map via the new
   `QueryServiceEx.CoreBackendMetrics` (`apps/query-service/lib/query_service_ex/core_backend_metrics.ex`).
   p99 is computed by the new `PrometheusParser.histogram_quantile/4`
   (`apps/query-service/lib/query_service_ex/prometheus_parser.ex`) — it **aggregates the
   `allsource_query_duration_seconds_bucket` counts across every `query_type`** (`sum by (le)`), then
   linear-interpolates the 0.99 quantile. The web hook's existing `backend.p99_latency_us` read now
   resolves (`apps/web/src/hooks/use-dashboard-stats.ts`). Live value ≈ **22.9 ms**.
   _Parsed once in the QS_ (not the dashboard) so the value is reusable and the raw text is preserved
   under `backend.raw`.

2. **30-day Query Usage chart — wired REAL, TENANT, honest-empty today.** The QS analytics controller
   (`apps/query-service/lib/query_service_ex_web/controllers/usage_analytics_controller.ex`) emits a new
   per-tenant `query_rate` daily series (same bucketing as `ingestion_rate`), surfaced through the web
   client type (`apps/web/src/lib/api/client.ts`), the dashboard-stats hook, and the
   `UsageChart` on the Overview (`apps/web/src/app/dashboard/page.tsx`). Reads aren't event-sourced, so
   `query_rate` is `[]` now (honest empty state, no fake trend) and starts reporting automatically once
   query events appear under `query.`/`audit.query`/`read.`.

### Phase B — Core (`apps/core`)

3. **Storage size — REAL, PLATFORM.** Core now populates `allsource_storage_size_bytes`,
   `allsource_parquet_files_total`, and `allsource_wal_segments_total` from the **real on-disk
   footprint**: `EventStore::refresh_storage_metrics` (`apps/core/src/store.rs`) sums
   `ParquetStorage::stats().total_size_bytes` + the new `WriteAheadLog::on_disk_stats()`
   (`apps/core/src/infrastructure/persistence/wal.rs`, walks `wal-*.log` segments). Called once at boot
   (`apps/core/src/main.rs`) and at the end of every checkpoint (default 60 s). Surfaced to the dashboard
   through QS `backend.storage_bytes`.

### Platform-vs-tenant labelling decisions (029)

- **p99 latency = PLATFORM.** Core's query histogram is process-global and reset-on-restart; it's a
  system property, not one tenant's. Shown, but labelled "Platform query latency" / "p99 latency
  (platform)" — never "your latency".
- **Storage = PLATFORM.** `allsource_storage_size_bytes` is the whole data directory on disk (every
  tenant's Parquet + WAL). Labelled "storage on disk (platform)". A genuine per-tenant storage figure
  would need per-tenant byte accounting in Core — not built; not faked.
- **30-day queries = TENANT,** but no honest per-tenant query *time-series* source exists yet (a query
  only bumps the monotonic `queries_used` counter — no timestamp). So the series is honestly empty and
  the chart says so, rather than reusing the global, all-tenant `allsource_query_duration_seconds_count`
  (which would be a cross-tenant lie). The chart's scalar `used` stays the tenant `queries_used` (028).
- `allsource_storage_events_total` (process-global) is exposed in the structured `backend` for
  observability but is **deliberately not** used as a tenant "total events" number anywhere.

### Builds verified green (029)

- `bun run type-check` (apps/web): exit 0.
- `mix compile --warnings-as-errors` (QS): ok (120 files).
- QS tests: `prometheus_parser_test.exs` + `core_backend_metrics_test.exs` → 26 tests, 0 failures;
  `metrics_controller_test.exs` → 5 tests, 0 failures.
- `cargo build -p allsource-core --features server`: Finished.
- Core tests: new `test_storage_size_gauge_populated_from_on_disk_bytes` + `test_wal_on_disk_stats`
  pass; wal/storage/metrics/checkpoint modules → 0 failures.

### Deploy commands the user must run (029 does NOT deploy)

```bash
# Phase A — Query Service (parses Core metrics into structured backend + query_rate series)
fly deploy --config apps/query-service/fly.toml

# Phase B — Core (populates allsource_storage_size_bytes from real on-disk bytes)
fly deploy --config apps/core/fly.toml
```

Web ships via Vercel on `git push origin main` (no web Fly app). Until the **Core** deploy lands,
`backend.storage_bytes` stays 0 and the storage card honestly renders "—". Until the **QS** deploy lands,
`backend.p99_latency_us` is absent and the p99 card honestly renders "—".
