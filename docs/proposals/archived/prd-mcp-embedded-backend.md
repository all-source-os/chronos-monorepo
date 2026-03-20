# PRD: MCP Embedded Backend

## Overview

Add an embedded Core backend to the MCP server so it can run standalone without a remote Core server. A `CoreBackend` behaviour abstracts the interface. The existing HTTP client (`CoreClient`) and a new Rustler NIF client (`CoreEmbedded`) both implement it. Config picks which backend is active at startup via `CORE_MODE=remote|embedded`.

This enables offline MCP usage from Claude Desktop — single process, no Docker, no network, local WAL+Parquet storage, with optional cloud sync.

## Goals

- Extract a `CoreBackend` behaviour covering all 40 `CoreClient` public functions
- Refactor all 61 MCP tool handlers to dispatch through the behaviour instead of calling `CoreClient` directly
- Build a Rustler NIF crate that wraps `EmbeddedCore` and implements all behaviour callbacks
- Support `CORE_MODE=embedded` startup that opens an in-process Core with local persistence
- Unsupported operations in embedded mode return clear error messages
- Optional cloud sync via `SyncClient` on a periodic timer

## Quality Gates

### Epic-Level (run once on epic completion)
- `make ci` — full CI pipeline (Rust + Go + Elixir)

### Story-Level (checked per story)
- **Elixir stories:** `cd apps/mcp-server-elixir && mix test` passes
- **Rust NIF stories:** `cd apps/mcp-server-elixir/native/core_nif && cargo build && cargo test` passes
- **Integration stories:** MCP server starts with `CORE_MODE=embedded` and responds to `tools/list`

## User Stories

### US-001: Define CoreBackend behaviour [Backend]
**Description:** As a developer, I want a behaviour module that defines the contract for all Core operations so that multiple backends can implement it.

**Acceptance Criteria:**
- [ ] File `lib/mcp_server_elixir/infrastructure/core_backend.ex` exists
- [ ] Defines `@callback` for all 40 public functions from `CoreClient` (query_events, ingest_event, reconstruct_state, get_snapshot, get_stats, semantic_search, hybrid_search, compact_storage, storage_stats, partition_info, wal_status, backup_create, backup_restore, backup_list, health_deep, performance_report, audit_log, get_cluster_status, register_schema, validate_schema, list_schemas, get_schema, list_schema_versions, set_compatibility, analytics_frequency, analytics_summary, analytics_correlation)
- [ ] All callbacks use normalized signatures: no `client` first arg, opts default to `[]`
- [ ] Module compiles: `mix compile --warnings-as-errors` succeeds
- [ ] `cd apps/mcp-server-elixir && mix test` passes (no behaviour used yet, just defined)

### US-002: Make CoreClient implement CoreBackend [Backend]
**Description:** As a developer, I want `CoreClient` to implement the `CoreBackend` behaviour so it can be used interchangeably with future backends.

**Acceptance Criteria:**
- [ ] `CoreClient` declares `@behaviour McpServerElixir.Infrastructure.CoreBackend`
- [ ] All 40 callbacks are implemented (wrap existing functions, drop `client` arg — store Tesla client in process/module state)
- [ ] `CoreClient` is now a GenServer (or uses Agent/persistent_term) to hold the Tesla client internally
- [ ] Public API changes from `CoreClient.query_events(client, params)` to `CoreClient.query_events(params)`
- [ ] All existing tests still pass: `cd apps/mcp-server-elixir && mix test`
- [ ] No callers still use the old 2-arity `CoreClient.query_events(client, params)` form — grep confirms zero matches

### US-003: Thread backend module through MCP server state [Backend]
**Description:** As a developer, I want the MCP server to hold a `backend` module in its state so tool handlers can dispatch to whichever backend is configured.

**Acceptance Criteria:**
- [ ] `McpServerElixir.Server` state includes `backend: module()` field
- [ ] Backend module is read from `Application.get_env(:mcp_server_elixir, :core_backend)` at init
- [ ] Default value is `McpServerElixir.Infrastructure.CoreClient` (backwards compatible)
- [ ] `config/config.exs` and `config/runtime.exs` set `:core_backend` config key
- [ ] Backend module is passed through to `McpTools.call_tool/4` via state
- [ ] `cd apps/mcp-server-elixir && mix test` passes

### US-004: Refactor tool handlers to use backend module [Backend]
**Description:** As a developer, I want all 61 tool handlers in `McpTools` to call `state.backend.function()` instead of `CoreClient.function()` so they work with any backend.

**Acceptance Criteria:**
- [ ] All `CoreClient.` calls in `mcp_tools.ex` are replaced with `state.backend.` calls
- [ ] Grep for `CoreClient.query_events\|CoreClient.ingest_event\|CoreClient.get_stats\|CoreClient.get_snapshot\|CoreClient.reconstruct_state` in `mcp_tools.ex` returns zero matches
- [ ] Grep for `state.core_client` in `mcp_tools.ex` returns zero matches
- [ ] All tool handlers that called `CoreClient` now call through `state.backend`
- [ ] `cd apps/mcp-server-elixir && mix test` passes — all 571 tests green
- [ ] MCP server starts and responds to `tools/list` JSON-RPC request (manual smoke test or existing test)

### US-005: Create Rustler NIF crate scaffold [Backend]
**Description:** As a developer, I want a Rustler NIF crate that compiles and loads into the BEAM so I can bridge EmbeddedCore to Elixir.

**Acceptance Criteria:**
- [ ] Directory `apps/mcp-server-elixir/native/core_nif/` exists with `Cargo.toml` and `src/lib.rs`
- [ ] `Cargo.toml` depends on `allsource-core` with features `["embedded", "embedded-toon", "embedded-projections", "embedded-sync"]` and `rustler`
- [ ] `src/lib.rs` defines `rustler::init!` with module name `Elixir.McpServerElixir.Infrastructure.CoreNif`
- [ ] Implements a single test NIF: `fn nif_ping() -> String` that returns `"pong"`
- [ ] `mix.exs` adds `{:rustler, "~> 0.36"}` to deps and configures the NIF crate
- [ ] `CoreNif` Elixir module exists at `lib/mcp_server_elixir/infrastructure/core_nif.ex` with `use Rustler` and `def nif_ping(), do: :erlang.nif_error(:not_loaded)`
- [ ] `CoreNif.nif_ping()` returns `"pong"` in `iex -S mix`
- [ ] `cd apps/mcp-server-elixir/native/core_nif && cargo build` succeeds
- [ ] `cd apps/mcp-server-elixir && mix test` passes (NIF loads without breaking existing tests)

### US-006: Implement core NIF functions [Backend]
**Description:** As a developer, I want NIF functions that wrap EmbeddedCore's full API so the Elixir backend can call them.

**Acceptance Criteria:**
- [ ] `nif_open(config_map)` — opens `EmbeddedCore` with config, returns `ResourceArc`; stores core ref in `:persistent_term`
- [ ] `nif_shutdown(ref)` — calls `core.shutdown()`, removes from `:persistent_term`
- [ ] `nif_query(ref, params_map)` — calls `core.query(Query)`, returns `{:ok, %{events: [...], count: N}}`
- [ ] `nif_ingest(ref, params_map)` — calls `core.ingest(IngestEvent)`, returns `{:ok, %{id: uuid}}`
- [ ] `nif_get_stats(ref)` — calls `core.stats()`, returns `{:ok, stats_map}`
- [ ] `nif_get_snapshot(ref, entity_id)` — calls `core.projection("snapshot", entity_id)`, returns `{:ok, snapshot}` or `{:error, "not found"}`
- [ ] `nif_reconstruct_state(ref, entity_id, opts)` — queries all events for entity, folds into state map
- [ ] `nif_query_toon(ref, params_map)` — calls `core.query_toon(Query)`, returns TOON string
- [ ] `nif_list_schemas(ref)` — returns schemas from Core's schema registry
- [ ] `nif_register_schema(ref, params)` — registers schema in Core
- [ ] `nif_validate_schema(ref, params)` — validates event against schema
- [ ] `nif_get_schema(ref, event_type)` — returns schema for event type
- [ ] `nif_semantic_search(ref, params)` — calls Core's vector search (or returns `{:error, "not available"}` if feature disabled)
- [ ] `nif_hybrid_search(ref, params)` — calls Core's hybrid search (or returns `{:error, "not available"}`)
- [ ] `nif_compact_storage(ref)` — triggers compaction
- [ ] `nif_storage_stats(ref)` — returns WAL/Parquet stats
- [ ] `nif_wal_status(ref)` — returns WAL status
- [ ] `nif_health_deep(ref)` — returns health check map
- [ ] `nif_analytics_frequency(ref, opts)` — returns frequency analytics (or `{:error, "not available"}` without DataFusion)
- [ ] `nif_analytics_summary(ref, opts)` — returns summary analytics
- [ ] `nif_analytics_correlation(ref, opts)` — returns correlation analytics
- [ ] All blocking NIF functions use `schedule = "DirtyCpu"` to avoid blocking the BEAM scheduler
- [ ] Response maps match the shapes `CoreClient` returns (tool handlers must see no difference)
- [ ] `cd apps/mcp-server-elixir/native/core_nif && cargo build` succeeds
- [ ] `cd apps/mcp-server-elixir/native/core_nif && cargo test` passes

### US-007: Implement CoreEmbedded behaviour module [Backend]
**Description:** As a developer, I want a `CoreEmbedded` module that implements `CoreBackend` by delegating to the NIF functions.

**Acceptance Criteria:**
- [ ] File `lib/mcp_server_elixir/infrastructure/core_embedded.ex` exists
- [ ] Declares `@behaviour McpServerElixir.Infrastructure.CoreBackend`
- [ ] All 40 behaviour callbacks implemented — each delegates to the corresponding `CoreNif.nif_*` function
- [ ] Unsupported operations (backup_create, backup_restore, backup_list, partition_info, performance_report, audit_log, get_cluster_status, list_schema_versions, set_compatibility) return `{:error, "not supported in embedded mode"}`
- [ ] `CoreEmbedded.Supervisor` module exists — GenServer that calls `CoreNif.nif_open(config)` on init and `CoreNif.nif_shutdown(ref)` on terminate
- [ ] Config read from `Application.get_env(:mcp_server_elixir, :embedded_config)` with keys: `data_dir`, `node_id`, `wal_fsync_interval_ms`, `parquet_flush_interval_secs`
- [ ] `cd apps/mcp-server-elixir && mix test` passes

### US-008: Add CORE_MODE config switch and conditional startup [Backend]
**Description:** As a developer, I want `CORE_MODE=embedded|remote` to select the backend at startup so the MCP server can run in either mode.

**Acceptance Criteria:**
- [ ] `config/runtime.exs` reads `CORE_MODE` env var (default `"remote"`)
- [ ] `CORE_MODE=remote` sets `:core_backend` to `CoreClient` (existing behaviour, backwards compatible)
- [ ] `CORE_MODE=embedded` sets `:core_backend` to `CoreEmbedded` and configures `:embedded_config` from env vars (`CORE_DATA_DIR`, `CORE_NODE_ID`)
- [ ] `application.ex` conditionally starts `CoreWebSocketClient` only in remote mode
- [ ] `application.ex` conditionally starts `CoreEmbedded.Supervisor` only in embedded mode
- [ ] Default `CORE_MODE=remote` — existing deployments unchanged
- [ ] MCP server starts with `CORE_MODE=embedded CORE_DATA_DIR=/tmp/test-embedded mix run --no-halt` without errors
- [ ] MCP server starts with `CORE_MODE=remote` (default) and works as before
- [ ] `cd apps/mcp-server-elixir && mix test` passes

### US-009: End-to-end embedded mode smoke test [Integration]
**Description:** As a developer, I want an integration test that starts the MCP server in embedded mode, calls tools, and verifies responses.

**Acceptance Criteria:**
- [ ] Test file `test/mcp_server_elixir/integration/embedded_mode_test.exs` exists
- [ ] Test tagged `@tag :embedded` (excluded from default test run, run explicitly)
- [ ] Test starts `CoreEmbedded.Supervisor` with in-memory config (no `data_dir`)
- [ ] Test calls `McpTools.call_tool("ingest_event", %{"entity_id" => "test-1", "event_type" => "test.created", "payload" => %{"k" => "v"}}, state, :toon)` — asserts `{:ok, _}`
- [ ] Test calls `McpTools.call_tool("query_events", %{"entity_id" => "test-1"}, state, :toon)` — asserts response contains the ingested event
- [ ] Test calls `McpTools.call_tool("get_stats", %{}, state, :toon)` — asserts `{:ok, _}` with event count > 0
- [ ] Test calls an unsupported tool (e.g., `"backup_create"`) — asserts `{:error, "not supported in embedded mode"}`
- [ ] Test passes: `cd apps/mcp-server-elixir && mix test test/mcp_server_elixir/integration/embedded_mode_test.exs`

### US-010: Add periodic cloud sync worker [Integration]
**Description:** As a developer, I want embedded mode to optionally sync with a remote Core on a timer so local data can replicate to the cloud.

**Acceptance Criteria:**
- [ ] `CoreNif` exposes `nif_sync(ref, remote_url, node_id)` — calls `SyncClient::new(url, node_id).sync(&core)`, returns `{:ok, %{pushed: N, pulled: N, conflicts: N}}`
- [ ] NIF uses `schedule = "DirtyCpu"` for the sync call
- [ ] File `lib/mcp_server_elixir/infrastructure/core_embedded/sync_worker.ex` exists as a GenServer
- [ ] SyncWorker reads `CORE_SYNC_URL` and `CORE_SYNC_INTERVAL_MS` (default `60_000`) from env
- [ ] If `CORE_SYNC_URL` is not set, SyncWorker does not start (no-op)
- [ ] SyncWorker calls `CoreNif.nif_sync/3` on the configured interval
- [ ] Logs sync stats at info level: `"[SyncWorker] Synced: pushed=N, pulled=N, conflicts=N"`
- [ ] Logs sync errors at warning level without crashing
- [ ] SyncWorker is started as a child of `CoreEmbedded.Supervisor` (only in embedded mode)
- [ ] `cd apps/mcp-server-elixir && mix test` passes
- [ ] `cd apps/mcp-server-elixir/native/core_nif && cargo build` succeeds
