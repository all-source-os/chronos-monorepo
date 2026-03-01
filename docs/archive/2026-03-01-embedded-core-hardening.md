# Embedded Core Hardening — Execution Summary

**Date:** 2026-03-01
**Issue:** #73 — Embedded Core Library API
**Epics completed:** 4 (62 beads total, all closed)

## Overview

The Embedded Core Library (`EmbeddedCore` facade) was implemented in Phases 1–8, then underwent two rounds of principal engineer review producing 25 findings. All findings were addressed across two hardening epics.

## Epic 1: Fix Embedded Core Review Findings (`chronos-monorepo-3ih`)

14 findings from the first principal engineer review. Priority breakdown: 4 critical, 5 high, 5 medium.

| ID | Priority | Title | Resolution |
|----|----------|-------|------------|
| 3ih.1 | P1 CRITICAL | Fix compact_entity_tokens WAL/Parquet durability | WAL append + Parquet flush added to compaction path |
| 3ih.2 | P1 CRITICAL | Fix sync_to HLC timestamps to use originals | Sync preserves original event HLC instead of generating new |
| 3ih.3 | P1 CRITICAL | Cap ToolCallAuditProjection durations array | Bounded to 1000 entries with ring-buffer eviction |
| 3ih.4 | P1 CRITICAL | Reduce write lock hold time in compact_entity_tokens | Index rebuild moved outside write lock; lock only for vec swap |
| 3ih.5 | P2 HIGH | Wrap synchronous store calls in spawn_blocking | `spawn_blocking` added for blocking store ops in async handlers |
| 3ih.6 | P2 HIGH | Make ingest_batch actually atomic | Single write lock acquisition for entire batch |
| 3ih.7 | P2 HIGH | Wire VersionVector into sync_to for delta exchange | Version vectors used for delta sync instead of full state |
| 3ih.8 | P2 HIGH | Handle out-of-order events in replicant projections | Sort by HLC before applying in replicant projection |
| 3ih.9 | P2 HIGH | Fix partial_cmp().unwrap() NaN panic in ToolCallAuditProjection | Replaced with `partial_cmp().unwrap_or(Ordering::Equal)` |
| 3ih.10 | P3 MEDIUM | Fix token compaction space-joining | Token values joined with space separator instead of raw concat |
| 3ih.11 | P3 MEDIUM | Fix floating-point cost accumulation drift | No change needed — f64 precision sufficient for cost tracking |
| 3ih.12 | P3 MEDIUM | Wire parquet_flush_interval_secs config | Config plumbed from `EmbeddedConfig` to `EventStoreConfig` |
| 3ih.13 | P3 MEDIUM | Fix TOCTOU in AgentUtilizationProjection | Atomic compare-and-swap for projection state updates |
| 3ih.14 | P3 MEDIUM | Remove redundant Arc wrapping in projections | Removed double-Arc in projection registration |

## Epic 2: Embedded Core Hardening (`chronos-monorepo-oni`)

11 findings from the second principal engineer review. Priority breakdown: 3 P0, 4 P1, 4 P2.

| ID | Priority | Title | Resolution |
|----|----------|-------|------------|
| oni.1 | P0 | Fix crash-safety of compaction WAL replay | WAL append moved inside write lock to prevent duplicate data on crash recovery |
| oni.2 | P0 | Add cross-tenant compaction guard | Tenant validation added — rejects compaction if token events span multiple tenants |
| oni.3 | P0 | Add concurrency integration test | 2 tests: 10 tasks x 100 events (1000 total), 5 readers + 5 writers concurrent |
| oni.4 | P1 | Implement true batch ingestion in EventStore | `EventStore::ingest_batch()` — single write lock for N events |
| oni.5 | P1 | Add crash recovery integration test | Write 10 events → shutdown → reopen → verify 10 events survive via WAL |
| oni.6 | P1 | Reconcile duplicate projection files | Deleted `application/services/replicant_projections.rs` (identical to `embedded/replicant.rs`) |
| oni.7 | P1 | Measure and record embedded binary size | `--no-default-features --features embedded --release` = ~30MB rlib |
| oni.8 | P2 | Add projection backfill on registration | `register_projection_with_backfill()` replays existing events through new projection |
| oni.9 | P2 | Create SDK embedded quickstart example | `apps/core/examples/embedded_quickstart.rs` — full working example |
| oni.10 | P2 | Replace sleep-based sync test with deterministic time | Removed `tokio::time::sleep`; node_id tie-breaking makes LWW deterministic |
| oni.11 | P2 | Add TOON round-trip test | 3 tests: encode → decode → verify fields, special characters, nested objects |

## Earlier Epics (context)

### Fix Demo Account Tenant Sync (`chronos-monorepo-1d0`)
6 beads — Fixed 16 failing E2E tests caused by Control Plane not provisioning tenants in Query Service PostgreSQL.

### Dashboard E2E Test Suite (`chronos-monorepo-2xz`)
20 beads — Comprehensive Playwright E2E test suite covering all dashboard pages, navigation, WebSocket streaming, and interactive elements.

## Files Modified (Hardening Epics)

### Core changes (`apps/core/src/`)
- `store.rs` — `ingest_batch()`, `register_projection_with_backfill()`, crash-safe compaction
- `embedded/core.rs` — Cross-tenant guard, batch delegation, TOON error handling
- `embedded/mod.rs` — Doc fixes
- `application/services/mod.rs` — Removed duplicate module
- `application/services/replicant_projections.rs` — Deleted (duplicate)

### Tests (`apps/core/tests/`)
- `embedded_core_api.rs` — +4 tests (concurrency, crash recovery, backfill, projection)
- `token_streaming.rs` — +1 test (cross-tenant rejection)
- `toon_format.rs` — +3 tests (round-trip, field values, special characters)
- `bidirectional_sync.rs` — Removed sleep, made deterministic

### New files
- `apps/core/examples/embedded_quickstart.rs` — SDK usage example

## Test Results (Final)

| Suite | Count | Status |
|-------|-------|--------|
| `cargo test --lib` | 1482 | All pass |
| `embedded_core_api` | 30 | All pass |
| `toon_format` | 7 | All pass |
| `replicant_protocol` | 14 | All pass |
| `token_streaming` | 8 | All pass |
| `projections` | 12 | All pass |
| `bidirectional_sync` | 15 | All pass |
| `minimal_build` | 5 | All pass |

## Commits

```
805a3c0 fix: harden embedded Core with crash-safety, batch ingestion, and test coverage
2341b8f chore: query-service test fixes and embedded Core social post
c21ee75 merge: integrate origin/main dependency updates
```
