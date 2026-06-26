# Per-Tenant Projections (design)

**Epic:** t-822210 · **Symptom that triggered it:** t-7f9600 (dashboard "Active
Projections" shows the global engine registry as the tenant's).

## Problem

Projections in Core are a **global registry** — `EventStore.register_projection`
adds an `Arc<dyn Projection>` processed against *every* tenant's events;
`list_projections()` returns `Vec<(String, Arc<dyn Projection>)>` with no tenant
dimension. The dashboard therefore shows platform engine views
(indices/portfolios/sagas/trades) as "your projections" — wrong, and a
cross-tenant leak in the same class as the streams/event-types scoping already
fixed.

A normal tenant's honest projection count today is **0** (they own none of the
global ones). We want projections to be a real per-tenant concept.

## Non-goal (v1)

**Users do not author projection code.** Letting tenants ship reducer logic
(Rust/Wasm/DSL) is a much larger, security-sensitive surface. v1 is
**enablement-based**: the engine ships a catalog of projection *templates*; a
tenant enables the ones it wants; the engine maintains that tenant's own state.

## Model

### 1. Templates (catalog)
The existing projection implementations become a named **template catalog**
(e.g. `event-count`, `entity-activity`, …). Read-only, platform-defined.
`GET /api/v1/projection-templates`.

### 2. Enablement (per-tenant, durable)
A tenant's enabled set lives in **Core tenant metadata** (JSON blob, no Postgres),
e.g. `metadata.projections.enabled: ["event-count"]`. Enable/disable mutate this
list (read-modify-write under the per-tenant lock, like subscriptions).

### 3. Per-tenant state (isolated)
Projection state is keyed by **(tenant_id, projection, entity)** so one tenant's
state never mixes with another's. The engine, when processing an event, updates
state only for projections the event's tenant has enabled.

### 4. Backfill on enable
Enabling a projection replays the tenant's existing events (tenant-scoped query)
into its new state — bounded/paged like `BackfillEventsUsedUseCase`, honest
"capped" result for very large tenants. Disable tombstones the tenant's state.

### 5. API (all tenant-scoped, fail closed — no tenant → empty/deny)
- `GET  /api/v1/projections` — the tenant's enabled projections (+ status, last_processed_at).
- `GET  /api/v1/projection-templates` — the catalog.
- `POST /api/v1/projections {template}` — enable (honors `tenant.can_create_projection()` quota).
- `DELETE /api/v1/projections/{name}` — disable.
- `GET  /api/v1/projections/{name}/state?entity_id=…` — tenant-scoped state.

QS forwards the authenticated tenant to all of these (today it forwards none —
`ProjectionController.index` + `RustCoreClient.list_projections` drop the tenant).

## Dashboard
"Active Projections" counts the **tenant's enabled** projections, not the global
registry. Closes t-7f9600. Add a projections page to enable/disable + view state.

## Isolation & gating
- Every projections endpoint fails closed on a missing tenant (mirror
  streams/event-types + the tenant-isolation work).
- The `tenant-isolation-check` gate should grow to cover projection topics/queries.

## Phasing (beads)
1. **t-bf412a** — this design.
2. **t-fa3f57** — Core: per-tenant enablement + state + `list_projections_for_tenant`.
3. **t-2494c9** — Core API: tenant-scoped list/enable/disable/state + template catalog.
4. **t-a7cc15** — QS: forward tenant to all projection endpoints.
5. **t-4ad39a** — Web: enable/disable UI + accurate dashboard count.

## Open questions for review
- **Catalog scope:** expose all existing engine projections as templates, or a
  curated subset? (Some global ones — sagas/portfolios — may be demo-only.)
- **State store:** reuse the existing projection state store keyed by tenant, or
  a dedicated per-tenant store? Affects memory + the lazy-load/eviction model.
- **Backfill cap + async:** enable returns immediately with a "building" status
  and backfills in the background, or blocks until built?
