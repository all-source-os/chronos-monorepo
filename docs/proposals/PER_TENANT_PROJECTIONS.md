# Per-Tenant Projections (design)

**Epic:** t-822210 · **Symptom that triggered it:** t-7f9600 (dashboard "Active
Projections" shows the global engine registry as the tenant's).

> **Architecture note (2026-06-26 re-scope):** This feature was originally
> designed Core-centric — per-tenant projection compute + state inside Core's
> Rust projection engine, on the ingest hot path. That was wrong (see
> [Why not Core](#why-not-core) below). The model is now **Query-Service-centric**:
> QS computes per-tenant read-models over the tenant's (already isolated) event
> stream; Core's only added responsibility is storing the *enabled set* as
> opaque tenant metadata. The Core projection engine is **not** touched.

## Problem

The dashboard's "Active Projections" counts Core's **global projection
registry** — `EventStore.register_projection` adds an `Arc<dyn Projection>` that
processes *every* tenant's events; `list_projections()` returns
`Vec<(String, Arc<dyn Projection>)>` with no tenant dimension. The Query Service
then passes those straight through (`ProjectionController.index` +
`RustCoreClient.list_projections` drop the tenant). So the dashboard shows
platform engine views (`entity_snapshots`, `event_counters`, Prime's graph/vector
projections, embedded demo sagas/portfolios/trades) as "your projections" —
wrong, and a cross-tenant leak in the same class as the streams/event-types
scoping already fixed.

A normal tenant's honest projection count today is **0** (they own none of the
global engine projections). We want projections to be a real per-tenant concept —
and the place to build that is the **Query Service**, the layer that already
folds Core's event stream into read-models and already terminates the tenant.

## Non-goal (v1)

**Users do not author projection code.** Letting tenants ship reducer logic
(Rust/Wasm/DSL) is a much larger, security-sensitive surface. v1 is
**enablement-based**: QS ships a catalog of projection *templates*; a tenant
enables the ones it wants; QS maintains that tenant's own folded state.

## Where things live (the boundary)

```
┌──────────────────────────────────────────────────────────────────┐
│ Query Service (Elixir, port 3902) — owns the per-tenant feature    │
│                                                                    │
│  • Template catalog        (curated generic templates)             │
│  • Enablement API          (enable/disable/list/state, tenant-     │
│                             scoped, fail-closed)                   │
│  • Per-tenant folding       ProjectionSync, ETS keyed by           │
│                             (tenant_id, projection, entity)        │
│  • Background backfill      building → ready                       │
│  • Quota check surface      tenant.can_create_projection()         │
│  • Reads enabled set + reads the tenant's event stream from Core   │
│  • Writes the enabled set into Core tenant metadata                │
└──────────────────────────────────────────────────────────────────┘
                 │ reads tenant-scoped event stream (WS + query)
                 │ reads/writes metadata.projections.enabled
                 ▼
┌──────────────────────────────────────────────────────────────────┐
│ Core (Rust, port 3900) — the durable event store. UNCHANGED.       │
│                                                                    │
│  • Durable event log (WAL + Parquet + DashMap), tenant-scoped      │
│    queries + WS subscription (fail-closed) — already shipped       │
│  • Tenant metadata holds the enabled set as an OPAQUE JSON blob:   │
│    metadata.projections.enabled = ["event-count", ...]             │
│  • The GLOBAL projection engine (Projection trait,                 │
│    ProjectionManager, process_event on ingest) — internal          │
│    database read-models only. NOT per-tenant. NOT touched.         │
└──────────────────────────────────────────────────────────────────┘
```

## Model

### 1. Templates (catalog) — QS
QS defines a curated catalog of generic projection *templates* (e.g.
`event-count`, `entity-activity`). Read-only, platform-defined, served by QS:
`GET /api/v1/projections/templates`. These are **not** Core engine projections —
they are fold definitions QS applies to a tenant's stream. The Core engine's
internal projections (`entity_snapshots`, `event_counters`, Prime's 9, the
embedded demo set) are never templates.

### 2. Enablement (per-tenant, durable) — enabled set in Core metadata, logic in QS
A tenant's enabled set lives in **Core tenant metadata** as an opaque JSON blob,
`metadata.projections.enabled: ["event-count"]`. QS reads and read-modify-writes
this list through the existing tenant-metadata path (`TenantRepository.save` /
`update_quotas` → event-sourced `_system:tenant:updated`). Core stores it; Core
does **not** interpret it, fold over it, or serve per-tenant projection state.
All the enable/disable logic (validation against the catalog, quota check,
status) is QS's.

### 3. Per-tenant state (isolated) — QS ETS, keyed by tenant
QS folds the tenant's events into projection state in **`ProjectionSync`'s ETS
store, keyed by `(tenant_id, projection, entity)`** so one tenant's state never
mixes with another's. `ProjectionSync` already folds Core's event broadcasts into
ETS-cached state and serves it over `projection_channel.ex`; this extends that
mechanism with the tenant dimension and the enabled-set filter. State is a
**rebuildable read-model** — Core remains the durable log of record.

### 4. Backfill on enable (background) — QS
Enabling a projection returns immediately with status `building`. QS folds the
tenant's existing history asynchronously (a tenant-scoped query against Core,
paged/bounded), then flips the status to `ready`. Disable tombstones the tenant's
ETS state and removes the template from the enabled set. The dashboard shows
`building` while the backfill runs.

### 5. API (QS, all tenant-scoped, fail closed — no tenant → empty/deny)
Served by the Query Service, replacing the tenant-dropping `ProjectionController`:
- `GET    /api/v1/projections` — the tenant's enabled projections (+ status, last_processed_at).
- `GET    /api/v1/projections/templates` — the catalog.
- `POST   /api/v1/projections {template}` — enable (honors `tenant.can_create_projection()` quota).
- `DELETE /api/v1/projections/{name}` — disable.
- `GET    /api/v1/projections/{name}/state?entity_id=…` — tenant-scoped state from ETS.

QS resolves the authenticated tenant on every call. Today `ProjectionController.index`
+ `RustCoreClient.list_projections` pass no tenant and surface Core's global
registry — that is the leak this replaces.

## Dashboard

"Active Projections" counts the **tenant's enabled** projections (from QS's
enablement API), not Core's global registry. Closes t-7f9600. Add a projections
page to enable/disable from the catalog + view per-tenant state.

## Why not Core

The original design put per-tenant projection compute + state inside Core's Rust
projection engine — registering per-tenant projections, keying the engine's
`projection_state_cache` by `tenant_id`, folding `metadata.projections.enabled`
on the ingest path (`store.rs:531`, where every event is fed to every registered
projection). **This is the wrong layer.** Two reasons:

1. **It rewires Core's hottest path for a per-tenant, user-facing feature.**
   `process_event` runs on every ingest. Adding per-tenant state and an
   enabled-set lookup there is latency + memory risk on the path that every
   tenant's writes share, for a feature only the enabling tenant uses.

2. **It violates the Core/QS role split** (CLAUDE.md: "Core IS the database;
   Query Service is the API gateway + read-model layer"). A per-tenant,
   user-facing, enable/disable read-model that computes over *one tenant's*
   stream is a **gateway/read-model concern**, not a database-engine concern.
   Core's global engine projections (`entity_snapshots`, `event_counters`,
   Prime's 9, embedded demo set) are internal database read-models — global,
   not user-facing, not per-tenant. The per-tenant feature is a different animal
   and belongs in QS, which already folds Core's stream into ETS read-models and
   already terminates the tenant.

**The honest trade-off:** QS folds by reading the tenant's stream over the
network (WS subscription + a tenant-scoped backfill query) rather than computing
in-memory beside the data inside Core. That is a real cost — an extra network
hop and QS holding the folded state. It is acceptable because:
- `ProjectionSync` **already does exactly this** for the existing projection
  channel — the per-tenant feature reuses proven machinery, it does not invent a
  new pipeline.
- The state is a **rebuildable read-model**, not a source of truth. If QS loses
  it, it re-folds from Core's durable log. Putting it in Core would make it
  durable but at the cost of coupling a user feature to the engine.
- The tenant's event stream is **already tenant-scoped and fail-closed** in
  Core after the tenant-isolation work, so QS can read exactly the right slice.

**The one case where Core would be right** — and why this isn't it: if we needed
**durable materialized views** that must survive QS restarts without a re-fold,
or **ingest-latency** read-models shared by *all* tenants (the global engine
projections), Core is the correct home — those are database-internal concerns on
the write path. A per-tenant, user-toggled, rebuildable read-model is none of
those. So it stays in QS.

This boundary is enforced, not just documented: see
[Isolation & gating](#isolation--gating).

## Isolation & gating

- Every QS projections endpoint **fails closed** on a missing tenant (mirrors
  the streams/event-types + tenant-isolation work).
- The **`tenant-isolation-check`** gate (`tooling/tenant-isolation-check/`) now
  has a second responsibility: in addition to scanning the Query Service for
  non-tenant-scoped PubSub topics, it scans **`apps/core/src`** and FAILS if Core
  gains a per-tenant projection-compute concern (e.g. `list_projections_for_tenant`,
  a `tenant_id`-keyed projection state, or logic that *folds*
  `metadata.projections.enabled`). Core may **store** the enabled set as opaque
  tenant metadata; it may not **compute/serve** per-tenant projection state. A
  genuine exception must carry an inline `// CORE_PROJECTION_OK: <reason>`
  comment (mirroring `ISOLATION_OK`) — overridable, never silent.
- CLAUDE.md states the boundary rule ("per-tenant read-model compute lives in QS,
  not Core") and points to this section + the gate.

## Phasing (beads) — re-scoped 2026-06-26 to QS ownership

1. **t-bf412a** — Design: this doc (QS-centric). *(done; re-scoped)*
2. **t-fa3f57** — Core: persist `metadata.projections.enabled` (opaque set) +
   tenant-scoped metadata read. **No projection-engine change.**
   *(re-scoped from "tenant-scoped projection registry + list_projections_for_tenant")*
3. **t-2494c9** — QS: template catalog + tenant-scoped projections API
   (enable/disable/list/state), replacing the tenant-dropping
   `ProjectionController` / `RustCoreClient.list_projections`.
   *(re-scoped from "Core API … endpoints")*
4. **t-a7cc15** — QS: per-tenant `ProjectionSync` folding keyed by
   `(tenant_id, projection, entity)` + background backfill (`building`→`ready`).
   *(re-scoped from "QS: forward tenant to projections endpoints")*
5. **t-4ad39a** — Web: enable/disable UI + accurate dashboard count (closes t-7f9600).
6. **t-6f6875** — Gate: extend `tenant-isolation-check` to forbid per-tenant
   projection compute in Core + CLAUDE.md boundary rule. *(this prompt's gate work)*

## Resolved (2026-06-26) — decisions kept, relocated to QS

- **Catalog scope:** a **curated generic set** (e.g. `event-count`,
  `entity-activity`), defined in **QS**. Demo/domain projections
  (sagas/portfolios/trades) and the engine's internal projections stay
  Core-internal, never tenant templates.
- **Backfill:** **background**, in **QS**. Enabling returns immediately with
  status `building`; QS folds the tenant's history async (tenant-scoped Core
  query) and flips to `ready`. The dashboard shows `building`.
- **State store:** **QS ETS** (extend the existing `ProjectionSync` state),
  keyed by `(tenant_id, projection, entity)`. State is a rebuildable read-model;
  Core remains the durable log.
- **Enabled set (only Core-side bit):** stored in Core tenant metadata
  (`metadata.projections.enabled`) as an opaque blob, read/written by QS through
  the existing tenant-metadata path. No Core projection-engine change.
</content>
</invoke>
