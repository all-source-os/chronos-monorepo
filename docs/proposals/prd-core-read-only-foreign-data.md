# PRD: Read-Only Core Against a Foreign Data Directory

**Issue:** [#130 — Core: Replay existing WAL/Parquet files on startup from foreign data dir](https://github.com/all-source-os/all-source/issues/130)
**Status:** Draft — awaiting review
**Author:** session drafting
**Target release:** v0.19 (minor, additive)

## Overview

Let the `allsource-core` binary boot in a **read-only query mode** against a data directory produced by another process (typically a Tauri / desktop app that embeds `allsource-core` as a library). In this mode Core replays existing WAL + Parquet files, serves queries over HTTP / WebSocket / MCP, and rejects every write path with HTTP 403. It performs no bootstrap, no tenant creation, no schema registration, no auth enforcement, and no background compaction.

The target use case is debugging: point a stock `ghcr.io/all-source-os/allsource-core:latest` container at `~/.longhand/data` and ask Claude Code (via MCP) or a browser to query the events in place, without the app having to expose them over HTTP itself.

## Problem Statement

Today `allsource-core` **already replays on startup** — `EventStore::with_config` at `apps/core/src/store.rs:247-292` reads every Parquet row from `ALLSOURCE_DATA_DIR/storage/`, recovers the WAL from `ALLSOURCE_DATA_DIR/wal/`, re-indexes each event, and re-runs all registered projections. The issue is not that replay is missing. The issue is that none of it fires in the scenario the user actually needs:

```bash
docker run --rm -v "/longhand/data:/data" -e DATA_DIR=/data -p 3900:3900 \
  ghcr.io/all-source-os/allsource-core:latest
# Health endpoint: {"total_events": 0}
# Despite /longhand/data containing 400+ Parquet files and an active WAL
```

There are **four independent failure modes**, each of which alone produces the zero-events symptom:

1. **Env var name.** Core reads `ALLSOURCE_DATA_DIR`. The user (and the Docker image docs, and anyone copying from similar projects) naturally types `DATA_DIR`. When `ALLSOURCE_DATA_DIR` is unset, `EventStoreConfig::from_env_vars` at `store.rs:1729` falls all the way through to `Self::default()` — the `"in-memory"` branch — and writes `Persistence: NONE (in-memory only)` to the log. The container is running in-memory over a full on-disk dataset and nobody notices because the log line is one line among dozens at boot.

2. **Directory layout.** Core expects the data directory to contain two subdirectories: `storage/` (Parquet files) and `wal/` (WAL segments). An embedded client is free to lay out its data however it wants — Longhand 0.13.x predates the subdir convention used by the current binary, and puts files at the root of its data dir. Even with `ALLSOURCE_DATA_DIR` set correctly, Core reads an empty `<dir>/storage` and `<dir>/wal`.

3. **Tenant filter.** Every event carries a `tenant_id`. Every read-path HTTP handler filters results by the caller's JWT `tenant_id` claim. The bootstrap tenant Core creates at startup (`ALLSOURCE_BOOTSTRAP_TENANT`, default `default`) has no relationship to the tenant the embedded client wrote against. Even after a successful replay, an admin token scoped to `default` would see zero events from a foreign app.

4. **Schema drift.** An embedded client on `allsource-core` 0.13.x may have written events in an older payload shape. Core 0.17.x's decoder does not refuse with a visible error — depending on the mismatch it silently skips events, yielding another flavor of "0 events."

This PRD solves (1), (2), and (3) as a single coherent feature. Schema drift (4) is explicitly deferred — see [Out of Scope](#out-of-scope) below.

## Goals

- **G1.** `docker run -v /data:/data -e ALLSOURCE_DATA_DIR=/data -e CORE_READ_ONLY=true <image>` replays the data dir and `/health` reports the actual event count, regardless of whether the data was written with subdir layout or flat layout.
- **G2.** `GET /api/v1/events/query` (and every other read endpoint) returns events from that foreign dir without the caller needing to know the original tenant ID or provide a JWT.
- **G3.** Every write endpoint (POST events, POST tenants, DELETE snapshots, etc.) returns HTTP 403 with an explicit "read-only mode" reason, not a 500 or a silent success that gets reverted on restart.
- **G4.** Read-only mode is opt-in. Default deployments are unchanged — no behavior change for the production Fly.io Core, the CI test matrix, or any existing docker-compose setup.
- **G5.** An operator who starts Core read-only sees a loud, unambiguous startup banner. The log line and the `/health` response both advertise the mode so it's visible in monitoring.

## Non-Goals

- Writable mode against a foreign dir. If you want to write, use normal mode and let Core own the directory.
- Cross-version schema compat (0.13 → 0.17). Separate ticket, separate release.
- Automatic tenant remapping. Read-only mode exposes events as-is; there is no "re-home events to tenant X" transformation.
- Schema registry changes. Read-only mode does not register schemas and does not validate writes (since it rejects them all).
- Replication. A read-only foreign-dir Core does not act as a replication follower — replication has its own separate leader/follower env vars.

## Proposed Design

### User-facing shape

```bash
docker run --rm \
  -v "/longhand/data:/data:ro" \
  -e ALLSOURCE_DATA_DIR=/data \
  -e CORE_READ_ONLY=true \
  -p 3900:3900 \
  ghcr.io/all-source-os/allsource-core:latest
```

Startup banner (stderr):

```
🌟 AllSource Core v0.19.0 starting...
   Starting as LEADER
⚠️  CORE_READ_ONLY=true — serving /data/ in read-only query mode
⚠️    auth disabled, writes rejected with 403, no bootstrap/projections/compaction
⚠️    DO NOT expose this port to untrusted networks
   Persistence: read-only replay (storage_dir=/data/storage, wal_dir=/data/wal)
   📂 Loading 12834 persisted events...
   ✅ Successfully loaded 12834 events from storage
   📂 Recovered 47 new events from WAL (12881 total)
   ✅ Read-only mode ready — listening on 0.0.0.0:3900
```

`GET /health` (extended):

```json
{
  "status": "ok",
  "read_only": true,
  "total_events": 12881,
  "data_dir": "/data",
  "layout": "flat"
}
```

Any write request:

```
POST /api/v1/events
→ 403 Forbidden
{ "error": "read-only mode: writes are disabled (CORE_READ_ONLY=true)" }
```

### Env var surface

| Env var | Type | Default | Purpose |
|---|---|---|---|
| `CORE_READ_ONLY` | `"true"` / `"1"` / `"yes"` | `false` | Master switch. All other behavior below is gated on this. |
| `ALLSOURCE_DATA_DIR` | path | unset | Existing. In read-only mode, also accepted as `DATA_DIR` (see alias below). |
| `DATA_DIR` | path | unset | **New alias.** When `CORE_READ_ONLY=true` and `ALLSOURCE_DATA_DIR` is unset, fall back to `DATA_DIR`. Does **not** apply outside read-only mode — existing deployments are unaffected. |
| `ALLSOURCE_READ_ONLY_LAYOUT` | `"auto"` / `"subdirs"` / `"flat"` | `"auto"` | How to interpret the data dir. See [Layout auto-detection](#layout-auto-detection). |

**Precedence with existing flags:**
- `CORE_READ_ONLY=true` forces `ALLSOURCE_AUTH_DISABLED=true` internally, regardless of the caller's setting. The two flags compose: setting `ALLSOURCE_AUTH_DISABLED=true` alone does not imply read-only.
- `CORE_READ_ONLY=true` forces replication to `disabled`, regardless of `ALLSOURCE_REPLICATION_ENABLED`. A read-only node cannot be a leader (no writes to ship) and the WAL receiver path is irrelevant.
- `CORE_READ_ONLY=true` skips `SystemBootstrap::try_initialize` entirely — no `__system` tenant directory, no consumer registry writes.

### Layout auto-detection

When `ALLSOURCE_READ_ONLY_LAYOUT=auto` (the default):

1. If `<data_dir>/storage/` **exists and is a directory**, use subdir layout: `storage_dir=<data_dir>/storage`, `wal_dir=<data_dir>/wal`. This matches normal-mode Core and clients that already use the convention.
2. Else if `<data_dir>` directly contains `*.parquet` files **or** a `*.wal` / `*.log` file, use flat layout: `storage_dir=<data_dir>`, `wal_dir=<data_dir>`.
3. Else fail fast with a clear error listing what was inspected and what was missing. Do **not** silently fall back to in-memory.

Operators can override the detection with `ALLSOURCE_READ_ONLY_LAYOUT=subdirs` or `flat` when auto-detect picks wrong (e.g., a directory that contains both `storage/` and loose parquet files).

### Auth / tenant behavior in read-only mode

- Every HTTP handler that currently calls `require_permission` or `auth_ctx.tenant_id()` short-circuits to "admin on the requested tenant." Practically, this is implemented by reusing the existing `is_dev_mode()` path in `apps/core/src/infrastructure/security/middleware.rs:166-196` and having `CORE_READ_ONLY=true` flip `DEV_MODE_ENABLED` at startup the same way `ALLSOURCE_AUTH_DISABLED` already does.
- Tenant filtering on read endpoints is bypassed by treating the synthetic dev-mode context as having `Role::Admin` (existing behavior of `dev_mode_auth_context()`).
- The startup banner tells the operator this is happening.

### Write gate

Every route handler for a write endpoint gains a single guard at the top:

```rust
if state.read_only {
    return Err((StatusCode::FORBIDDEN, "read-only mode: writes disabled".into()));
}
```

Instead of sprinkling this across ~30 handlers, we add a tower middleware that runs **after** auth but **before** the route handler, classifies the request as write/read using the existing `is_write_request(method, path)` at `api_v1.rs:548`, and returns 403 for writes when `state.read_only` is true. That function is already the canonical "is this a mutation" check — reusing it keeps the classification in one place.

### Background work gating

In read-only mode, skip spawning the following tokio tasks in `main.rs`:

- WAL shipper (replication leader)
- WAL receiver (replication follower)
- Compaction manager (no new writes → nothing to compact)
- Webhook dispatch loop
- Periodic Parquet flush (no in-memory buffer to flush)
- Projection rebuild worker (projections still run during the initial replay, just not afterwards)

The gating is an `if !config.read_only { ... }` block around each `tokio::spawn`. No change to the task implementations themselves — this keeps read-only a pure configuration mode.

## Code Touchpoints

| File | What changes |
|---|---|
| `apps/core/src/store.rs` | Add `read_only: bool` to `EventStoreConfig`. Extend `from_env_vars` to read `CORE_READ_ONLY` + `DATA_DIR` alias + `ALLSOURCE_READ_ONLY_LAYOUT`. New helper `detect_layout(data_dir: &Path) -> LayoutResult` with unit tests. |
| `apps/core/src/store.rs` (`with_config`) | When `read_only == true`, skip registering the two default projections that mutate on write (`EventCounterProjection` can stay; `EntitySnapshotProjection` stays since it only reads). Skip `CompactionManager` construction. |
| `apps/core/src/infrastructure/security/middleware.rs` | Have `CORE_READ_ONLY=true` also flip the `DEV_MODE_ENABLED` LazyLock (already done for `ALLSOURCE_AUTH_DISABLED` in #131). Extend the warning message with a read-only branch. |
| `apps/core/src/infrastructure/web/api_v1.rs` | New tower middleware `read_only_write_guard` inserted into the router layer. Reuses `is_write_request(method, path)` (already exists at line 548). Returns 403 for write requests when `AppState.read_only == true`. |
| `apps/core/src/infrastructure/web/api_v1.rs` | `AppState` gains `pub read_only: bool`. |
| `apps/core/src/infrastructure/web/health.rs` | `/health` response includes `read_only`, `data_dir`, `layout` fields. |
| `apps/core/src/main.rs` | Read `CORE_READ_ONLY` early. When true: skip `SystemBootstrap::try_initialize`, skip replication shipper/receiver spawn, skip compaction loop spawn, print the read-only startup banner. |
| `docs/guides/` | New guide `CORE_READ_ONLY_MODE.md` with the docker invocation, troubleshooting for each of the four failure modes, and a pointer from `docs/README.md`. |
| `docs/deployment/DOCKER.md` | Paragraph pointing at the new guide. |

**Estimated diff size:** ~400 LOC across the core crate, ~200 LOC of new tests, ~250 LOC of docs. No dependency changes.

## Test Plan

### Unit tests (run in `cargo test -p allsource-core`)

- `test_from_env_reads_core_read_only_flag` — various casings, `true`/`1`/`yes`, parity with `ALLSOURCE_AUTH_DISABLED`.
- `test_from_env_falls_back_to_data_dir_alias_only_in_read_only` — `DATA_DIR=/x` without `CORE_READ_ONLY` is ignored; with `CORE_READ_ONLY=true` it's honored.
- `test_detect_layout_subdirs` — fixture dir with `storage/` subdir → `LayoutResult::Subdirs`.
- `test_detect_layout_flat` — fixture dir with bare `*.parquet` → `LayoutResult::Flat`.
- `test_detect_layout_empty_errors` — fixture dir with nothing → `LayoutResult::Empty` with actionable error.
- `test_detect_layout_explicit_override` — `ALLSOURCE_READ_ONLY_LAYOUT=subdirs` overrides auto.
- `test_read_only_config_forces_auth_disabled` — setting `read_only=true` in the config flips the dev-mode LazyLock.

### Integration tests (run in `cargo test -p allsource-core --test read_only_foreign_data`)

- `test_replays_foreign_dir_subdirs` — spin up Core with a tempdir that has pre-written `storage/*.parquet` + `wal/*.log`, assert `/health` shows the right event count and `/api/v1/events/query` returns them all.
- `test_replays_foreign_dir_flat` — same but flat layout.
- `test_write_rejected_with_403` — send `POST /api/v1/events` in read-only mode, assert 403 with the read-only error body.
- `test_tenant_filter_bypassed_in_read_only` — events were written under tenant `longhand-abc`, query with no JWT, expect all events back.
- `test_health_reports_mode` — `/health.read_only == true` and `/health.total_events` matches the replayed count.

### End-to-end (deferred stretch per earlier decision, tracked as follow-up)

- `tools/e2e/read_only_foreign_data_spec.ts` — full docker + curl flow matching the issue reproduction. Run manually on release; not part of default CI.

### Manual verification against Longhand

The core shipping requirement is: the exact reproduction from issue #130 returns a non-zero event count when the env vars include `ALLSOURCE_DATA_DIR=/data` and `CORE_READ_ONLY=true`. This is done by hand on the issue author's own data dir before merging.

## Rollout

- **v0.19.0** — feature lands behind `CORE_READ_ONLY=false` default. Existing deployments see no behavior change. New guide published. Release notes call out the new flag as "debugging / local query layer."
- **v0.19.1** (opportunistic) — if telemetry shows wide adoption, extend the `/health` response to include a machine-parseable `mode: "read-only"` field for orchestrators that want to gate their own health checks on it.
- **No deprecation window needed** — this is purely additive.
- **No migration needed** — no schema changes, no breaking changes to existing env vars, no changes to the default binary behavior.

## Security Notes

Read-only mode intentionally disables auth. This is documented loudly:

- Startup banner warns on every boot.
- `/health` advertises the mode so any health-check-based monitoring sees it.
- The guide has a "do not expose this to untrusted networks" callout.

The operational assumption is that a read-only Core is run on a developer's laptop, inside a trusted container network, or behind an upstream reverse proxy that provides its own auth. If a user exposes a read-only Core directly to the internet, that is a misconfiguration we cannot prevent — same risk profile as `ALLSOURCE_DEV_MODE=true` / `ALLSOURCE_AUTH_DISABLED=true` today.

If (only if) someone later wants "read-only but still auth-required," we would add `CORE_READ_ONLY_REQUIRE_AUTH=true` as a secondary flag. Not in this PRD.

## Out of Scope

| Concern | Why deferred | Follow-up |
|---|---|---|
| Cross-version schema compat (0.13 → 0.17) | Scope explosion — event format migration is a separate multi-release effort | File new issue once someone hits a concrete incompat |
| Writable mode against foreign dir | Conflicts with the app's ownership of its own data files | Not planned |
| Automatic tenant remapping | Requires a policy decision per-deployment, not a per-binary toggle | Not planned |
| Partial replays / time-bounded views | Useful but a different feature — filtering is a query concern, not a boot concern | Separate query-side RFC |
| Schema-lax mode (skip events that fail to decode) | Suggested as a stretch goal in the earlier discussion | New issue, filed after this PRD ships |

## Open Questions

1. **Tenant visibility in `/api/v1/tenants`** — in read-only mode, should `GET /api/v1/tenants` enumerate every tenant seen in the replayed events, or return an empty list and let the operator infer from event payloads? *Proposal: enumerate, with a `source: "replayed"` marker. Cheap if we piggyback on the existing `TenantRegistryProjection`.*
2. **Prime projections on foreign data** — if the foreign data dir contains Prime events (`prime.node.created` etc.), should read-only mode also register Prime's graph projections so `/api/v1/prime/*` endpoints work? *Proposal: yes, gated on `CORE_READ_ONLY_PRIME=true` (defaults to on). Prime projections are read-only themselves. But this bleeds into schema drift since Prime's projection logic has evolved.*
3. **Parquet compaction state corruption** — when Core runs against a dir that another process is actively writing, the Parquet files can be mid-compaction. Should we document this as "quiesce the writing app first" or add file-locking detection? *Proposal: document only. File locks get complex fast and the debugging use case accepts a best-effort snapshot view.*

These three questions don't block the PRD — they each have a default answer I'll implement unless told otherwise during review.

## Implementation Order (once PRD is approved)

1. **Config plumbing** — add `read_only` to `EventStoreConfig`, extend `from_env_vars`, add layout detection + its unit tests. Smallest possible PR, lands standalone.
2. **Main.rs wiring** — skip bootstrap, skip replication, add startup banner. Gated so default deployments are unchanged.
3. **Middleware + write gate** — hook into dev-mode auth bypass, add the write-guard tower middleware, extend `/health`.
4. **Integration tests** — the five tests above. These prove the reproduction from the issue actually works.
5. **Docs** — new guide, update `docs/README.md` index, update `docs/deployment/DOCKER.md`.
6. **Manual verification against Longhand** — run against the issue author's real data dir, link the verification output from the closing comment on #130.

Each step is a separately-reviewable commit. The whole thing fits in one PR.
