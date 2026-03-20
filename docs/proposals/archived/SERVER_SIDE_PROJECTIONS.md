# Server-Side Projections & Fold-on-Read

> **Status**: Implemented (v0.10.5) — see ADR-006
> **Author**: Design session 2026-02-17
> **Scope**: Query Service — projection engine, fold-on-read, snapshot-aware queries
>
> **Note (2026-03-01):** Phase 1 (fold-on-read) and Phase 2 (continuous projections) were implemented in v0.10.5. This document is preserved as the original design proposal. See `docs/adr/006-server-side-projections-fold-on-read.md` for the decision record.

---

## 1. Problem Statement

Clients consuming AllSource events must implement their own fold logic to reconstruct entity state from raw events. Every entity type (indices, trades, sagas, portfolios, rebalancing) requires a custom fold function that:

1. Fetches all events for an entity type
2. Groups by entity ID
3. Reduces events into current state (applying creates, updates, deletes)
4. Filters projected fields (e.g., `is_deleted = false`)
5. Handles pagination client-side (can't paginate before folding)

This produces significant boilerplate across every client SDK and repository layer. The fold logic for `IndexState` alone is ~60 lines — and must be replicated for every entity type.

### What We Have Today

```
Client SDK (Rust/Python/TS)
    │
    ▼
Query Service ──HTTP──► Core
    │                    │
    │ Returns raw        │ Stores events
    │ events only        │ Has projection state cache (DashMap)
    ▼                    │ Has snapshot manager
                         │ Has WebSocket event stream
Client folds             │
events locally           ▼
    │              Projection domain model exists
    ▼              but ProjectionManager only has
IndexState[]       2 hardcoded projections
```

### Existing Building Blocks

| Component | Location | Status |
|-----------|----------|--------|
| `Projection.apply_events()` | QS `domain/entities/projection.ex` | Working fold via `Enum.reduce` |
| `ProjectionSync` GenServer | QS `application/services/projection_sync.ex` | L1 ETS + L2 Core DashMap sync |
| `ProjectionServer` GenServer | QS `application/use_cases/projection_server.ex` | Per-entity projection state |
| `CoreWebSocketClient` | QS `infrastructure/adapters/core_websocket_client.ex` | Subscribes to Core event stream |
| Projection state CRUD | Core `/api/v1/projections/{name}/{entity_id}/state` | Full API with bulk ops |
| Snapshot manager | Core `infrastructure/persistence/snapshot.rs` | Create, retrieve, merge, prune |
| Projection domain entity | Core `domain/entities/projection.rs` | Rich model — unused by manager |

**The plumbing exists. It just isn't wired together.**

---

## 2. Design Goals

1. **Eliminate client-side fold boilerplate** — clients query projected state, not raw events
2. **Leverage existing infrastructure** — no new databases, no new services
3. **Incremental delivery** — fold-on-read first (quick win), then continuous projections
4. **Snapshot-aware** — reduce fold cost from N events to delta-since-snapshot
5. **Core IS the database** — projection state lives in Core's DashMap, not PostgreSQL
6. **Type-safe projection definitions** — registered schemas, not arbitrary code execution

---

## 3. Architecture

### Target State

```
Client SDK
    │
    ▼
Query Service
    │
    ├─► GET /api/query/projected?projection=indices&user_id=...
    │       │
    │       ├─ Check Core for latest snapshot
    │       ├─ Fetch only events AFTER snapshot
    │       ├─ Apply registered fold function
    │       ├─ Filter on projected fields (is_deleted, user_id)
    │       ├─ Paginate on projected results
    │       └─ Return IndexState[] (not RawEvent[])
    │
    ├─► Continuous projection pipeline (Phase 2)
    │       │
    │       ├─ CoreWebSocketClient receives events
    │       ├─ Route to registered ProjectionServer per entity type
    │       ├─ Fold incrementally (1 event at a time)
    │       ├─ Sync state to Core DashMap via ProjectionSync
    │       └─ Periodic snapshot creation
    │
    └─► Core (unchanged)
            ├─ Event storage (WAL + Parquet + DashMap)
            ├─ Projection state cache (DashMap)
            └─ Snapshot storage (DashMap)
```

---

## 4. Phased Implementation

### Phase 1: Fold-on-Read Endpoint

**Goal**: New QS endpoint that folds events server-side and returns projected state.

#### 4.1.1 Projection Registry

A compile-time registry of named projection definitions. No dynamic code execution — each projection is an Elixir module implementing a behaviour.

```elixir
defmodule QueryServiceEx.Projections.Registry do
  @projections %{
    "indices" => QueryServiceEx.Projections.IndexState,
    "trades" => QueryServiceEx.Projections.TradeState,
    "portfolios" => QueryServiceEx.Projections.PortfolioState,
    "sagas" => QueryServiceEx.Projections.SagaState
  }

  def get(name), do: Map.get(@projections, name)
  def list, do: Map.keys(@projections)
end
```

Each projection module implements:

```elixir
defmodule QueryServiceEx.Projections.IndexState do
  @behaviour QueryServiceEx.Projections.Behaviour

  @impl true
  def entity_type, do: "index"

  @impl true
  def initial_state, do: %{}

  @impl true
  def apply_event(state, %{"event_type" => "index.created"} = event) do
    Map.merge(state, %{
      "id" => event["entity_id"],
      "name" => get_in(event, ["data", "name"]),
      "is_deleted" => false,
      "created_at" => event["timestamp"],
      "updated_at" => event["timestamp"]
    })
  end

  def apply_event(state, %{"event_type" => "index.updated"} = event) do
    state
    |> Map.merge(event["data"])
    |> Map.put("updated_at", event["timestamp"])
  end

  def apply_event(state, %{"event_type" => "index.deleted"} = event) do
    Map.merge(state, %{"is_deleted" => true, "updated_at" => event["timestamp"]})
  end

  def apply_event(state, _event), do: state

  @impl true
  def filterable_fields, do: ["is_deleted", "user_id", "name"]
end
```

#### 4.1.2 Fold-on-Read Controller

```
POST /api/query/projected
{
  "projection": "indices",
  "filters": {"is_deleted": false, "user_id": "..."},
  "page": 1,
  "page_size": 50,
  "sort_by": "updated_at",
  "sort_order": "desc"
}

Response:
{
  "data": [IndexState, ...],
  "count": 50,
  "total": 234,
  "projection": "indices",
  "folded_from": {
    "event_count": 1847,
    "snapshot_used": true,
    "snapshot_age_seconds": 3600,
    "events_after_snapshot": 23,
    "fold_duration_ms": 12
  }
}
```

#### 4.1.3 Snapshot-Aware Folding

The fold pipeline checks for existing snapshots before fetching events:

```
1. Lookup latest snapshot for (projection, entity_id) from Core
2. If snapshot exists:
   a. Use snapshot state as initial accumulator
   b. Fetch only events with timestamp > snapshot.as_of
3. If no snapshot:
   a. Use projection.initial_state as accumulator
   b. Fetch all events for entity type
4. Fold events using projection.apply_event/2
5. Filter on projected fields
6. Paginate and return
```

#### 4.1.4 Automatic Snapshot Creation

After fold-on-read, if the event count since last snapshot exceeds a threshold (default: 100), create a new snapshot in Core:

```elixir
if folded_from.events_after_snapshot > @snapshot_threshold do
  Task.async(fn ->
    CoreClient.create_snapshot(entity_id, "automatic")
    CoreClient.save_projection_state(projection, entity_id, state)
  end)
end
```

This is lazy — snapshots are created as a side effect of reads, reducing fold cost over time.

---

### Phase 2: Continuous Projections

**Goal**: QS maintains materialized read models by subscribing to Core's event stream.

#### 4.2.1 Projection Supervisor

A `DynamicSupervisor` that starts a `ProjectionServer` for each registered projection:

```
QueryServiceEx.Projections.Supervisor
    ├─ ProjectionServer (indices)
    ├─ ProjectionServer (trades)
    ├─ ProjectionServer (portfolios)
    └─ ProjectionServer (sagas)
```

Each `ProjectionServer`:
1. Subscribes to `events:type:{entity_type}` via PubSub (already broadcast by `CoreWebSocketClient`)
2. Maintains current projected state per entity in ETS
3. Applies events incrementally (1 at a time, no refetch)
4. Syncs dirty state to Core DashMap via `ProjectionSync` (periodic batch)
5. Creates snapshots when event threshold is reached

#### 4.2.2 Query Endpoint for Materialized State

```
GET /api/query/projected?projection=indices&is_deleted=false&user_id=...&page=1&page_size=50
```

Reads directly from ETS (sub-millisecond). Falls back to fold-on-read if ETS is cold (after QS restart).

#### 4.2.3 Consistency Guarantees

- **Eventual consistency** by default: reads from local ETS, may lag behind Core by up to the sync interval
- **Strong consistency** opt-in: `?consistency=strong` forces a fold-on-read from Core's latest events
- Consistency mode maps to existing `client_for_consistency/1` routing

---

### Phase 3: Wire Format Cleanup

**Goal**: Standardize response shapes across all QS endpoints.

#### Changes

| Current | Standardized |
|---------|-------------|
| `{data, count}` (some endpoints) | `{data, count, total}` everywhere |
| `{webhooks, total}` | `{data, count, total}` |
| `{replays, total}` | `{data, count, total}` |
| Snapshot controller maps `total` → `count` | Fix: `count` = returned items, `total` = full cardinality |
| Core uses `total`, QS uses `count` | Standardize: `count` in paginated responses = items returned |

#### Versioning

Add `Accept: application/vnd.allsource.v2+json` header support. v1 responses unchanged, v2 uses standardized format. Default to v2 for new endpoints.

---

## 5. What Does NOT Change

- **Core** — no changes needed. All existing APIs are sufficient.
- **Event storage** — events remain the source of truth, projections are derived.
- **PostgreSQL** — still only for operational metadata. Projections live in Core's DashMap.
- **Client SDKs** — existing raw event queries continue to work. Projected queries are additive.

---

## 6. Performance Characteristics

| Operation | Phase 1 (Fold-on-Read) | Phase 2 (Continuous) |
|-----------|----------------------|---------------------|
| First query (cold) | Fetch all events + fold | Same (ETS cold after restart) |
| Subsequent queries | Snapshot + delta fold | ETS read (~0.1ms) |
| With 1000 events, no snapshot | ~50ms fold | ~0.1ms read |
| With 1000 events + snapshot at 900 | ~5ms fold (100 event delta) | ~0.1ms read |
| Write overhead | None | PubSub + fold per event (~1ms) |

---

## 7. Migration Path

1. Ship Phase 1 (fold-on-read) — clients can adopt immediately
2. Existing raw event endpoints remain unchanged — no breaking changes
3. Ship Phase 2 (continuous projections) — transparent performance upgrade
4. Clients don't change code between Phase 1 and Phase 2 — same query endpoint
5. Ship Phase 3 (wire format) — versioned, non-breaking

---

## 8. Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Fold-on-read latency for large event streams | Snapshot-aware folding reduces to delta. Phase 2 eliminates entirely. |
| Projection state diverges from events | Projection reset endpoint exists. Rebuild from events at any time. |
| QS restart loses ETS state | Fall back to fold-on-read. ProjectionSync rehydrates from Core DashMap. |
| New event types not handled by projection | `apply_event/2` default clause returns state unchanged. No crash. |
| Memory pressure from large projections | Core DashMap is the source of truth. QS ETS is a cache with configurable eviction. |
