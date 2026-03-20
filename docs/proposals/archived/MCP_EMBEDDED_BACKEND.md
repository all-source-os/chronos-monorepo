---
title: "MCP Server: Embedded Core Backend via Rustler NIF"
status: PROPOSAL
created: 2026-03-03
author: decebal
---

# MCP Server: Embedded Core Backend via Rustler NIF

## Problem

The MCP server (Elixir) talks to Core exclusively over HTTP. This means:

- **Requires a running Core server** — can't run the MCP server standalone
- **Network latency on every tool call** — even when Core is on the same machine
- **No offline mode** — Claude Desktop users need the full stack running
- **Heavy deployment** — Docker, port management, health checks, just to use MCP tools locally

## Proposal

Add `EmbeddedCore` as a second backend for the MCP server, alongside the existing HTTP client. A `CoreBackend` behaviour defines the contract. Config picks which backend is active at startup.

```
┌─────────────────────────────────────┐
│         MCP Tool Handlers           │  ← unchanged
│         (61 tools)                  │
└──────────────┬──────────────────────┘
               │ CoreBackend behaviour
       ┌───────┴───────┐
       ▼               ▼
  CoreClient      CoreEmbedded
  (Tesla/HTTP)    (Rustler NIF)
  ↓               ↓
  Remote Core     In-process EmbeddedCore
  :3900           WAL + Parquet local
```

Both backends implement the same behaviour. Tool handlers don't know or care which one is active.

## Design

### CoreBackend behaviour

Extract the contract from what `CoreClient` already exposes — 40 functions across these categories:

```elixir
defmodule McpServerElixir.Infrastructure.CoreBackend do
  @doc "Query events with filters (entity_id, event_type, since, until, limit)"
  @callback query_events(params :: map()) :: {:ok, map()} | {:error, term()}

  @doc "Ingest a single event"
  @callback ingest_event(params :: map()) :: {:ok, map()} | {:error, term()}

  @doc "Reconstruct entity state by folding events"
  @callback reconstruct_state(entity_id :: String.t(), opts :: keyword()) ::
              {:ok, map()} | {:error, term()}

  @doc "Get latest snapshot for an entity"
  @callback get_snapshot(entity_id :: String.t()) :: {:ok, map()} | {:error, term()}

  @doc "Store statistics"
  @callback get_stats() :: {:ok, map()} | {:error, term()}

  @doc "Semantic vector search"
  @callback semantic_search(params :: map()) :: {:ok, map()} | {:error, term()}

  @doc "Hybrid search (semantic + keyword)"
  @callback hybrid_search(params :: map()) :: {:ok, map()} | {:error, term()}

  # Schema operations
  @callback register_schema(params :: map()) :: {:ok, map()} | {:error, term()}
  @callback validate_schema(params :: map()) :: {:ok, map()} | {:error, term()}
  @callback list_schemas(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback get_schema(event_type :: String.t(), opts :: keyword()) ::
              {:ok, map()} | {:error, term()}
  @callback list_schema_versions(event_type :: String.t()) :: {:ok, map()} | {:error, term()}
  @callback set_compatibility(event_type :: String.t(), mode :: String.t()) ::
              {:ok, map()} | {:error, term()}

  # Operational
  @callback compact_storage(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback storage_stats(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback partition_info(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback wal_status() :: {:ok, map()} | {:error, term()}
  @callback backup_create(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback backup_restore(backup_id :: String.t()) :: {:ok, map()} | {:error, term()}
  @callback backup_list(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback health_deep() :: {:ok, map()} | {:error, term()}
  @callback performance_report(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback audit_log(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback get_cluster_status() :: {:ok, map()} | {:error, term()}

  # Analytics
  @callback analytics_frequency(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback analytics_summary(opts :: keyword()) :: {:ok, map()} | {:error, term()}
  @callback analytics_correlation(opts :: keyword()) :: {:ok, map()} | {:error, term()}
end
```

### CoreClient (existing HTTP backend)

Wraps the existing `CoreClient` module with the behaviour. No logic changes — just add `@behaviour CoreBackend` and normalize signatures to drop the `client` first argument (it becomes module state or process state).

```elixir
defmodule McpServerElixir.Infrastructure.CoreClient do
  @behaviour McpServerElixir.Infrastructure.CoreBackend

  # Existing Tesla HTTP implementation, unchanged
  # The `client` (Tesla struct) is held in module state or passed through
end
```

### CoreEmbedded (new NIF backend)

Thin Elixir module that delegates to Rustler NIFs:

```elixir
defmodule McpServerElixir.Infrastructure.CoreEmbedded do
  @behaviour McpServerElixir.Infrastructure.CoreBackend

  use Rustler, otp_app: :mcp_server_elixir, crate: "core_nif"

  # NIFs — Rustler generates the bridge
  def nif_open(_config), do: :erlang.nif_error(:not_loaded)
  def nif_query(_ref, _params), do: :erlang.nif_error(:not_loaded)
  def nif_ingest(_ref, _params), do: :erlang.nif_error(:not_loaded)
  def nif_get_stats(_ref), do: :erlang.nif_error(:not_loaded)
  def nif_get_snapshot(_ref, _entity_id), do: :erlang.nif_error(:not_loaded)
  def nif_reconstruct_state(_ref, _entity_id, _opts), do: :erlang.nif_error(:not_loaded)
  def nif_shutdown(_ref), do: :erlang.nif_error(:not_loaded)
  # ... one per Core operation

  # Behaviour implementation delegates to NIFs
  @impl true
  def query_events(params) do
    case nif_query(core_ref(), params) do
      {:ok, _} = ok -> ok
      {:error, _} = err -> err
    end
  end

  @impl true
  def ingest_event(params) do
    case nif_ingest(core_ref(), params) do
      {:ok, _} = ok -> ok
      {:error, _} = err -> err
    end
  end

  # ...

  defp core_ref do
    :persistent_term.get(:embedded_core_ref)
  end
end
```

### Rust NIF crate

A new Rustler crate at `apps/mcp-server-elixir/native/core_nif/`:

```rust
use rustler::{Env, NifResult, ResourceArc, Term};
use allsource_core::embedded::{EmbeddedCore, Config, IngestEvent, Query};
use std::sync::Arc;
use tokio::runtime::Runtime;

struct CoreResource {
    core: Arc<EmbeddedCore>,
    rt: Arc<Runtime>,
}

#[rustler::nif]
fn nif_open(config_map: Term) -> NifResult<ResourceArc<CoreResource>> {
    let rt = Arc::new(Runtime::new().unwrap());
    let config = decode_config(config_map)?;
    let core = rt.block_on(EmbeddedCore::open(config))
        .map_err(|e| rustler::Error::Term(Box::new(e.to_string())))?;
    Ok(ResourceArc::new(CoreResource {
        core: Arc::new(core),
        rt,
    }))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_query(resource: ResourceArc<CoreResource>, params: Term) -> NifResult<Term> {
    let query = decode_query(params)?;
    let events = resource.rt.block_on(resource.core.query(query))
        .map_err(|e| rustler::Error::Term(Box::new(e.to_string())))?;
    // Return as list of maps matching CoreClient's response shape
    Ok(encode_events(events))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_ingest(resource: ResourceArc<CoreResource>, params: Term) -> NifResult<Term> {
    let event = decode_ingest_event(params)?;
    let result = resource.rt.block_on(resource.core.ingest(event))
        .map_err(|e| rustler::Error::Term(Box::new(e.to_string())))?;
    Ok(encode_ingest_result(result))
}

#[rustler::nif]
fn nif_get_stats(resource: ResourceArc<CoreResource>) -> NifResult<Term> {
    let stats = resource.rt.block_on(resource.core.stats());
    Ok(encode_stats(stats))
}

#[rustler::nif]
fn nif_shutdown(resource: ResourceArc<CoreResource>) -> NifResult<()> {
    resource.rt.block_on(resource.core.shutdown());
    Ok(())
}
```

Key Rustler details:
- `schedule = "DirtyCpu"` for blocking operations — prevents NIF from stalling the BEAM scheduler
- `ResourceArc<CoreResource>` holds the `EmbeddedCore` instance across NIF calls
- One Tokio runtime per resource (NIFs are sync, embedded Core is async)
- Response encoding must match the shapes `CoreClient` returns so tool handlers see no difference

### Backend selection

```elixir
# config/runtime.exs
core_mode = System.get_env("CORE_MODE", "remote")

case core_mode do
  "embedded" ->
    data_dir = System.get_env("CORE_DATA_DIR", "data/mcp-embedded")
    config :mcp_server_elixir,
      core_backend: McpServerElixir.Infrastructure.CoreEmbedded,
      embedded_config: %{
        data_dir: data_dir,
        node_id: 1,
        wal_fsync_interval_ms: 100,
        parquet_flush_interval_secs: 300
      }

  "remote" ->
    config :mcp_server_elixir,
      core_backend: McpServerElixir.Infrastructure.CoreClient,
      core_url: System.get_env("ALLSOURCE_CORE_URL", "http://localhost:3900")
end
```

### Tool handler wiring

Currently tool handlers receive `state` which contains `state.core_client` (a Tesla struct). Change this to `state.backend` (a module):

```elixir
# Before (in McpTools)
case CoreClient.query_events(state.core_client, params) do

# After
case state.backend.query_events(params) do
```

The `state.core_client` Tesla struct only matters for the HTTP backend. The embedded backend doesn't need it. Each backend manages its own connection state internally.

### Startup in Application supervisor

```elixir
# application.ex
backend_mod = Application.get_env(:mcp_server_elixir, :core_backend)

children = [
  {Phoenix.PubSub, name: McpServerElixir.PubSub},
  # Only start WebSocket client in remote mode
  if(backend_mod == CoreClient,
    do: {McpServerElixir.Infrastructure.CoreWebSocketClient, []},
    else: nil
  ),
  # Initialize embedded Core if in embedded mode
  if(backend_mod == CoreEmbedded,
    do: {McpServerElixir.Infrastructure.CoreEmbedded.Supervisor, []},
    else: nil
  ),
  {McpServerElixir.Context.ConversationContext, []},
  {McpServerElixir.Server, []}
] |> Enum.reject(&is_nil/1)
```

### Cloud sync (optional)

When running in embedded mode, the MCP server can periodically sync to a remote Core:

```elixir
# CoreEmbedded.SyncWorker — GenServer that runs on a timer
defmodule McpServerElixir.Infrastructure.CoreEmbedded.SyncWorker do
  use GenServer

  def handle_info(:sync, state) do
    case CoreEmbedded.nif_sync_pull(core_ref(), state.remote_url) do
      {:ok, stats} -> Logger.info("Synced: #{inspect(stats)}")
      {:error, reason} -> Logger.warning("Sync failed: #{reason}")
    end
    schedule_sync(state.interval_ms)
    {:noreply, state}
  end
end
```

This uses the `embedded-sync` feature's HTTP pull/push transport — the same mechanism desktop ↔ cloud sync uses.

## Feature coverage

Which embedded Core features map to which MCP tools:

| MCP Tool Category | Embedded API | Feature Flag |
|-------------------|-------------|--------------|
| Query (query_events, sample_events) | `core.query(Query)` | `embedded` |
| Ingest (ingest_event, import_events) | `core.ingest(IngestEvent)` | `embedded` |
| State (reconstruct_state, get_snapshot) | `core.query()` + fold / `core.projection()` | `embedded` |
| Search (semantic_search, hybrid_search) | Not in embedded API yet | `vector-search`, `keyword-search` |
| Schema (register, validate, list) | Not in embedded API yet | Needs new embedded methods |
| Analytics (frequency, summary, correlation) | Not in embedded API yet | `analytics` via DataFusion |
| Operational (compact, WAL, backup) | Partial — `core.stats()`, `core.shutdown()` | `embedded` |
| Sync | `core.sync_to()`, `events_for_sync()` | `embedded-sync` |
| TOON output | `core.query_toon()` | `embedded-toon` |
| Tool audit | `McpToolTracker` | `embedded-projections` |

**Day 1 coverage**: Query, ingest, state reconstruction, stats, TOON — covers the most-used tools (~15 of 61).

**Gaps to fill later**: Search (needs fastembed/tantivy in embedded), schema registry, analytics (needs DataFusion in embedded), operational tools. These can return `{:error, :not_supported_in_embedded}` initially, falling through to a remote Core if available.

## Hybrid mode (future)

A natural extension: use embedded for reads (fast, local), forward writes to remote Core, and sync back. Or use embedded as a local cache that syncs bidirectionally:

```
CORE_MODE=hybrid

embedded (local reads) ←─ sync ─→ remote Core (source of truth)
```

This is already possible with `embedded-sync` but would need wiring in the backend layer.

## Implementation phases

### Phase 1: Extract CoreBackend behaviour
- Define `CoreBackend` behaviour module
- Make `CoreClient` implement it
- Thread `state.backend` through tool dispatch instead of `state.core_client`
- **No NIF, no Rust** — pure refactor, all tests still pass
- Estimated: ~2 stories

### Phase 2: Rustler NIF crate
- Create `native/core_nif/` Rustler crate
- Implement `nif_open`, `nif_query`, `nif_ingest`, `nif_get_stats`, `nif_get_snapshot`, `nif_shutdown`
- Response encoding to match `CoreClient` shapes
- `CoreEmbedded` module implementing `CoreBackend`
- Estimated: ~3 stories

### Phase 3: Wiring and config
- `CORE_MODE=embedded|remote` config switch
- `CoreEmbedded.Supervisor` for lifecycle (open on start, shutdown on stop)
- Startup selection in Application supervisor
- Estimated: ~1 story

### Phase 4: Sync and hybrid
- `SyncWorker` for periodic cloud sync
- Hybrid mode (embedded reads, remote writes)
- Estimated: ~2 stories

### Phase 5: Extended API coverage
- Add schema, search, analytics to embedded API
- Wire through NIF
- Estimated: ~3 stories

## Constraints

- **Apple Silicon**: Rustler NIF compiles natively — no QEMU cross-compilation issue (unlike Docker)
- **BEAM scheduler**: All NIF calls that touch disk or do compute must use `schedule = "DirtyCpu"` to avoid blocking the scheduler
- **Erlang NIF lifecycle**: `ResourceArc` prevents the `EmbeddedCore` from being dropped while references exist. Shutdown must be explicit via `nif_shutdown` in Application stop callback.
- **Response shape compatibility**: The NIF must return data in the same shape as `CoreClient` HTTP responses. Tool handlers must not need to know which backend they're talking to.

## Files to create/modify

| File | Action |
|------|--------|
| `lib/mcp_server_elixir/infrastructure/core_backend.ex` | Create — behaviour definition |
| `lib/mcp_server_elixir/infrastructure/core_client.ex` | Modify — add `@behaviour`, normalize signatures |
| `lib/mcp_server_elixir/infrastructure/core_embedded.ex` | Create — NIF wrapper |
| `lib/mcp_server_elixir/infrastructure/core_embedded/supervisor.ex` | Create — lifecycle management |
| `native/core_nif/Cargo.toml` | Create — Rustler crate |
| `native/core_nif/src/lib.rs` | Create — NIF implementations |
| `lib/mcp_server_elixir/protocol/mcp_tools.ex` | Modify — `state.core_client` → `state.backend` |
| `lib/mcp_server_elixir/server.ex` | Modify — initialize backend from config |
| `lib/mcp_server_elixir/application.ex` | Modify — conditional supervisor children |
| `config/runtime.exs` | Modify — `CORE_MODE` switch |
| `mix.exs` | Modify — add `rustler` dep |
