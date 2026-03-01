# ADR-001: Embedded Core Library API

**Status:** Accepted
**Date:** 2026-02-28
**Supersedes:** N/A

## Context

AllSource Core runs as an HTTP server (axum) on port 3900. Applications like Tauri desktop apps and CLI tools need to embed Core as a library without running a server process. The raw `EventStore` API requires constructing internal value objects (`EventType`, `EntityId`, `TenantId`) and understanding Rust-specific types.

Issue #73 requested an ergonomic facade for embedding Core.

## Decision

Implement `EmbeddedCore` — a thin facade over `EventStore` behind an `embedded` feature flag.

### Key design choices:

1. **String-based API**: `IngestEvent` and `Query` accept `&str` / `String` instead of value objects. Conversion happens internally via `Event::from_strings()`.

2. **Feature flag gating**: `#[cfg(feature = "embedded")]` gates the entire `embedded` module. The axum `IntoResponse` impl is gated with `#[cfg(not(feature = "embedded-only"))]` so the embedded feature doesn't force axum compilation.

3. **Facade, not fork**: `EmbeddedCore` delegates to `EventStore` methods. No parallel implementation.
   - `open()` → `EventStore::with_config(EventStoreConfig::production(...))`
   - `ingest()` → `Event::from_strings()` + `store.ingest()`
   - `ingest_batch()` → `store.ingest_batch()` (single write lock)
   - `query()` → `store.query(QueryEventsRequest{...})` → `Vec<EventView>`
   - `shutdown()` → `store.flush_storage()`

4. **Owned config builder**: `EmbeddedConfig` uses a builder pattern with `data_dir()`, `single_tenant()`, `wal_enabled()`, etc.

5. **Zero-copy input types**: `IngestEvent<'a>` borrows `&str` to avoid allocation at the call site. `Query` owns `String` fields since it's held across await points.

6. **Sub-features for optional modules**: `embedded-streaming`, `embedded-replicant`, `embedded-toon` gate token streaming, multi-node sync, and TOON serialization respectively.

## Consequences

### Positive
- Tauri/desktop apps embed Core with ~10 lines of setup code
- No HTTP overhead for local event operations
- Same durability guarantees (WAL + Parquet) as server mode
- ~30MB rlib binary size (arrow + parquet dominate)
- Single-tenant mode auto-fills `tenant_id`, reducing boilerplate

### Negative
- API surface to maintain alongside the HTTP handlers
- Feature flag matrix increases CI complexity
- Sub-features (streaming, replicant, toon) create 2^4 = 16 possible feature combinations

### Risks mitigated during hardening
- **Crash-safety**: WAL append moved inside write lock in compaction to prevent duplicate data on crash recovery
- **Cross-tenant corruption**: Compaction validates all token events share the same tenant_id
- **Batch atomicity**: `ingest_batch()` acquires write lock once for N events instead of N separate acquisitions
- **Projection backfill**: `register_projection_with_backfill()` replays historical events through newly registered projections

## Alternatives considered

1. **gRPC interface**: Higher overhead, requires protobuf tooling, unnecessary for in-process use.
2. **C FFI (`extern "C"`)**: Would enable non-Rust embedders but adds unsafe boundary. Can be added later on top of this facade.
3. **Separate crate**: Rejected — the facade is thin enough to live inside `allsource-core` behind a feature flag.
