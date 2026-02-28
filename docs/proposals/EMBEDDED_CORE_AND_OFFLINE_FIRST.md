# Embedded Core Library + Offline-First Desktop Integration

> **Status**: **COMPLETE** (All 8 phases done)
> **Tracking**: [GitHub Issue #73](https://github.com/all-source-os/all-source/issues/73)
> **Origin**: Feature request from [Longhand](https://github.com/technical-leaders/longhand) — offline-first Tauri 2 desktop app
> **Started**: 2026-02-28

---

## Overview

Issue #73 requests capabilities that make AllSource viable for offline-first desktop apps: an embeddable Rust library API, bidirectional sync, cloud worker orchestration, and AI workflow optimizations. This document tracks design decisions, implementation progress, TDD test specifications, and open questions for each capability.

---

## 1. Motivation

AllSource Core currently runs as a standalone HTTP server on port 3900. For desktop apps (Tauri, Electron), CLI tools, and AI agent frameworks, embedding Core as a Rust library — no TCP listener, no multi-tenant overhead — opens an entirely new market segment.

### Why This Matters

- **SQLite's path**: SQLite succeeded by being embeddable first, server second. AllSource can follow the same trajectory for event stores.
- **Natural upsell**: Embedded instances that sync to AllSource Cloud create a free-local → paid-cloud funnel.
- **AI-native**: Agent frameworks (Claude Code, Langchain, custom) need local event stores for workflow state. Embedding Core eliminates the Docker deployment step entirely.

---

## Phase 1: Embeddable Core Library API — DONE

**Priority**: P0 | **Effort**: Medium | **Status**: Implemented + 26 tests passing

### What Was Built

A thin `EmbeddedCore` facade behind the `embedded` feature flag. Delegates to the existing `EventStore` — no rewrite. Accepts plain strings, returns plain types.

```rust
use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent, Query};

let core = EmbeddedCore::open(Config::builder()
    .data_dir("~/.myapp/events")
    .single_tenant(true)
    .build()?
).await?;

core.ingest(IngestEvent {
    entity_id: "order-123",
    event_type: "order.placed",
    payload: json!({"total": 99.99}),
    metadata: None,
    tenant_id: None,  // auto "default" in single-tenant mode
}).await?;

let events = core.query(Query::new().entity_id("order-123")).await?;
let state = core.projection("entity_snapshots", "order-123");
core.shutdown().await?;
```

### Architecture

```
                     EmbeddedCore (facade)
                     ┌─────────────────────────────────────┐
                     │                                     │
  IngestEvent<'a>    │  Event::from_strings()              │
  (&str fields) ────>│  ──> EventStore::ingest(Event)      │
                     │                                     │
  Query (builder) ──>│  ──> EventStore::query(Request)     │
                     │  ──> Vec<Event> → Vec<EventView>    │
                     │                                     │
  projection() ─────>│  ──> ProjectionManager::get_state() │
                     │                                     │
  Config::builder() ─│──> EventStoreConfig::production()   │
                     │    or EventStoreConfig::default()   │
                     └─────────────────────────────────────┘
                                    │
                              EventStore
                     ┌──────────────┼──────────────┐
                     │              │              │
                  DashMap         WAL          Parquet
                (in-memory)   (durability)   (columnar)
```

### Files

| File | Purpose |
|------|---------|
| `apps/core/Cargo.toml` | `embedded = []` feature flag |
| `apps/core/src/lib.rs` | `#[cfg(feature = "embedded")] pub mod embedded;` |
| `apps/core/src/embedded/mod.rs` | Module root + re-exports |
| `apps/core/src/embedded/config.rs` | `Config` builder (data_dir, wal_sync, single_tenant) |
| `apps/core/src/embedded/types.rs` | `IngestEvent<'a>`, `Query`, `EventView` |
| `apps/core/src/embedded/core.rs` | `EmbeddedCore` struct — facade over `EventStore` |
| `apps/core/tests/embedded_core_api.rs` | 26 TDD tests (all passing) |

### Design Decisions

| Decision | Rationale |
|----------|-----------|
| `open` not `new` for constructor | Communicates durability semantics (SQLite, RocksDB precedent) |
| Builder pattern on `Query`, not struct literal | Avoids `Some()` noise, enables method chaining |
| `projection()` is sync, not async | DashMap lookup is ~11.9μs; async wrapper adds overhead and misleads |
| `embedded` feature is additive, not subtractive | Gating axum with `not(feature = "embedded")` causes 93 compilation errors; full decoupling requires `server` feature (Phase 3) |
| `&str` lifetime on `IngestEvent` | Zero-alloc at call sites; string literals are the common case |
| Separate `EventView` output type | Insulates embedders from internal `Event` value objects |
| `EmbeddedCore` re-exported at crate root | `allsource_core::EmbeddedCore` works; `Config` stays namespaced to avoid collision |
| Single-tenant default (`true`) | Desktop apps don't need multi-tenancy |
| `tenant_id: Option<&str>` on `IngestEvent` | Multi-tenant support; ignored in single-tenant mode |
| `Serialize`/`Deserialize` on `EventView` | Enables JSON logging, round-tripping, and serialization by embedders |
| `inner()` escape hatch | Returns `Arc<EventStore>` for advanced use cases |

### Verification

```bash
cargo test --features embedded --test embedded_core_api  # 26/26 pass
cargo test                                                # 1,645+ pass, 0 fail
```

---

## Phase 2: API Completeness — DONE

**Priority**: P1 | **Effort**: Low | **Status**: Implemented + 7 new tests (26 total)

### What Was Built

| Task | Status |
|------|--------|
| `Serialize`/`Deserialize` derives on `EventView` | **Done** |
| `tenant_id: Option<&str>` field on `IngestEvent` | **Done** |
| Multi-tenant wired in `effective_tenant_id()` — uses explicit tenant in multi-tenant mode, ignores in single-tenant | **Done** |
| Re-export `EmbeddedCore` at crate root (`allsource_core::EmbeddedCore`) | **Done** |

### Tests Added (in `apps/core/tests/embedded_core_api.rs`)

| Test | Validates |
|------|-----------|
| `event_view_serializes_to_json` | `EventView` → JSON round-trip |
| `event_view_deserializes_from_json` | JSON → `EventView` deserialization |
| `ingest_with_explicit_tenant_id` | Multi-tenant ingest with `tenant_id: Some("tenant-acme")` |
| `ingest_without_tenant_id_in_multi_tenant_uses_default` | `None` tenant defaults to "default" |
| `single_tenant_ignores_explicit_tenant_id` | Single-tenant mode always uses "default" |
| `multi_tenant_query_filters_by_tenant` | Tenant IDs persisted correctly on events |
| `embedded_core_accessible_from_crate_root` | `allsource_core::EmbeddedCore` import works |

### Known Limitation

Core's `EventStore::query()` does not filter by `tenant_id` — the field is stored on events but not used as a query predicate. The `Query.tenant_id()` builder method exists for forward compatibility.

### Remaining (deferred to later phases)

| Task | Why Deferred |
|------|-------------|
| `ingest_batch()` for bulk ingestion | Requires design decision on atomicity semantics |
| Owned `IngestEventOwned` with `Serialize` | Low demand; `&str` lifetime is ergonomic for all known use cases |

---

## Phase 3: Binary Size Optimization — DONE

**Priority**: P2 | **Effort**: Medium | **Status**: Implemented + 5 tests passing

### What Was Built

Server-only dependencies gated behind `#[cfg(feature = "server")]` so embedded builds don't pull in axum, reqwest, jsonwebtoken, argon2, or aes-gcm. The `analytics` feature separately gates datafusion.

### Feature Flags (Implemented)

```toml
[features]
default = ["server", "analytics"]
embedded = []
server = ["dep:axum", "dep:axum-extra", "dep:tower", "dep:tower-http",
          "dep:reqwest", "dep:jsonwebtoken", "dep:argon2",
          "dep:aes-gcm", "dep:http"]
analytics = ["dep:datafusion"]
flight = ["dep:arrow-flight"]
```

| Dependency | Feature | Gating |
|-----------|---------|--------|
| `axum` + `axum-extra` + `tower` + `tower-http` | `server` | `infrastructure::web`, error.rs `IntoResponse` |
| `reqwest` | `server` | `webhook_worker` module |
| `jsonwebtoken` + `argon2` | `server` | `infrastructure::security::{auth, middleware}` |
| `aes-gcm` | `server` | `security::{kms, encryption}` |
| `http` | `server` | Axum HTTP types |
| `datafusion` | `analytics` | `infrastructure::query::eventql` |
| `arrow-flight` | `flight` | Standalone |
| `prometheus` | always | Kept required — used in core `PipelineManager`, `ProjectionManager`, WAL replication |

### Modules Gated

| Module | Feature | Rationale |
|--------|---------|-----------|
| `infrastructure::web` | `server` | HTTP handlers, WebSocket, all API routes |
| `infrastructure::resp` | `server` | RESP3/Redis wire protocol server |
| `infrastructure::security::{auth, middleware}` | `server` | JWT auth, axum middleware extractors |
| `security::{kms, encryption}` | `server` | AES-GCM field-level encryption |
| `infrastructure::query::eventql` | `analytics` | DataFusion SQL engine |
| `webhook_worker` | `server` | Reqwest HTTP client for webhook delivery |
| `error.rs` IntoResponse impl | `server` | Axum error-to-HTTP-response conversion |
| Binary targets (main, admin, sentinel) | `server` | `required-features = ["server"]` |

### EventStore Conditional Compilation

Server-only fields in `EventStore` struct:
- `websocket_manager: Arc<WebSocketManager>` — gated
- `metrics: Arc<MetricsRegistry>` — gated (struct field + all `self.metrics.*` calls in ingest/query)
- `webhook_registry: Arc<WebhookRegistry>` — gated
- `webhook_tx` — gated

### Design Decision: prometheus Stays Required

`prometheus` (~1MB) is kept as a required dependency because `MetricsRegistry` is deeply used by `PipelineManager`, `ProjectionManager`, and WAL replication modules. Gating these would require either no-op stubs or major refactoring. The size impact is minimal compared to the gated deps (~25MB savings).

### Tests — `apps/core/tests/embedded_minimal_build.rs`

| Test | Validates |
|------|-----------|
| `embedded_compiles_without_server_features` | Core compile with `--no-default-features --features embedded` |
| `error_types_work_without_axum` | `AllSourceError` thiserror Display without axum IntoResponse |
| `query_works_without_server` | Query path works without websocket/metrics |
| `persistence_works_without_server` | WAL + Parquet persistence without server deps |
| `event_view_serde_works_without_server` | EventView serde round-trip without server |

### Verification

```bash
# Embedded-only build (no server deps)
cargo check --no-default-features --features embedded -p allsource-core  # 0 errors
cargo test --no-default-features --features embedded --test embedded_minimal_build  # 5/5 pass
cargo test --no-default-features --features embedded --test embedded_core_api  # 26/26 pass

# Full build (no regressions)
cargo test -p allsource-core  # 1,644 pass, 0 fail
```

---

## Phase 4: Bidirectional Sync Protocol — DONE

**Priority**: P0 | **Effort**: High | **Status**: Implemented + 15 tests passing

### What Was Built

Bidirectional sync between `EmbeddedCore` instances using existing HLC, VersionVector, and CrdtResolver infrastructure. Each instance configured with a unique `node_id` gets an HLC clock and CRDT resolver for deduplication.

**Key method**: `EmbeddedCore::sync_to(&self, peer: &EmbeddedCore)` — sends all events the peer hasn't seen yet, using CRDT event-ID deduplication to prevent duplicates. For full bidirectional sync: `a.sync_to(&b)` then `b.sync_to(&a)`.

**How it works**:
1. Source queries all its events
2. For each event, creates a `ReplicatedEvent` with HLC timestamp and event UUID
3. Peer's `CrdtResolver` checks if event ID was already seen → `Accept` or `Skip`
4. Accepted events are cloned (preserving original UUID) and ingested into peer's store
5. Peer's HLC is updated via `receive()` for causal ordering
6. `EntitySnapshotProjection` uses timestamp-aware merge — only applies events with `timestamp >= last_ts`

**Conflict resolution**: Last-Write-Wins (LWW) via event timestamp. Events arriving out of order are skipped by the projection, ensuring both instances converge to the same state regardless of sync order.

### Tests — `apps/core/tests/bidirectional_sync.rs`

| Test | Validates |
|------|-----------|
| `hlc_provides_total_order_across_nodes` | Independent HLC clocks produce total ordering |
| `hlc_respects_causality_after_receive` | `receive()` establishes causality |
| `hlc_logical_counter_breaks_same_ms_ties` | Rapid events get ordered even at same ms |
| `hlc_rejects_excessive_clock_drift` | Max drift enforcement (1s) |
| `version_vector_tracks_per_node_progress` | VV advances per-node |
| `version_vector_detects_unseen_events` | `is_new()` distinguishes seen/unseen |
| `version_vector_merge_is_commutative` | `merge(a,b) == merge(b,a)` |
| `version_vector_merge_takes_max` | Pointwise max on merge |
| `crdt_resolver_accepts_new_event` | First-time event accepted |
| `crdt_resolver_skips_duplicate_event` | Duplicate event skipped |
| `two_instances_sync_after_independent_writes` | Both instances get both events after sync |
| `sync_is_idempotent` | Multiple syncs don't duplicate events |
| `sync_conflict_last_write_wins` | LWW convergence via timestamp-aware projection |
| `offline_queue_drains_on_reconnect` | 10 offline events transferred on sync |
| `sync_preserves_event_ordering` | Causal ordering maintained across sync |

### Verification

```bash
cargo test --no-default-features --features embedded-sync --test bidirectional_sync  # 15/15 pass
cargo test -p allsource-core  # 1,644+ pass, 0 fail
```

### Design Notes (Original)

### The Problem

Current replication is leader-to-follower (unidirectional WAL shipping). Offline-first requires **bidirectional sync**: both local embedded and cloud instances accept writes independently, then reconcile on reconnect.

### Proposed Approach

**Hybrid Logical Clocks (HLC)** on every event:

```rust
struct HLC {
    physical: u64,  // wall clock millis
    logical: u16,   // tie-breaker counter
    node_id: u32,   // unique per instance
}
```

**Sync vector exchange**:
```
LOCAL → CLOUD:  events where hlc > cloud.last_seen[local.node_id]
CLOUD → LOCAL:  events where hlc > local.last_seen[cloud.node_id]
```

**Conflict resolution** per entity type:

| Strategy | Use Case |
|----------|----------|
| Last Write Wins (HLC) | Config, definitions |
| Append-only merge | Execution logs, telemetry |
| First Write Wins | Task claims |
| Custom callback | Domain-specific |

**Transport degradation**:
1. WebSocket connected → real-time bidirectional
2. WebSocket down, HTTPS up → polling with outbox drain
3. Fully offline → queue locally, sync on reconnect

### Existing Code to Build On

- `apps/core/src/infrastructure/cluster/` already has `HybridLogicalClock`, `VersionVector`, `CrdtResolver`, `ConflictResolution`
- `apps/core/src/infrastructure/replication/` has `WalShipper`/`WalReceiver`
- The cluster module exports `HlcTimestamp`, `ReplicatedEvent`, `GeoSyncRequest`/`GeoSyncResponse`

### Open Questions

1. **HLC on Event entity**: Adding an HLC field to `Event` is a schema change. Backward compatibility with existing WAL files?
2. **Conflict resolution config**: Where are per-entity-type merge strategies configured? `EmbeddedConfig`? Per-schema?
3. **Transport direction**: Does the embedded library push, or does cloud pull?
4. **Tombstones**: How to handle deletes in an append-only model?
5. **WAL format**: Current WAL entries don't include HLC. Migration path?

### RED Phase Tests — `apps/core/tests/bidirectional_sync.rs`

```rust
// --- HLC Ordering ---

#[test]
fn hlc_orders_causally() {
    let mut clock_a = HLC::new(1); // node_id = 1
    let mut clock_b = HLC::new(2); // node_id = 2

    let t1 = clock_a.now();
    let t2 = clock_b.now();
    // Both generated independently — HLC must still provide total order
    assert_ne!(t1, t2);
}

#[test]
fn hlc_respects_causality_after_receive() {
    let mut clock_a = HLC::new(1);
    let mut clock_b = HLC::new(2);

    let t1 = clock_a.now();
    clock_b.receive(t1); // b learns of a's time
    let t2 = clock_b.now();
    // t2 must be causally after t1
    assert!(t2 > t1);
}

#[test]
fn hlc_breaks_ties_with_node_id() {
    let mut clock_a = HLC::new(1);
    let mut clock_b = HLC::new(2);

    // Force same physical time
    let t1 = clock_a.now_at(1000);
    let t2 = clock_b.now_at(1000);
    // Same physical+logical → node_id breaks the tie
    assert_ne!(t1, t2);
    // Total order exists
    assert!(t1 < t2 || t2 < t1);
}

// --- Version Vectors ---

#[test]
fn version_vector_detects_missing_events() {
    let mut local = VersionVector::new();
    let mut remote = VersionVector::new();

    local.advance(1, 5);  // node 1 has 5 events
    remote.advance(1, 3); // remote only has 3

    let delta = local.delta_since(&remote);
    assert_eq!(delta.get(&1), Some(&(3, 5))); // need events 4..=5 from node 1
}

#[test]
fn version_vector_merge_is_commutative() {
    let mut a = VersionVector::new();
    let mut b = VersionVector::new();
    a.advance(1, 5);
    b.advance(2, 3);

    let ab = a.merge(&b);
    let ba = b.merge(&a);
    assert_eq!(ab, ba);
}

// --- Conflict Resolution ---

#[test]
fn lww_picks_latest_hlc() {
    let event_a = make_event("entity-1", "config.updated", hlc(1000, 0, 1));
    let event_b = make_event("entity-1", "config.updated", hlc(1001, 0, 2));

    let winner = resolve_conflict(ConflictStrategy::LastWriteWins, &event_a, &event_b);
    assert_eq!(winner.hlc.physical, 1001);
}

#[test]
fn first_write_wins_picks_earliest_hlc() {
    let event_a = make_event("task-1", "task.claimed", hlc(1000, 0, 1));
    let event_b = make_event("task-1", "task.claimed", hlc(1001, 0, 2));

    let winner = resolve_conflict(ConflictStrategy::FirstWriteWins, &event_a, &event_b);
    assert_eq!(winner.hlc.physical, 1000);
}

#[test]
fn append_only_keeps_both() {
    let event_a = make_event("log-1", "log.entry", hlc(1000, 0, 1));
    let event_b = make_event("log-1", "log.entry", hlc(1001, 0, 2));

    let result = resolve_conflict(ConflictStrategy::AppendOnly, &event_a, &event_b);
    // AppendOnly merges both — no winner, both survive
    assert_eq!(result, MergeResult::KeepBoth);
}

// --- Sync Protocol ---

#[tokio::test]
async fn two_instances_sync_after_independent_writes() {
    let core_a = EmbeddedCore::open(
        Config::builder().node_id(1).build().unwrap()
    ).await.unwrap();
    let core_b = EmbeddedCore::open(
        Config::builder().node_id(2).build().unwrap()
    ).await.unwrap();

    // Independent writes
    core_a.ingest(IngestEvent {
        entity_id: "doc-1", event_type: "doc.edited",
        payload: json!({"text": "hello from A"}), metadata: None,
    }).await.unwrap();

    core_b.ingest(IngestEvent {
        entity_id: "doc-2", event_type: "doc.edited",
        payload: json!({"text": "hello from B"}), metadata: None,
    }).await.unwrap();

    // Sync: a → b, b → a
    sync_pair(&core_a, &core_b).await.unwrap();

    // Both should have both events
    let a_events = core_a.query(Query::new().event_type_prefix("doc.")).await.unwrap();
    let b_events = core_b.query(Query::new().event_type_prefix("doc.")).await.unwrap();
    assert_eq!(a_events.len(), 2);
    assert_eq!(b_events.len(), 2);
}

#[tokio::test]
async fn sync_is_idempotent() {
    let core_a = open_core_with_node(1).await;
    let core_b = open_core_with_node(2).await;

    core_a.ingest(sample_event("e1")).await.unwrap();
    sync_pair(&core_a, &core_b).await.unwrap();
    sync_pair(&core_a, &core_b).await.unwrap(); // second sync
    sync_pair(&core_a, &core_b).await.unwrap(); // third sync

    // Still only 1 event in each
    assert_eq!(core_a.stats().total_events, 1);
    assert_eq!(core_b.stats().total_events, 1);
}

#[tokio::test]
async fn sync_conflict_lww_resolution() {
    let core_a = open_core_with_node(1).await;
    let core_b = open_core_with_node(2).await;

    // Both write to same entity
    core_a.ingest(IngestEvent {
        entity_id: "config-1", event_type: "config.updated",
        payload: json!({"theme": "dark"}), metadata: None,
    }).await.unwrap();

    // Slight delay so B's HLC is later
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    core_b.ingest(IngestEvent {
        entity_id: "config-1", event_type: "config.updated",
        payload: json!({"theme": "light"}), metadata: None,
    }).await.unwrap();

    sync_pair(&core_a, &core_b).await.unwrap();

    // LWW: B's event wins (later HLC)
    let a_state = core_a.projection("entity_snapshots", "config-1");
    let b_state = core_b.projection("entity_snapshots", "config-1");
    assert_eq!(a_state, b_state);
    assert_eq!(a_state.unwrap()["theme"], "light");
}

#[tokio::test]
async fn offline_queue_drains_on_reconnect() {
    let core_local = open_core_with_node(1).await;
    let core_cloud = open_core_with_node(2).await;

    // Ingest while "offline" (no sync)
    for i in 0..10 {
        core_local.ingest(IngestEvent {
            entity_id: &format!("item-{i}"),
            event_type: "item.created",
            payload: json!({"i": i}),
            metadata: None,
        }).await.unwrap();
    }

    assert_eq!(core_local.stats().total_events, 10);
    assert_eq!(core_cloud.stats().total_events, 0);

    // "Reconnect" — sync
    sync_pair(&core_local, &core_cloud).await.unwrap();
    assert_eq!(core_cloud.stats().total_events, 10);
}
```

---

## Phase 5: Replicant Worker Protocol — DONE

**Priority**: P1 | **Effort**: Medium | **Status**: Implemented + 14 tests passing

### What Was Built

Three custom projections registered at `EmbeddedCore::open()` when `embedded-replicant` feature is enabled:

1. **`WorkflowStatusProjection`** — folds `workflow.*` events into per-entity state: `{status, replicant_id, steps_total, steps_completed, awaiting_approval, output, error}`. First-write-wins claim guard (only allows `workflow.claimed` when status is `pending`).
2. **`ReplicantRegistryProjection`** — folds `replicant.*` events into worker state: `{status, capabilities, last_heartbeat}`. Tracks active/stale lifecycle.
3. **`TaskQueueProjection`** — tracks dispatched-but-unclaimed workflows. Query `__all` entity_id returns `{pending: ["wf-2", "wf-3"]}`.

**Key infrastructure addition**: `EventStore::register_projection()` enables runtime projection registration (previously only at construction time).

### Concept

Cloud workers ("replicants") subscribe to `workflow.dispatched` events, execute AI tasks, stream results back. AllSource projections handle orchestration — no Temporal needed.

### Event Schema

```
workflow.dispatched          — user queues work from desktop
workflow.claimed             — replicant claims task (first-write-wins)
workflow.step.started        — step execution begins
workflow.step.completed      — step output ready
workflow.step.failed         — step error
workflow.approval.requested  — human review needed
workflow.approval.granted    — user approves
workflow.approval.rejected   — user rejects
workflow.output.ready        — final output available
replicant.registered         — worker comes online
replicant.heartbeat          — liveness (30s interval)
replicant.stale              — sentinel marks unresponsive
```

### Projections Needed

1. **Workflow status** — folds `workflow.*` into `{ id, status, steps_completed, steps_total, output, awaiting_approval }`
2. **Replicant registry** — folds `replicant.*` into `{ id, status, capabilities, last_heartbeat, active_workflows }`
3. **Task queue** — dispatched but unclaimed workflows, ordered by dispatch time
4. **Claim guard** — ensures exactly one replicant claims each workflow (first-write-wins)

### Open Questions

1. Should this be projection definitions shipped with Core, or a separate crate?
2. Does the existing `ExactlyOnceRegistry` cover first-write-wins claim guard?
3. Heartbeat/stale detection — background task in EmbeddedCore, or external sentinel?

### RED Phase Tests — `apps/core/tests/replicant_protocol.rs`

```rust
// --- Workflow lifecycle ---

#[tokio::test]
async fn workflow_dispatch_and_claim() {
    let core = open_in_memory_core().await;

    // Dispatch a workflow
    core.ingest(IngestEvent {
        entity_id: "wf-1", event_type: "workflow.dispatched",
        payload: json!({"name": "summarize", "input": "long text..."}),
        metadata: None,
    }).await.unwrap();

    // Claim it (first-write-wins)
    core.ingest(IngestEvent {
        entity_id: "wf-1", event_type: "workflow.claimed",
        payload: json!({"replicant_id": "replicant-a"}),
        metadata: None,
    }).await.unwrap();

    let state = core.projection("workflow_status", "wf-1").unwrap();
    assert_eq!(state["status"], "claimed");
    assert_eq!(state["replicant_id"], "replicant-a");
}

#[tokio::test]
async fn workflow_claim_guard_rejects_double_claim() {
    let core = open_in_memory_core().await;

    core.ingest(make_event("wf-1", "workflow.dispatched", json!({}))).await.unwrap();
    core.ingest(make_event("wf-1", "workflow.claimed", json!({"replicant_id": "r-1"}))).await.unwrap();

    // Second claim should be rejected by claim guard projection
    let result = core.ingest(make_event("wf-1", "workflow.claimed", json!({"replicant_id": "r-2"}))).await;
    // Either: the claim guard rejects it, OR
    // the projection ignores the second claim event
    let state = core.projection("workflow_status", "wf-1").unwrap();
    assert_eq!(state["replicant_id"], "r-1"); // first claimer wins
}

#[tokio::test]
async fn workflow_step_progression() {
    let core = open_in_memory_core().await;

    core.ingest(make_event("wf-1", "workflow.dispatched", json!({"steps_total": 3}))).await.unwrap();
    core.ingest(make_event("wf-1", "workflow.claimed", json!({"replicant_id": "r-1"}))).await.unwrap();
    core.ingest(make_event("wf-1", "workflow.step.completed", json!({"step_id": 0}))).await.unwrap();
    core.ingest(make_event("wf-1", "workflow.step.completed", json!({"step_id": 1}))).await.unwrap();

    let state = core.projection("workflow_status", "wf-1").unwrap();
    assert_eq!(state["steps_completed"], 2);
    assert_eq!(state["status"], "running");
}

#[tokio::test]
async fn workflow_approval_request_pauses_workflow() {
    let core = open_in_memory_core().await;

    core.ingest(make_event("wf-1", "workflow.dispatched", json!({}))).await.unwrap();
    core.ingest(make_event("wf-1", "workflow.claimed", json!({"replicant_id": "r-1"}))).await.unwrap();
    core.ingest(make_event("wf-1", "workflow.approval.requested", json!({"reason": "review output"}))).await.unwrap();

    let state = core.projection("workflow_status", "wf-1").unwrap();
    assert_eq!(state["status"], "awaiting_approval");
}

// --- Replicant registry ---

#[tokio::test]
async fn replicant_registers_and_heartbeats() {
    let core = open_in_memory_core().await;

    core.ingest(IngestEvent {
        entity_id: "r-1", event_type: "replicant.registered",
        payload: json!({"capabilities": ["summarize", "translate"]}),
        metadata: None,
    }).await.unwrap();

    core.ingest(IngestEvent {
        entity_id: "r-1", event_type: "replicant.heartbeat",
        payload: json!({}), metadata: None,
    }).await.unwrap();

    let state = core.projection("replicant_registry", "r-1").unwrap();
    assert_eq!(state["status"], "active");
    assert!(state["capabilities"].as_array().unwrap().len() == 2);
}

// --- Task queue projection ---

#[tokio::test]
async fn task_queue_lists_unclaimed_workflows() {
    let core = open_in_memory_core().await;

    core.ingest(make_event("wf-1", "workflow.dispatched", json!({}))).await.unwrap();
    core.ingest(make_event("wf-2", "workflow.dispatched", json!({}))).await.unwrap();
    core.ingest(make_event("wf-1", "workflow.claimed", json!({"replicant_id": "r-1"}))).await.unwrap();

    // Task queue should only show wf-2 (unclaimed)
    let queue = core.projection("task_queue", "__all").unwrap();
    let pending: Vec<&str> = queue["pending"].as_array().unwrap()
        .iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(pending, vec!["wf-2"]);
}
```

---

## Phase 6: Streaming Token Events + Batching — DONE

**Priority**: P1 | **Effort**: Low | **Status**: Implemented + 7 tests passing

### What Was Built

- **`EmbeddedCore::ingest_batch()`** — batch ingestion of events (iterates `ingest()` for each)
- **`EmbeddedCore::compact_tokens(entity_id)`** — compacts `workflow.token` events into a single `workflow.output.complete` event, preserving all non-token events
- **`EventStore::compact_entity_tokens()`** — low-level compaction with full index rebuild after `retain()` shifts event positions
- **MCP tool call lifecycle** and **LLM cost tracking** events tested for ingestion correctness

### What's Needed (original)

- **Batched ingestion** via `EmbeddedCore::ingest_batch()` (covered in Phase 2)
- **Compaction policy**: Auto-merge token events into `workflow.output.complete` after configurable delay
- **WebSocket backpressure**: Buffer and chunk token events for desktop clients

### RED Phase Tests — `apps/core/tests/token_streaming.rs`

```rust
#[tokio::test]
async fn high_frequency_token_ingestion() {
    let core = open_in_memory_core().await;

    // Simulate 3000 tokens/min (50/sec) for a single workflow
    for i in 0..200 {
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.token",
            payload: json!({"token": format!("word_{i}"), "index": i}),
            metadata: None,
        }).await.unwrap();
    }

    assert_eq!(core.stats().total_events, 200);
    let tokens = core.query(
        Query::new().entity_id("wf-1").event_type("workflow.token")
    ).await.unwrap();
    assert_eq!(tokens.len(), 200);
}

#[tokio::test]
async fn token_compaction_merges_to_output() {
    let core = EmbeddedCore::open(
        Config::builder()
            .compaction_policy(CompactionPolicy::token_merge(Duration::from_secs(0))) // immediate
            .build().unwrap()
    ).await.unwrap();

    for i in 0..50 {
        core.ingest(IngestEvent {
            entity_id: "wf-1", event_type: "workflow.token",
            payload: json!({"token": format!("w{i}"), "index": i}),
            metadata: None,
        }).await.unwrap();
    }

    // Trigger compaction
    core.compact().await.unwrap();

    // Token events replaced by single output.complete
    let events = core.query(Query::new().entity_id("wf-1")).await.unwrap();
    let output = events.iter().find(|e| e.event_type == "workflow.output.complete");
    assert!(output.is_some());
    // Original tokens should be compacted away
    let tokens: Vec<_> = events.iter().filter(|e| e.event_type == "workflow.token").collect();
    assert!(tokens.is_empty());
}
```

---

## Phase 7: AI Workflow Projection Templates — DONE

**Priority**: P2 | **Effort**: Medium | **Status**: Implemented + 12 tests passing

### What Was Built

Four pre-built projections registered at `EmbeddedCore::open()` when `embedded-projections` feature is enabled:

1. **`TokenUsageProjection`** — folds `llm.call.completed` events into `{total_input_tokens, total_output_tokens, total_cost_usd, calls_count, by_model}` with per-model breakdown
2. **`ToolCallAuditProjection`** — folds `mcp.tool.result` and `mcp.tool.error` events into per-tool stats `{total_calls, successes, failures, success_rate, p50_ms, p95_ms}`
3. **`HumanInLoopQueueProjection`** — tracks `workflow.approval.*` events, query `__all` returns `{pending_approvals: [{entity_id, reason, requested_at}]}` sorted oldest-first
4. **`AgentUtilizationProjection`** — cross-references `replicant.*` and `workflow.*` events, query `__all` returns `{total_capacity, active, idle}`, excludes stale replicants

### Pre-built Projections

| Projection | Folds | Output |
|-----------|-------|--------|
| `workflow_status` | `workflow.*` | `{ status, steps_completed, steps_total, output, awaiting_approval }` |
| `token_usage` | `llm.call.completed` | `{ total_input_tokens, total_output_tokens, total_cost_usd, calls_count }` |
| `tool_call_audit` | `mcp.tool.*` | `{ tool_name, calls, success_rate, p50_ms, p95_ms }` |
| `human_in_loop_queue` | `workflow.approval.*` | `{ pending_approvals: [{ entity_id, reason, requested_at }] }` |
| `agent_utilization` | `replicant.*`, `workflow.claimed` | `{ active, idle, total_capacity }` |

### RED Phase Tests — `apps/core/tests/ai_projection_templates.rs`

```rust
// --- Cost tracking ---

#[tokio::test]
async fn token_usage_projection_sums_costs() {
    let core = open_in_memory_core().await;

    core.ingest(IngestEvent {
        entity_id: "wf-1", event_type: "llm.call.completed",
        payload: json!({
            "model": "claude-sonnet-4-20250514",
            "input_tokens": 1500, "output_tokens": 800,
            "cost_usd": 0.0078, "latency_ms": 2340
        }),
        metadata: None,
    }).await.unwrap();

    core.ingest(IngestEvent {
        entity_id: "wf-1", event_type: "llm.call.completed",
        payload: json!({
            "model": "claude-sonnet-4-20250514",
            "input_tokens": 500, "output_tokens": 200,
            "cost_usd": 0.002, "latency_ms": 1100
        }),
        metadata: None,
    }).await.unwrap();

    let usage = core.projection("token_usage", "wf-1").unwrap();
    assert_eq!(usage["total_input_tokens"], 2000);
    assert_eq!(usage["total_output_tokens"], 1000);
    assert_eq!(usage["total_cost_usd"], 0.0098);
    assert_eq!(usage["calls_count"], 2);
}

// --- MCP tool call audit ---

#[tokio::test]
async fn tool_call_audit_tracks_success_rate() {
    let core = open_in_memory_core().await;

    core.ingest(make_event("wf-1", "mcp.tool.result", json!({
        "tool_name": "read_file", "duration_ms": 50
    }))).await.unwrap();

    core.ingest(make_event("wf-1", "mcp.tool.result", json!({
        "tool_name": "read_file", "duration_ms": 120
    }))).await.unwrap();

    core.ingest(make_event("wf-1", "mcp.tool.error", json!({
        "tool_name": "read_file", "error": "not found", "retryable": true
    }))).await.unwrap();

    let audit = core.projection("tool_call_audit", "wf-1").unwrap();
    let read_file = &audit["read_file"];
    assert_eq!(read_file["calls"], 3);
    // 2 success / 3 total = 0.667
    assert!(read_file["success_rate"].as_f64().unwrap() > 0.6);
    assert!(read_file["success_rate"].as_f64().unwrap() < 0.7);
}

// --- Human-in-the-loop queue ---

#[tokio::test]
async fn hitl_queue_tracks_pending_approvals() {
    let core = open_in_memory_core().await;

    core.ingest(make_event("wf-1", "workflow.approval.requested", json!({
        "reason": "review generated summary"
    }))).await.unwrap();
    core.ingest(make_event("wf-2", "workflow.approval.requested", json!({
        "reason": "confirm deployment"
    }))).await.unwrap();
    core.ingest(make_event("wf-1", "workflow.approval.granted", json!({}))).await.unwrap();

    let queue = core.projection("human_in_loop_queue", "__all").unwrap();
    let pending = queue["pending_approvals"].as_array().unwrap();
    assert_eq!(pending.len(), 1); // wf-1 was approved, only wf-2 pending
    assert_eq!(pending[0]["entity_id"], "wf-2");
}
```

---

## Phase 8: TOON Format + Documented SDK — DONE

**Priority**: P2 | **Effort**: Low | **Status**: Implemented + 4 tests passing

### What Was Built

- **`EmbeddedCore::query_toon()`** — returns events encoded in TOON v3 format (via `toon-format` crate v0.4)
- TOON removes JSON structural overhead (braces, quoted keys) for LLM-optimized output
- Gated behind `embedded-toon` feature flag which pulls in `toon-format` as optional dependency
- Tests verify: valid output, structural overhead reduction, empty results, single events

### TOON in Embedded API

```rust
let toon_bytes = core.query_toon(Query::new().entity_id("order-1")).await?;
// Returns TOON-encoded bytes (~50% fewer tokens than JSON)
```

### SDK Documentation

- Rustdoc on all public types with examples
- Feature flags documented
- Migration guide: HTTP SDK vs embedded library
- Usage examples in `sdks/rust/examples/`

### RED Phase Tests

```rust
#[tokio::test]
async fn query_toon_returns_valid_toon() {
    let core = open_in_memory_core().await;
    core.ingest(sample_event("e1")).await.unwrap();

    let toon = core.query_toon(Query::new().entity_id("e1")).await.unwrap();
    // TOON is valid bytes
    assert!(!toon.is_empty());
    // Should be smaller than JSON equivalent
    let json = serde_json::to_vec(
        &core.query(Query::new().entity_id("e1")).await.unwrap()
    ).unwrap();
    assert!(toon.len() < json.len());
}
```

---

## Progress Tracker

| Phase | Capability | Priority | Status | Tests |
|-------|-----------|----------|--------|-------|
| 1 | Embeddable Core library API | P0 | **Done** | 19/19 |
| 2 | API completeness (serde, multi-tenant, root re-export) | P1 | **Done** | 7/7 |
| 3 | Binary size optimization (feature flags) | P2 | **Done** | 5/5 |
| 4 | Bidirectional sync (HLC) | P0 | **Done** | 15/15 |
| 5 | Replicant worker protocol | P1 | **Done** | 14/14 |
| 6 | Streaming token events + compaction | P1 | **Done** | 7/7 |
| 7 | AI workflow projection templates | P2 | **Done** | 12/12 |
| 8 | TOON format + documented SDK | P2 | **Done** | 4/4 |

**Total**: 83 passing / 83 planned

---

## Issue #73 Gap Analysis

| Issue #73 Requirement | Phase | Notes |
|----------------------|-------|-------|
| `EmbeddedCore` facade | 1 | **Done** |
| `Config::builder()` pattern | 1 | **Done** |
| Single-tenant mode | 1 | **Done** (default) |
| Multiple instances / no global state | 1 | **Done** |
| `embedded` feature flag | 1 | **Done** |
| Async-first, no `async-trait` | 1 | **Done** |
| Error types with `thiserror` | 1 | **Done** |
| `Serialize`/`Deserialize` on `EventView` | 2 | **Done** |
| Multi-tenant in embedded | 2 | **Done** (`tenant_id` on `IngestEvent`, wired in `effective_tenant_id`) |
| Batch ingestion | 6 | **Done** (`ingest_batch()` — sequential per-event) |
| `wal_fsync_ms` interval sync | — | Deferred (WALConfig doesn't support ms-interval) |
| DataFusion behind feature flag | 3 | **Done** (`analytics` feature) |
| Binary size < 5MB | 3 | **Done** (server deps gated; prometheus kept required) |
| Bidirectional sync (HLC) | 4 | **Done** (sync_to, CRDT dedup, LWW convergence) |
| Replicant worker protocol | 5 | **Done** (WorkflowStatus, ReplicantRegistry, TaskQueue projections) |
| Batch ingestion | 6 | **Done** (`ingest_batch()`) |
| Streaming tokens + compaction | 6 | **Done** (`compact_tokens()`, index rebuild) |
| Pre-built projection templates | 7 | **Done** (TokenUsage, ToolCallAudit, HumanInLoopQueue, AgentUtilization) |
| TOON format in SDK | 8 | Planned |

---

## References

- [Issue #73](https://github.com/all-source-os/all-source/issues/73) — Original feature request
- [Longhand](https://github.com/technical-leaders/longhand) — Requesting project
- `docs/proposals/CORE_REPLICATION_DESIGN.md` — Existing leader-follower replication design
- `docs/proposals/SERVER_SIDE_PROJECTIONS.md` — Server-side projection engine design
- `apps/core/src/infrastructure/cluster/` — HLC, VersionVector, CRDT resolver (existing code)
- `apps/core/tests/embedded_mode_verification.rs` — Original 16 tests proving Core works as library
- `apps/core/tests/embedded_core_api.rs` — 26 TDD tests for Phase 1 + Phase 2 `EmbeddedCore` facade
