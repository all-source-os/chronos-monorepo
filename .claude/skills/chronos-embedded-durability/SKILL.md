---
name: chronos-embedded-durability
description: Test AllSource Core embedded durability by running Rust integration tests that verify WAL recovery, crash recovery, and checkpoint correctness. Catches silent-no-op checkpoint bugs like issue #84. Triggers on "test embedded durability", "embedded crash recovery", "test wal recovery", "test embedded persistence", "verify embedded durability", "issue 84".
category: testing
color: red
displayName: Chronos Embedded Durability Test
---

# Chronos Embedded Durability Test

Tests the `allsource-core` Rust crate's durability guarantees directly — no Docker, no HTTP, no MCP. Exercises the exact code paths that run when `EmbeddedCore::open` is called on a directory with existing WAL/Parquet data.

## Why this exists

Issue [#84](https://github.com/all-source-os/all-source/issues/84) revealed a class of bug where:
1. WAL recovery loads events into memory correctly
2. `flush_storage()` returns `Ok(())` as a **silent no-op** (empty Parquet batch)
3. WAL is truncated unconditionally after "successful" checkpoint
4. Events exist only in memory → process exit → data loss

The existing Docker durability test (`chronos-durability`) doesn't catch this because it tests the HTTP server path, not the embedded library path. The embedded data flow test uses clean shutdown, which works fine — the bug only manifests on **unclean restart**.

## When invoked:

1. Run the targeted Rust integration tests:
   ```bash
   cd apps/core
   cargo test --test integration_tests test_wal_durability_and_recovery -- --nocapture
   cargo test --test embedded_core_api --features embedded events_survive -- --nocapture
   ```

2. If those pass, run the full embedded durability suite:
   ```bash
   cd apps/core
   cargo test --test embedded_core_api --features embedded -- --nocapture
   cargo test --test integration_tests -- --nocapture
   ```

3. If the `durability_status()` API is available, run the #84 regression suite:
   ```bash
   bash tooling/embedded-durability-test/test-embedded-durability.sh --status
   ```

4. For the full suite including all scenarios:
   ```bash
   bash tooling/embedded-durability-test/test-embedded-durability.sh
   ```

5. Analyze the output and report:
   - Which tests passed/failed
   - Whether the WAL recovery checkpoint bug (#84 pattern) is present
   - Whether events survive unclean shutdown (drop without `shutdown()`)
   - Whether Parquet files are actually written during checkpoint-on-open

## Test Matrix

The script and Rust tests cover these scenarios:

| Scenario | What's Tested | Bug #84 Relevant? |
|----------|--------------|-------------------|
| **Clean shutdown + reopen** | `shutdown()` → drop → `open()` → query | No (this path works) |
| **Drop without shutdown + reopen** | drop (no `shutdown()`) → `open()` → query | **YES — this is the #84 path** |
| **WAL-only recovery** | Write events, skip Parquet flush, reopen | **YES** |
| **Parquet-only recovery** | Flush to Parquet, delete WAL, reopen | No (different path) |
| **WAL + Parquet recovery** | Both exist on disk, reopen | Partially |
| **Checkpoint verification** | After recovery, assert Parquet files exist on disk | **YES — catches silent no-op** |
| **durability_status() after ingest** | `memory > 0, wal > 0, durable=true` | Validates fsync is working |
| **durability_status() after recovery** | `parquet_pending_batch == memory_events` | **YES — the exact #84 invariant** |
| **durability_status() warns on memory-only** | `warnings.len() > 0` for dangerous state | Runtime detection of #84 |
| **durability_status() unclean restart** | Drop → reopen → `durable=true, no warnings` | **End-to-end #84 regression** |
| **Large volume recovery** | 1000+ events, drop, reopen, verify count | Stress variant of #84 |
| **Concurrent write + kill** | Spawn writer tasks, kill mid-write, reopen | Edge case |

## Key Assertions (what distinguishes this from other durability tests)

1. **After unclean restart, `query()` returns all events** — not just the ones that were in Parquet before the crash
2. **After recovery checkpoint, Parquet files exist on disk** — `ls storage/` is not empty
3. **After recovery checkpoint, WAL can be safely truncated** — events are in Parquet, not just memory
4. **`flush_storage()` after WAL recovery writes > 0 bytes** — catches the silent no-op

## `durability_status()` API (#84 regression tests)

The other thread is adding `EmbeddedCore::durability_status()` which returns the exact internal state
that caused #84. The `--status` flag exercises this API through 5 invariant checks:

```json
{
  "memory_events": 63,
  "wal_entries": 63,
  "wal_bytes": 91000,
  "wal_sequence": 63,
  "parquet_files": 0,
  "parquet_bytes": 0,
  "parquet_pending_batch": 63,
  "durable": false,
  "warnings": ["63 events in memory but 0 in Parquet and 0 in WAL — data loss on restart"]
}
```

| Test | Invariant Checked | #84 Signal |
|------|-------------------|------------|
| `durability_status_after_ingest` | `memory_events > 0 && wal_entries > 0 && durable == true` | If `wal_entries == 0`, fsync isn't working |
| `durability_status_after_flush` | `parquet_files > 0 && parquet_pending_batch == 0` | Parquet actually wrote to disk |
| `durability_status_after_recovery` | `memory == wal && parquet_pending_batch == memory && durable == true` | **The #84 check** — after WAL recovery, events must be in `parquet_pending_batch` before truncation |
| `durability_status_warns_on_memory_only` | `warnings.len() > 0` when events in memory but not WAL/Parquet | Would have flagged #84 at runtime |
| `durability_status_survives_unclean_restart` | Drop (no shutdown) → reopen → `durable == true && warnings.is_empty()` | End-to-end #84 regression |

Run with:
```bash
bash tooling/embedded-durability-test/test-embedded-durability.sh --status
```

These tests expect corresponding `#[tokio::test]` functions in `apps/core/tests/embedded_core_api.rs`
that call `core.durability_status()` and assert the invariants above. The Rust tests are the source of truth;
this shell script just runs them and reports results.

## Common Failure Patterns

- **"events lost after restart"**: The #84 bug — WAL events loaded into memory but not checkpointed to Parquet before WAL truncation
- **"storage/ directory empty after recovery"**: Same root cause — `flush_storage()` is a no-op because `current_batch` was never populated from WAL recovery
- **"test_wal_durability_and_recovery fails"**: The EventStore-level recovery path has the bug
- **"events_survive_store_restart_via_wal passes but unclean test fails"**: Clean shutdown works (events go through `ingest()` → `append_event()` → `current_batch`), but recovery path doesn't populate `current_batch`

## Relationship to Other Skills

| Skill | Scope | Catches #84? |
|-------|-------|-------------|
| `chronos-durability` | Docker container restart (HTTP path) | No |
| `chronos-data-flow` | Docker stack connectivity | No |
| `chronos-data-flow-embedded` | MCP embedded backend (clean shutdown) | No |
| **`chronos-embedded-durability`** | **Rust crate crash recovery (unclean shutdown)** | **Yes** |

## Running Manually

```bash
# Quick: just the #84-relevant tests
bash tooling/embedded-durability-test/test-embedded-durability.sh --quick

# durability_status() API regression tests only
bash tooling/embedded-durability-test/test-embedded-durability.sh --status

# Full: all embedded durability tests
bash tooling/embedded-durability-test/test-embedded-durability.sh

# Or run cargo tests directly:
cd apps/core
cargo test --test integration_tests test_wal_durability -- --nocapture
cargo test --test embedded_core_api --features embedded durability_status -- --nocapture
cargo test --test embedded_core_api --features embedded events_survive -- --nocapture
```
