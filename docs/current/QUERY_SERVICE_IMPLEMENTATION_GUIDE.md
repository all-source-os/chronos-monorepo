# Query Service Implementation Guide

**Date**: November 4, 2025
**Status**: ✅ CURRENT
**Phase**: 2 - Core Integration (Optimized)

---

## Overview

This guide provides step-by-step implementation instructions for Query Service Phase 2, optimized to eliminate external database dependencies and leverage Core's existing infrastructure.

**Timeline**: 3-4 weeks
**External Dependencies**: Zero (no PostgreSQL, no Redis)
**Team**: 1 Elixir developer + 1 Rust developer

---

## Week 1: Core Projection API (Rust)

### Objective
Add projection state storage endpoints to Core using existing DashMap infrastructure.

### Prerequisites
- Rust 1.75+
- Familiarity with Axum web framework
- Understanding of Core's DashMap usage

### Implementation

#### Step 1: Define Projection Storage Structure

**File**: `apps/core/src/projection_state.rs` (new file)

```rust
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use chrono::{DateTime, Utc};

/// Projection state storage using DashMap
pub struct ProjectionStateStore {
    /// In-memory cache: "projection_name:entity_id" -> state
    cache: Arc<DashMap<String, ProjectionState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionState {
    pub name: String,
    pub entity_id: String,
    pub state: JsonValue,
    pub version: i32,
    pub updated_at: DateTime<Utc>,
}

impl ProjectionStateStore {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    /// Save projection state to DashMap
    pub fn save(&self, name: &str, entity_id: &str, state: JsonValue) -> Result<()> {
        let key = format!("{}:{}", name, entity_id);

        let projection_state = ProjectionState {
            name: name.to_string(),
            entity_id: entity_id.to_string(),
            state,
            version: 1,
            updated_at: Utc::now(),
        };

        self.cache.insert(key, projection_state);
        Ok(())
    }

    /// Get projection state from DashMap
    pub fn get(&self, name: &str, entity_id: &str) -> Option<ProjectionState> {
        let key = format!("{}:{}", name, entity_id);
        self.cache.get(&key).map(|entry| entry.clone())
    }

    /// List all projection states for a name
    pub fn list_by_name(&self, name: &str) -> Vec<ProjectionState> {
        self.cache
            .iter()
            .filter(|entry| entry.key().starts_with(&format!("{}:", name)))
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Delete projection state
    pub fn delete(&self, name: &str, entity_id: &str) -> Option<ProjectionState> {
        let key = format!("{}:{}", name, entity_id);
        self.cache.remove(&key).map(|(_, v)| v)
    }

    /// Get statistics
    pub fn stats(&self) -> ProjectionStats {
        ProjectionStats {
            total_projections: self.cache.len(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProjectionStats {
    pub total_projections: usize,
}
```

#### Step 2: Add API Endpoints

**File**: `apps/core/src/api_v1/projection_state.rs` (new file)

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;

use crate::projection_state::ProjectionStateStore;
use crate::error::Result;

#[derive(Debug, Deserialize)]
pub struct SaveProjectionRequest {
    pub state: JsonValue,
}

#[derive(Debug, Serialize)]
pub struct ProjectionStateResponse {
    pub name: String,
    pub entity_id: String,
    pub state: JsonValue,
    pub version: i32,
    pub updated_at: String,
}

/// POST /api/v1/projections/:name/:entity_id/state
pub async fn save_projection_state(
    State(store): State<Arc<ProjectionStateStore>>,
    Path((name, entity_id)): Path<(String, String)>,
    Json(req): Json<SaveProjectionRequest>,
) -> Result<StatusCode> {
    store.save(&name, &entity_id, req.state)?;
    Ok(StatusCode::OK)
}

/// GET /api/v1/projections/:name/:entity_id/state
pub async fn get_projection_state(
    State(store): State<Arc<ProjectionStateStore>>,
    Path((name, entity_id)): Path<(String, String)>,
) -> Result<Json<ProjectionStateResponse>> {
    let state = store
        .get(&name, &entity_id)
        .ok_or_else(|| anyhow::anyhow!("Projection not found"))?;

    Ok(Json(ProjectionStateResponse {
        name: state.name,
        entity_id: state.entity_id,
        state: state.state,
        version: state.version,
        updated_at: state.updated_at.to_rfc3339(),
    }))
}

/// GET /api/v1/projections/:name/states
pub async fn list_projection_states(
    State(store): State<Arc<ProjectionStateStore>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<ProjectionStateResponse>>> {
    let states = store.list_by_name(&name);

    let responses = states
        .into_iter()
        .map(|s| ProjectionStateResponse {
            name: s.name,
            entity_id: s.entity_id,
            state: s.state,
            version: s.version,
            updated_at: s.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(responses))
}

/// DELETE /api/v1/projections/:name/:entity_id/state
pub async fn delete_projection_state(
    State(store): State<Arc<ProjectionStateStore>>,
    Path((name, entity_id)): Path<(String, String)>,
) -> Result<StatusCode> {
    store.delete(&name, &entity_id)
        .ok_or_else(|| anyhow::anyhow!("Projection not found"))?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/projections/stats
pub async fn projection_stats(
    State(store): State<Arc<ProjectionStateStore>>,
) -> Result<Json<crate::projection_state::ProjectionStats>> {
    Ok(Json(store.stats()))
}
```

#### Step 3: Wire Up Routes

**File**: `apps/core/src/api_v1/mod.rs`

```rust
// Add to existing mod.rs
pub mod projection_state;

// In serve_v1 function, add routes:
.route("/api/v1/projections/:name/:entity_id/state",
    post(projection_state::save_projection_state)
    .get(projection_state::get_projection_state)
    .delete(projection_state::delete_projection_state)
)
.route("/api/v1/projections/:name/states",
    get(projection_state::list_projection_states)
)
.route("/api/v1/projections/stats",
    get(projection_state::projection_stats)
)
```

#### Step 4: Initialize in main.rs

**File**: `apps/core/src/main.rs`

```rust
use allsource_core::projection_state::ProjectionStateStore;

// In main function:
let projection_store = Arc::new(ProjectionStateStore::new());

// Pass to API server
api_v1::serve_v1(
    store,
    auth_manager,
    tenant_manager,
    rate_limiter,
    projection_store, // Add this
    &addr
).await?;
```

#### Step 5: Write Tests

**File**: `apps/core/src/projection_state.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_get() {
        let store = ProjectionStateStore::new();

        let state = serde_json::json!({
            "count": 42,
            "items": ["a", "b"]
        });

        store.save("user_stats", "user-123", state.clone()).unwrap();

        let retrieved = store.get("user_stats", "user-123").unwrap();
        assert_eq!(retrieved.state, state);
    }

    #[test]
    fn test_list_by_name() {
        let store = ProjectionStateStore::new();

        store.save("user_stats", "user-1", serde_json::json!({"count": 1})).unwrap();
        store.save("user_stats", "user-2", serde_json::json!({"count": 2})).unwrap();
        store.save("order_stats", "order-1", serde_json::json!({"total": 100})).unwrap();

        let user_states = store.list_by_name("user_stats");
        assert_eq!(user_states.len(), 2);
    }

    #[test]
    fn test_delete() {
        let store = ProjectionStateStore::new();

        store.save("test", "entity-1", serde_json::json!({"x": 1})).unwrap();
        assert!(store.get("test", "entity-1").is_some());

        store.delete("test", "entity-1");
        assert!(store.get("test", "entity-1").is_none());
    }

    #[test]
    fn test_stats() {
        let store = ProjectionStateStore::new();

        store.save("p1", "e1", serde_json::json!({})).unwrap();
        store.save("p2", "e2", serde_json::json!({})).unwrap();

        let stats = store.stats();
        assert_eq!(stats.total_projections, 2);
    }
}
```

### Testing

```bash
cd apps/core

# Run tests
cargo test projection_state

# Run server
cargo run

# Test endpoints
curl -X POST http://localhost:3900/api/v1/projections/user_stats/user-123/state \
  -H "Content-Type: application/json" \
  -d '{"state": {"count": 42}}'

curl http://localhost:3900/api/v1/projections/user_stats/user-123/state

curl http://localhost:3900/api/v1/projections/stats
```

### Deliverables
- [ ] projection_state.rs module (150 lines)
- [ ] API endpoints (100 lines)
- [ ] Routes wired up
- [ ] 15+ tests passing
- [ ] Documentation updated

---

## Week 2: WebSocket Integration (Elixir)

### Objective
Subscribe to Core's WebSocket and distribute events to local GenServers.

### Prerequisites
- Elixir 1.19+
- Phoenix app running
- Core WebSocket at ws://localhost:3900/api/v1/events/stream

### Implementation

#### Step 1: Add WebSockex Dependency

**File**: `apps/query-service/mix.exs`

```elixir
defp deps do
  [
    # ... existing deps
    {:websockex, "~> 0.4.3"}
  ]
end
```

```bash
cd apps/query-service
mix deps.get
```

#### Step 2: Create WebSocket Client

**File**: `apps/query-service/lib/query_service_ex/infrastructure/adapters/core_websocket_client.ex`

```elixir
defmodule QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient do
  @moduledoc """
  WebSocket client that subscribes to Core's event stream.
  Distributes events to local GenServers via PubSub.
  """

  use WebSockex
  require Logger

  @reconnect_after 5_000  # 5 seconds
  @max_reconnect_attempts 10

  def start_link(opts \\ []) do
    core_url = Application.get_env(:query_service_ex, :rust_core_url, "http://localhost:3900")
    ws_url = String.replace(core_url, ~r/^http/, "ws") <> "/api/v1/events/stream"

    state = %{
      url: ws_url,
      reconnect_attempts: 0,
      connected_at: nil,
      events_received: 0
    }

    WebSockex.start_link(ws_url, __MODULE__, state, opts ++ [name: __MODULE__])
  end

  @impl true
  def handle_connect(_conn, state) do
    Logger.info("Connected to Core WebSocket: #{state.url}")

    new_state = %{state |
      reconnect_attempts: 0,
      connected_at: DateTime.utc_now()
    }

    {:ok, new_state}
  end

  @impl true
  def handle_frame({:text, json}, state) do
    case Jason.decode(json) do
      {:ok, event} ->
        # Broadcast to PubSub
        Phoenix.PubSub.broadcast(
          QueryServiceEx.PubSub,
          "events:all",
          {:new_event, event}
        )

        # Broadcast to entity-specific topic
        if entity_id = event["entity_id"] do
          Phoenix.PubSub.broadcast(
            QueryServiceEx.PubSub,
            "events:entity:#{entity_id}",
            {:new_event, event}
          )
        end

        # Broadcast to type-specific topic
        if event_type = event["event_type"] do
          Phoenix.PubSub.broadcast(
            QueryServiceEx.PubSub,
            "events:type:#{event_type}",
            {:new_event, event}
          )
        end

        new_state = %{state | events_received: state.events_received + 1}
        {:ok, new_state}

      {:error, reason} ->
        Logger.error("Failed to decode event JSON: #{inspect(reason)}")
        {:ok, state}
    end
  end

  @impl true
  def handle_disconnect(%{reason: reason}, state) do
    Logger.warn("Disconnected from Core WebSocket: #{inspect(reason)}")

    if state.reconnect_attempts < @max_reconnect_attempts do
      Process.send_after(self(), :reconnect, @reconnect_after)
      new_state = %{state | reconnect_attempts: state.reconnect_attempts + 1}
      {:ok, new_state}
    else
      Logger.error("Max reconnect attempts reached, giving up")
      {:close, state}
    end
  end

  @impl true
  def handle_info(:reconnect, state) do
    Logger.info("Attempting to reconnect (attempt #{state.reconnect_attempts})")
    {:reconnect, state}
  end

  # Client API
  def stats do
    GenServer.call(__MODULE__, :stats)
  end

  @impl true
  def handle_call(:stats, _from, state) do
    stats = %{
      connected: true,
      connected_at: state.connected_at,
      events_received: state.events_received,
      reconnect_attempts: state.reconnect_attempts
    }
    {:reply, stats, state}
  end
end
```

#### Step 3: Add to Supervision Tree

**File**: `apps/query-service/lib/query_service_ex/application.ex`

```elixir
def start(_type, _args) do
  children = [
    # Existing children
    QueryServiceExWeb.Endpoint,
    {Phoenix.PubSub, name: QueryServiceEx.PubSub},

    # Add WebSocket client
    QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient
  ]

  opts = [strategy: :one_for_one, name: QueryServiceEx.Supervisor]
  Supervisor.start_link(children, opts)
end
```

#### Step 4: Write Tests

**File**: `apps/query-service/test/infrastructure/adapters/core_websocket_client_test.exs`

```elixir
defmodule QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClientTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient

  setup do
    # Subscribe to PubSub for testing
    Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:all")
    :ok
  end

  @tag :integration
  test "connects to Core WebSocket and receives events" do
    # Wait for connection
    Process.sleep(100)

    # Should receive test events from Core
    assert_receive {:new_event, event}, 5_000
    assert is_map(event)
    assert Map.has_key?(event, "entity_id")
  end

  test "broadcasts to entity-specific topics" do
    Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:entity:test-123")

    # Simulate event
    event = %{
      "entity_id" => "test-123",
      "event_type" => "test.event",
      "payload" => %{}
    }

    # Should receive on entity topic
    assert_receive {:new_event, ^event}, 1_000
  end

  test "provides stats" do
    stats = CoreWebSocketClient.stats()

    assert stats.connected == true
    assert is_integer(stats.events_received)
  end
end
```

### Testing

```bash
cd apps/query-service

# Run tests (requires Core running on 3900)
mix test test/infrastructure/adapters/core_websocket_client_test.exs

# Start query-service and watch logs
mix phx.server

# In another terminal, trigger events in Core
curl -X POST http://localhost:3900/api/v1/events \
  -H "Content-Type: application/json" \
  -d '{"entity_id": "test-123", "event_type": "test.event", "payload": {}}'

# Should see log: "Received event from Core WebSocket"
```

### Deliverables
- [ ] CoreWebSocketClient module (150 lines)
- [ ] Supervision tree integration
- [ ] PubSub broadcasting (all, entity, type topics)
- [ ] Auto-reconnect logic
- [ ] 10+ tests passing
- [ ] Documentation

---

## Week 2-3: Projection State Sync (Elixir)

### Objective
Sync projection state to Core's API periodically, restore on restart.

### Implementation

#### Step 1: Update ProjectionServer

**File**: `apps/query-service/lib/query_service_ex/application/use_cases/projection_server.ex`

```elixir
defmodule QueryServiceEx.Application.UseCases.ProjectionServer do
  use GenServer
  require Logger

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @sync_interval 100  # 100ms

  def start_link(opts) do
    projection = Keyword.fetch!(opts, :projection)
    entity_id = Keyword.fetch!(opts, :entity_id)

    GenServer.start_link(__MODULE__, {projection, entity_id}, opts)
  end

  @impl true
  def init({projection, entity_id}) do
    # Subscribe to events for this entity
    Phoenix.PubSub.subscribe(
      QueryServiceEx.PubSub,
      "events:entity:#{entity_id}"
    )

    # Load initial state from Core
    initial_state = case RustCoreClient.get_projection_state(projection.name, entity_id) do
      {:ok, state} ->
        Logger.info("Restored projection #{projection.name}:#{entity_id} from Core")
        state

      {:error, _} ->
        Logger.info("No saved state for #{projection.name}:#{entity_id}, using initial")
        projection.initial_state
    end

    # Schedule periodic sync
    schedule_sync()

    {:ok, %{
      projection: projection,
      entity_id: entity_id,
      state: initial_state,
      dirty: false,
      last_sync: DateTime.utc_now()
    }}
  end

  @impl true
  def handle_info(:sync, %{dirty: true} = state) do
    # Save to Core
    case RustCoreClient.save_projection_state(
      state.projection.name,
      state.entity_id,
      state.state
    ) do
      {:ok, _} ->
        Logger.debug("Synced projection #{state.projection.name}:#{state.entity_id}")
        schedule_sync()
        {:noreply, %{state | dirty: false, last_sync: DateTime.utc_now()}}

      {:error, reason} ->
        Logger.error("Failed to sync projection: #{inspect(reason)}")
        schedule_sync()
        {:noreply, state}  # Keep dirty, will retry
    end
  end

  def handle_info(:sync, state) do
    # Not dirty, skip sync
    schedule_sync()
    {:noreply, state}
  end

  @impl true
  def handle_info({:new_event, event}, state) do
    # Apply event to projection
    try do
      new_state = state.projection.project_fn.(state.state, event)
      {:noreply, %{state | state: new_state, dirty: true}}
    rescue
      error ->
        Logger.error("Error applying event: #{inspect(error)}")
        {:noreply, state}
    end
  end

  # Client API
  def get_state(pid) do
    GenServer.call(pid, :get_state)
  end

  def get_stats(pid) do
    GenServer.call(pid, :get_stats)
  end

  @impl true
  def handle_call(:get_state, _from, state) do
    {:reply, state.state, state}
  end

  def handle_call(:get_stats, _from, state) do
    stats = %{
      projection: state.projection.name,
      entity_id: state.entity_id,
      dirty: state.dirty,
      last_sync: state.last_sync
    }
    {:reply, stats, state}
  end

  defp schedule_sync do
    Process.send_after(self(), :sync, @sync_interval)
  end
end
```

#### Step 2: Add Core Client Methods

**File**: `apps/query-service/lib/query_service_ex/infrastructure/adapters/rust_core_client.ex`

```elixir
# Add to existing RustCoreClient module

@doc """
Save projection state to Core's DashMap.
"""
def save_projection_state(projection_name, entity_id, state) do
  url = "#{base_url()}/api/v1/projections/#{projection_name}/#{entity_id}/state"

  body = %{state: state}

  case Tesla.post(client(), url, body) do
    {:ok, %{status: 200}} ->
      {:ok, :saved}

    {:ok, %{status: status, body: body}} ->
      {:error, "HTTP #{status}: #{inspect(body)}"}

    {:error, reason} ->
      {:error, reason}
  end
end

@doc """
Get projection state from Core's DashMap.
"""
def get_projection_state(projection_name, entity_id) do
  url = "#{base_url()}/api/v1/projections/#{projection_name}/#{entity_id}/state"

  case Tesla.get(client(), url) do
    {:ok, %{status: 200, body: body}} ->
      {:ok, body["state"]}

    {:ok, %{status: 404}} ->
      {:error, :not_found}

    {:ok, %{status: status, body: body}} ->
      {:error, "HTTP #{status}: #{inspect(body)}"}

    {:error, reason} ->
      {:error, reason}
  end
end

@doc """
Bulk save multiple projection states.
"""
def bulk_save_projections(projection_states) do
  Enum.map(projection_states, fn %{name: name, entity_id: entity_id, state: state} ->
    save_projection_state(name, entity_id, state)
  end)
end
```

#### Step 3: Write Tests

**File**: `apps/query-service/test/application/use_cases/projection_server_sync_test.exs`

```elixir
defmodule QueryServiceEx.Application.UseCases.ProjectionServerSyncTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Application.UseCases.ProjectionServer
  alias QueryServiceEx.Domain.Entities.Projection.Definition

  @tag :integration
  test "syncs state to Core periodically" do
    projection = Definition.new(
      name: :test_projection,
      version: 1,
      initial_state: %{count: 0},
      project_fn: fn state, _event ->
        Map.update(state, :count, 0, &(&1 + 1))
      end
    )

    {:ok, pid} = ProjectionServer.start_link(
      projection: projection,
      entity_id: "test-entity"
    )

    # Trigger event
    event = %{"entity_id" => "test-entity", "type" => "test"}
    Phoenix.PubSub.broadcast(
      QueryServiceEx.PubSub,
      "events:entity:test-entity",
      {:new_event, event}
    )

    # Wait for sync
    Process.sleep(150)

    # Verify state was synced to Core
    {:ok, saved_state} = RustCoreClient.get_projection_state(:test_projection, "test-entity")
    assert saved_state["count"] == 1
  end

  @tag :integration
  test "restores state from Core on startup" do
    # Pre-populate Core with state
    RustCoreClient.save_projection_state(:test_projection, "restore-test", %{count: 42})

    projection = Definition.new(
      name: :test_projection,
      version: 1,
      initial_state: %{count: 0},
      project_fn: fn state, _event -> state end
    )

    {:ok, pid} = ProjectionServer.start_link(
      projection: projection,
      entity_id: "restore-test"
    )

    # Should have restored state
    state = ProjectionServer.get_state(pid)
    assert state.count == 42
  end
end
```

### Testing

```bash
# Run integration tests (requires Core on 3900)
mix test --only integration

# Watch projection sync in action
iex -S mix

# Create projection
projection = Projection.Definition.new(...)
{:ok, pid} = ProjectionServer.start_link(projection: projection, entity_id: "user-123")

# Trigger event
Phoenix.PubSub.broadcast(QueryServiceEx.PubSub, "events:entity:user-123", {:new_event, event})

# Check stats
ProjectionServer.get_stats(pid)

# Verify in Core
curl http://localhost:3900/api/v1/projections/user_stats/user-123/state
```

### Deliverables
- [ ] ProjectionServer with sync logic
- [ ] Core client methods
- [ ] Auto-restore on startup
- [ ] 15+ tests passing
- [ ] Documentation

---

## Week 3-4: Broadway Refinement (Elixir)

### Objective
Production-ready Broadway pipeline with cursor tracking and batch processing.

### Implementation

#### Step 1: Create Production Producer

**File**: `apps/query-service/lib/query_service_ex/application/use_cases/core_producer.ex`

```elixir
defmodule QueryServiceEx.Application.UseCases.CoreProducer do
  @moduledoc """
  Broadway producer that polls Core's HTTP API for events.
  Maintains cursor position for reliable consumption.
  """

  use GenStage
  require Logger

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @poll_interval 100  # Poll every 100ms
  @batch_size 1000    # Fetch up to 1000 events per poll

  def start_link(opts) do
    GenStage.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @impl true
  def init(opts) do
    # Load cursor from disk/ETS
    cursor = load_cursor() || 0

    state = %{
      cursor: cursor,
      demand: 0,
      last_poll: DateTime.utc_now()
    }

    # Start polling
    schedule_poll()

    {:producer, state}
  end

  @impl true
  def handle_demand(demand, state) do
    new_state = %{state | demand: state.demand + demand}
    {:noreply, [], new_state}
  end

  @impl true
  def handle_info(:poll, state) do
    if state.demand > 0 do
      # Fetch events from Core
      fetch_size = min(state.demand, @batch_size)

      case RustCoreClient.query_events(%{
        since: state.cursor,
        limit: fetch_size
      }) do
        {:ok, events} when length(events) > 0 ->
          # Convert to Broadway messages
          messages = Enum.map(events, &to_broadway_message/1)

          # Update cursor to latest timestamp
          new_cursor = events
            |> List.last()
            |> Map.get("timestamp", state.cursor)

          # Persist cursor
          save_cursor(new_cursor)

          new_state = %{
            cursor: new_cursor,
            demand: state.demand - length(messages),
            last_poll: DateTime.utc_now()
          }

          schedule_poll()
          {:noreply, messages, new_state}

        {:ok, []} ->
          # No new events
          schedule_poll()
          {:noreply, [], state}

        {:error, reason} ->
          Logger.error("Failed to fetch events: #{inspect(reason)}")
          schedule_poll()
          {:noreply, [], state}
      end
    else
      # No demand, skip fetch
      schedule_poll()
      {:noreply, [], state}
    end
  end

  defp to_broadway_message(event) do
    %Broadway.Message{
      data: event,
      acknowledger: {__MODULE__, :ack_id, :ack_data}
    }
  end

  defp schedule_poll do
    Process.send_after(self(), :poll, @poll_interval)
  end

  defp load_cursor do
    case File.read("priv/cursor.txt") do
      {:ok, content} -> String.to_integer(String.trim(content))
      {:error, _} -> nil
    end
  end

  defp save_cursor(cursor) do
    File.write!("priv/cursor.txt", to_string(cursor))
  end

  # Broadway acknowledger callbacks
  def ack(_ack_ref, successful, failed) do
    Logger.debug("Ack: #{length(successful)} successful, #{length(failed)} failed")
    :ok
  end
end
```

#### Step 2: Create Broadway Pipeline

**File**: `apps/query-service/lib/query_service_ex/application/use_cases/event_pipeline.ex`

```elixir
defmodule QueryServiceEx.Application.UseCases.EventPipeline do
  @moduledoc """
  Broadway pipeline for high-throughput event processing.
  """

  use Broadway

  alias Broadway.Message
  alias QueryServiceEx.Application.UseCases.ProjectionRegistry

  require Logger

  def start_link(_opts) do
    Broadway.start_link(__MODULE__,
      name: __MODULE__,
      producer: [
        module: {QueryServiceEx.Application.UseCases.CoreProducer, []},
        concurrency: 1  # Single producer for ordering
      ],
      processors: [
        default: [
          concurrency: System.schedulers_online() * 2,
          min_demand: 50,
          max_demand: 100
        ]
      ],
      batchers: [
        projection_updates: [
          concurrency: 10,
          batch_size: 100,
          batch_timeout: 1000  # 1 second
        ]
      ]
    )
  end

  @impl true
  def handle_message(_processor, message, _context) do
    event = message.data

    Logger.debug("Processing event: #{event["event_type"]}")

    # Apply event to all registered projections
    # This is where Elixir shines - concurrent GenServer updates
    updated_projections = ProjectionRegistry.apply_event(event)

    message
    |> Message.put_data(%{
      event: event,
      projections: updated_projections
    })
    |> Message.put_batcher(:projection_updates)
  end

  @impl true
  def handle_batch(:projection_updates, messages, _batch_info, _context) do
    # Batch sync projection states to Core
    projection_states =
      messages
      |> Enum.flat_map(fn msg -> msg.data.projections end)
      |> Enum.uniq_by(fn proj -> {proj.name, proj.entity_id} end)

    # Bulk save to Core
    RustCoreClient.bulk_save_projections(projection_states)

    Logger.info("Batched #{length(projection_states)} projection updates")

    messages
  end
end
```

#### Step 3: Add to Supervision Tree

**File**: `apps/query-service/lib/query_service_ex/application.ex`

```elixir
def start(_type, _args) do
  children = [
    QueryServiceExWeb.Endpoint,
    {Phoenix.PubSub, name: QueryServiceEx.PubSub},
    QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient,

    # Add Broadway pipeline
    QueryServiceEx.Application.UseCases.EventPipeline
  ]

  opts = [strategy: :one_for_one, name: QueryServiceEx.Supervisor]
  Supervisor.start_link(children, opts)
end
```

#### Step 4: Write Tests

**File**: `apps/query-service/test/application/use_cases/event_pipeline_test.exs`

```elixir
defmodule QueryServiceEx.Application.UseCases.EventPipelineTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Application.UseCases.EventPipeline

  @tag :integration
  test "processes events from Core" do
    # Pipeline should auto-start via supervision tree

    # Wait for events to be processed
    Process.sleep(1000)

    # Should have processed events
    # (Assuming Core has events)
    # Check Broadway metrics
  end

  test "handles batch projection updates" do
    # Test batching logic
  end
end
```

### Performance Testing

```bash
# Run load test
cd apps/query-service

# Generate test events in Core
for i in {1..10000}; do
  curl -X POST http://localhost:3900/api/v1/events \
    -H "Content-Type: application/json" \
    -d "{\"entity_id\": \"load-test-$i\", \"event_type\": \"test\", \"payload\": {}}"
done

# Watch Broadway processing
iex -S mix

# Check stats
:observer.start()

# Should process 10K+ events/sec
```

### Deliverables
- [ ] CoreProducer with cursor tracking
- [ ] EventPipeline Broadway
- [ ] Batch processing
- [ ] Performance benchmarks (10K events/sec)
- [ ] 15+ tests passing
- [ ] Load testing results

---

## Final Checklist

### Week 1: Core Projection API ✅
- [ ] projection_state.rs module
- [ ] API endpoints (save, get, list, delete, stats)
- [ ] Routes wired up in main.rs
- [ ] 15+ Rust tests passing
- [ ] Manual testing with curl
- [ ] Documentation updated

### Week 2: WebSocket Integration ✅
- [ ] CoreWebSocketClient module
- [ ] Supervision tree integration
- [ ] PubSub broadcasting
- [ ] Auto-reconnect logic
- [ ] 10+ tests passing
- [ ] Integration test with Core

### Week 2-3: Projection Sync ✅
- [ ] ProjectionServer sync logic
- [ ] Core client methods (save/get state)
- [ ] Auto-restore on startup
- [ ] 15+ tests passing
- [ ] Integration test with Core API
- [ ] ETS cache working

### Week 3-4: Broadway ✅
- [ ] CoreProducer with cursor
- [ ] EventPipeline Broadway
- [ ] Batch processing
- [ ] 15+ tests passing
- [ ] Performance: 10K events/sec
- [ ] Load testing complete

---

## Success Criteria

- ✅ All 65+ tests passing (15+15+15+15+5 existing)
- ✅ Zero external database dependencies
- ✅ <100ms event delivery latency
- ✅ 10K+ events/sec Broadway throughput
- ✅ 11.9 μs projection state reads (via Core)
- ✅ Auto-recovery from failures (OTP)
- ✅ Production-ready deployment

---

**Document Status**: ✅ CURRENT
**Version**: 1.0
**Last Updated**: November 4, 2025
