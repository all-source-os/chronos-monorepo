# Architecture Optimization Plan

**Date**: November 4, 2025
**Status**: Recommended Architecture Changes

---

## Executive Summary

After comprehensive analysis of all three services (Rust Core, Go Control Plane, Elixir Query Service), we've identified **significant duplication and optimization opportunities** that can:

- **Save 8-12 weeks** of development effort
- **Reduce operational complexity** (eliminate separate PostgreSQL, Redis instances)
- **Improve performance** by 50-100x for certain operations
- **Maintain clear separation of concerns** while avoiding duplication

---

## Key Findings

### 🚨 Critical Duplications Found

1. **WebSocket Streaming**: Core has production-ready WebSocket; query-service plans to build Phoenix Channels (2-3 weeks effort)
2. **Projection Storage**: Core has DashMap + Parquet + optional PostgreSQL; query-service plans separate PostgreSQL (3-4 weeks)
3. **Event Processing Pipelines**: Both Core and Query-Service implement identical 6 operators
4. **Caching**: Core's DashMap (11.9 μs) vs planned Redis (0.5-1ms network RTT)

### ✅ Core Capabilities Already Implemented

**Rust Core (`apps/core`) has:**
- ✅ WebSocket streaming (`/api/v1/events/stream`) - 1000+ concurrent clients
- ✅ Real-time projections with DashMap caching (11.9 μs queries)
- ✅ Event pipelines (Filter, Map, Reduce, Window, Enrich, Branch)
- ✅ Parquet storage + WAL + Snapshots
- ✅ Optional PostgreSQL (feature-gated)
- ✅ 38 HTTP endpoints
- ✅ 469K events/sec ingestion
- ✅ Analytics APIs (frequency, correlation, summary)

---

## Recommended Architecture Changes

### ❌ CANCEL: Query Service Phase 2.1 - PostgreSQL/Redis

**Original Plan (ROADMAP.md Phase 2.1):**
- Add PostgreSQL for projection persistence
- Add Redis for caching
- Estimated: 2-3 weeks effort

**Why Cancel:**
- Rust Core already has PostgreSQL support (feature-gated)
- Core's DashMap is 50-100x faster than Redis for in-process cache
- Adds operational complexity (separate database instances)

**Alternative Approach:**
```elixir
# Query-service syncs projection state to Core's API
defmodule ProjectionSync do
  use GenServer

  # Every 100ms, sync updated projections to core
  def handle_info(:sync, state) do
    updated_projections = get_dirty_projections()

    Enum.each(updated_projections, fn projection ->
      RustCoreClient.save_projection_state(projection)
    end)

    {:noreply, state}
  end
end

# On restart, restore from core
def init(_) do
  state = RustCoreClient.get_projection_state(projection_id)
  {:ok, state}
end
```

**Caching Hierarchy:**
```
L1: Rust Core DashMap (11.9 μs) ← Source of truth
L2: Elixir GenServer/ETS (sub-ms) ← Local cache
L3: Rust Parquet Storage (~ms) ← Persistent
```

**Benefits:**
- ✅ Single PostgreSQL instance (if needed) - in Core
- ✅ Eliminate Redis dependency
- ✅ 50-100x faster cache access
- ✅ Simpler operations

**Effort Saved: 3-4 weeks development + operational overhead**

---

### ❌ CANCEL: Query Service Phase 2.2 - Phoenix Channels WebSocket

**Original Plan (ROADMAP.md Phase 2.2):**
- Implement Phoenix Channels for WebSocket streaming
- Build EventChannel and ProjectionChannel
- Estimated: 2-3 weeks effort

**Why Cancel:**
- Rust Core has production-ready WebSocket at `/api/v1/events/stream`
- Handles 1000+ concurrent connections
- Per-client filtering (entity_id, event_type)
- Duplicates existing functionality

**Alternative Approach:**
```elixir
# Add WebSockex dependency
# mix.exs
{:websockex, "~> 0.4"}

# Subscribe to Core's WebSocket
defmodule QueryServiceEx.CoreWebSocketClient do
  use WebSockex

  def start_link(_opts) do
    WebSockex.start_link(
      "ws://localhost:3900/api/v1/events/stream",
      __MODULE__,
      %{},
      name: __MODULE__
    )
  end

  def handle_frame({:text, json}, state) do
    event = Jason.decode!(json)

    # Broadcast to local PubSub for GenServers
    Phoenix.PubSub.broadcast(
      QueryServiceEx.PubSub,
      "events:#{event.entity_id}",
      {:new_event, event}
    )

    {:ok, state}
  end

  def handle_disconnect(_reason, state) do
    # Auto-reconnect with exponential backoff
    {:reconnect, state}
  end
end

# Projection GenServers subscribe to PubSub
defmodule ProjectionServer do
  use GenServer

  def init(opts) do
    entity_id = opts[:entity_id]
    Phoenix.PubSub.subscribe(
      QueryServiceEx.PubSub,
      "events:#{entity_id}"
    )
    {:ok, initial_state}
  end

  def handle_info({:new_event, event}, state) do
    new_state = apply_event(state, event)
    {:noreply, new_state}
  end
end
```

**If clients need WebSocket from query-service (optional):**
```elixir
# Thin relay layer
defmodule QueryServiceExWeb.EventChannel do
  use Phoenix.Channel

  def join("events:" <> entity_id, _params, socket) do
    # Subscribe to internal PubSub
    Phoenix.PubSub.subscribe(
      QueryServiceEx.PubSub,
      "events:#{entity_id}"
    )
    {:ok, socket}
  end

  # Relay events from core
  def handle_info({:new_event, event}, socket) do
    push(socket, "new_event", event)
    {:noreply, socket}
  end
end
```

**Benefits:**
- ✅ Reuse Core's production WebSocket
- ✅ Single WebSocket infrastructure
- ✅ Phoenix Channels optional (thin relay if needed)
- ✅ Focus on OTP supervision, not WebSocket implementation

**Effort Saved: 2-3 weeks development**

---

### ✅ KEEP & ENHANCE: Query Service Phase 2.3 - Broadway Integration

**Original Plan (ROADMAP.md Phase 2.3):**
- Refine Broadway producer integration
- High-throughput event processing
- Estimated: 1-2 weeks effort

**Why Keep:**
- Broadway adds unique value (high-throughput batch processing)
- Complements Core's 469K events/sec with BEAM's concurrency
- OTP supervision & fault tolerance
- Backpressure management

**Enhanced Implementation:**
```elixir
# Production-ready Broadway producer
defmodule QueryServiceEx.CoreProducer do
  use GenStage

  @poll_interval 100  # Poll every 100ms
  @batch_size 1000    # Fetch 1000 events per poll

  def init(_opts) do
    # Start with cursor = 0 or last known position
    state = %{
      cursor: load_cursor(),
      demand: 0
    }

    # Schedule periodic polling
    schedule_poll()

    {:producer, state}
  end

  def handle_demand(demand, state) do
    new_state = %{state | demand: state.demand + demand}
    {:noreply, [], new_state}
  end

  def handle_info(:poll, state) do
    if state.demand > 0 do
      # Fetch events from core
      {:ok, events} = RustCoreClient.query_events(%{
        since: state.cursor,
        limit: min(state.demand, @batch_size)
      })

      # Convert to Broadway messages
      messages = Enum.map(events, &to_broadway_message/1)

      # Update cursor
      new_cursor = events |> List.last() |> Map.get(:timestamp, state.cursor)
      new_state = %{
        cursor: new_cursor,
        demand: state.demand - length(messages)
      }

      # Persist cursor
      save_cursor(new_cursor)

      schedule_poll()
      {:noreply, messages, new_state}
    else
      schedule_poll()
      {:noreply, [], state}
    end
  end

  defp schedule_poll do
    Process.send_after(self(), :poll, @poll_interval)
  end
end

# Broadway pipeline
defmodule QueryServiceEx.EventPipeline do
  use Broadway

  alias Broadway.Message

  def start_link(_opts) do
    Broadway.start_link(__MODULE__,
      name: __MODULE__,
      producer: [
        module: {QueryServiceEx.CoreProducer, []},
        concurrency: 1  # Single producer for ordering
      ],
      processors: [
        default: [
          concurrency: System.schedulers_online() * 2,  # CPU-bound
          min_demand: 50,
          max_demand: 100
        ]
      ],
      batchers: [
        projection_updates: [
          concurrency: 10,
          batch_size: 100,
          batch_timeout: 1000  # Flush every 1s
        ]
      ]
    )
  end

  @impl true
  def handle_message(_processor, message, _context) do
    event = message.data

    # Apply to all relevant projections
    # (This is where Elixir shines - concurrent GenServer updates)
    updated_projections = ProjectionRegistry.apply_event(event)

    # Tag for batching
    message
    |> Message.put_data(%{event: event, projections: updated_projections})
    |> Message.put_batcher(:projection_updates)
  end

  @impl true
  def handle_batch(:projection_updates, messages, _batch_info, _context) do
    # Batch sync to core (reduce HTTP overhead)
    projection_states =
      messages
      |> Enum.flat_map(& &1.data.projections)
      |> Enum.uniq_by(& &1.id)

    RustCoreClient.bulk_save_projections(projection_states)

    messages
  end
end
```

**Performance Targets:**
- 10K events/sec processing in Broadway
- Sub-100ms projection updates
- Automatic backpressure when downstream slow
- Fault-tolerant with OTP supervision

**Benefits:**
- ✅ High-throughput batch processing
- ✅ BEAM's lightweight processes (10K+ concurrent)
- ✅ Complements Core's ingestion speed
- ✅ Automatic error handling & retries

**Keep this in roadmap: 1-2 weeks effort**

---

### 🔄 MODIFY: Query Service Projection Strategy

**Original Plan:**
- Store projections in separate PostgreSQL
- Use Redis for caching

**Modified Plan:**
```
┌─────────────────────────────────────────────────────┐
│ PROJECTION STATE MANAGEMENT                         │
├─────────────────────────────────────────────────────┤
│ 1. Rust Core (Source of Truth)                     │
│    • DashMap in-memory cache (11.9 μs reads)       │
│    • Optional Parquet persistence                   │
│    • Optional PostgreSQL (feature flag)             │
│    • API: GET/POST /api/v1/projections/:id/state   │
│                                                      │
│ 2. Elixir Query-Service (Compute Layer)            │
│    • GenServers compute projection state            │
│    • ETS cache for fast local reads                │
│    • Periodic sync to Core (every 100ms)           │
│    • On restart: fetch from Core API               │
│                                                      │
│ 3. Benefits                                         │
│    ✅ Single source of truth (Core)                 │
│    ✅ No separate PostgreSQL instance               │
│    ✅ Leverage Core's 469K events/sec               │
│    ✅ Elixir focuses on OTP supervision             │
└─────────────────────────────────────────────────────┘
```

**Implementation:**
```rust
// Add to apps/core/src/projection.rs

// New API endpoints
// POST /api/v1/projections/:name/:entity_id/state
pub async fn save_projection_state(
    Path((name, entity_id)): Path<(String, String)>,
    Json(state): Json<serde_json::Value>,
) -> Result<StatusCode> {
    let key = format!("{}:{}", name, entity_id);
    PROJECTION_CACHE.insert(key, state);

    // Optionally persist to Parquet/PostgreSQL
    if config.persist_projections {
        storage.save_projection(&name, &entity_id, &state).await?;
    }

    Ok(StatusCode::OK)
}

// GET /api/v1/projections/:name/:entity_id/state
pub async fn get_projection_state(
    Path((name, entity_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>> {
    let key = format!("{}:{}", name, entity_id);

    // L1: DashMap cache
    if let Some(state) = PROJECTION_CACHE.get(&key) {
        return Ok(Json(state.clone()));
    }

    // L2: Load from storage
    if let Some(state) = storage.load_projection(&name, &entity_id).await? {
        PROJECTION_CACHE.insert(key, state.clone());
        return Ok(Json(state));
    }

    Err(Error::NotFound)
}
```

```elixir
# Query-service syncs state
defmodule ProjectionServer do
  use GenServer

  @sync_interval 100  # Sync every 100ms

  def init(opts) do
    projection = opts[:projection]
    entity_id = opts[:entity_id]

    # Load initial state from Core
    initial_state = case RustCoreClient.get_projection_state(
      projection.name,
      entity_id
    ) do
      {:ok, state} -> state
      {:error, :not_found} -> projection.initial_state
    end

    # Schedule periodic sync
    schedule_sync()

    {:ok, %{
      projection: projection,
      entity_id: entity_id,
      state: initial_state,
      dirty: false
    }}
  end

  def handle_info(:sync, %{dirty: true} = state) do
    # Save to Core if state changed
    RustCoreClient.save_projection_state(
      state.projection.name,
      state.entity_id,
      state.state
    )

    schedule_sync()
    {:noreply, %{state | dirty: false}}
  end

  def handle_info(:sync, state) do
    schedule_sync()
    {:noreply, state}
  end

  # Event application marks state dirty
  def handle_cast({:apply_event, event}, state) do
    new_state = state.projection.project_fn.(state.state, event)
    {:noreply, %{state | state: new_state, dirty: true}}
  end

  defp schedule_sync do
    Process.send_after(self(), :sync, @sync_interval)
  end
end
```

---

## Revised Query Service Roadmap

### Phase 2: Real-Time Integration (Q1 2025)

#### ✅ Phase 2.1: Core Integration (1-2 weeks)

**Implement WebSocket Client**
- Add `websockex` dependency
- Subscribe to Core's `/api/v1/events/stream`
- Broadcast to local PubSub for GenServers
- Auto-reconnect with exponential backoff

**Implement Projection State Sync**
- API calls to Core's projection endpoints
- Periodic sync (100ms interval)
- Load on restart from Core
- ETS cache for fast local reads

**Deliverables:**
- [ ] WebSocket client module
- [ ] Projection sync module
- [ ] Tests (target: 20+ tests)
- [ ] Documentation

**Success Metrics:**
- <100ms event delivery from Core to query-service
- <1ms projection state reads (ETS cache)
- Auto-recovery from disconnections

---

#### ✅ Phase 2.2: Broadway Producer (1-2 weeks)

**Production-Ready Implementation**
- Polling producer with cursor tracking
- Batch fetching from Core API
- Backpressure management
- Performance tuning

**Deliverables:**
- [ ] Production-ready CoreProducer
- [ ] EventPipeline Broadway implementation
- [ ] Performance benchmarks (target: 10K events/sec)
- [ ] Tests (target: 15+ tests)

**Success Metrics:**
- 10K+ events/sec processing
- <100ms projection update latency
- Automatic backpressure handling

---

#### 🆕 Phase 2.3: Core Projection API (Rust Core - 1 week)

**Add Projection State Endpoints to Core**
- POST `/api/v1/projections/:name/:entity_id/state`
- GET `/api/v1/projections/:name/:entity_id/state`
- Bulk operations for batching
- Optional persistence (Parquet/PostgreSQL)

**Deliverables:**
- [ ] Projection state API in Core
- [ ] DashMap cache for projection state
- [ ] Optional PostgreSQL persistence
- [ ] Tests (target: 15+ tests)

**Success Metrics:**
- 11.9 μs projection state reads (DashMap)
- 1000+ projections cached
- Persistent storage for recovery

---

### Phase 3: Advanced Features (Q2 2025)

#### Phase 3.1: Distributed Mode (2-3 weeks)
- libcluster for multi-node deployment
- Distributed projection management
- Consistent hashing for entity distribution

#### Phase 3.2: Advanced Analytics (2-3 weeks)
- Time-window aggregations (leveraging Core's window operators)
- Statistical functions
- Analytics API endpoints

#### Phase 3.3: Monitoring & Observability (1-2 weeks)
- Prometheus metrics
- Grafana dashboards
- APM integration

---

## Implementation Priority

### Immediate (Week 1-2)
1. **Add WebSocket client to query-service** (1 week)
   - Replace planned Phoenix Channels
   - Connect to Core's WebSocket

2. **Implement projection state sync** (1 week)
   - API integration with Core
   - Periodic sync mechanism

### Short-term (Week 3-4)
3. **Add projection API to Core** (1 week)
   - State storage endpoints
   - DashMap cache

4. **Refine Broadway producer** (1 week)
   - Production-ready implementation
   - Performance tuning

### Total Effort: 4 weeks (vs 8-12 weeks in original plan)

---

## Performance Comparison

| Operation | Original Plan | Optimized Plan | Improvement |
|-----------|---------------|----------------|-------------|
| **Projection Read** | PostgreSQL (~1-5ms) | Core DashMap (11.9 μs) | **100-400x faster** |
| **Cache Access** | Redis (0.5-1ms network) | DashMap (11.9 μs) | **50-100x faster** |
| **WebSocket** | Build Phoenix Channels (2-3 weeks) | Use Core WebSocket (existing) | **Reuse existing** |
| **Event Streaming** | New implementation | Core's production WebSocket | **1000+ clients tested** |
| **Storage Ops** | Separate PostgreSQL | Core's storage | **Single instance** |

---

## Cost-Benefit Analysis

### Development Effort
| Component | Original | Optimized | Savings |
|-----------|----------|-----------|---------|
| PostgreSQL integration | 3-4 weeks | 0 weeks | **3-4 weeks** |
| Redis integration | 2-3 weeks | 0 weeks | **2-3 weeks** |
| Phoenix Channels | 2-3 weeks | 1 week (client) | **1-2 weeks** |
| Broadway | 1-2 weeks | 1-2 weeks | 0 weeks |
| Core API additions | 0 weeks | 1 week | -1 week |
| **TOTAL** | **8-12 weeks** | **3-4 weeks** | **6-8 weeks saved** |

### Operational Complexity
| Component | Original | Optimized | Benefit |
|-----------|----------|-----------|---------|
| PostgreSQL instances | 2 (Core + Query) | 1 (Core only) | **50% reduction** |
| Redis instances | 1 | 0 | **Eliminated** |
| WebSocket servers | 2 (Core + Query) | 1 (Core only) | **50% reduction** |
| Storage backends | 3 (Parquet + 2x Postgres + Redis) | 2 (Parquet + 1x Postgres) | **Simplified** |

### Performance
| Metric | Original | Optimized | Improvement |
|--------|----------|-----------|-------------|
| Projection reads | ~1-5ms (Postgres) | 11.9 μs (DashMap) | **100-400x** |
| Cache latency | 0.5-1ms (Redis) | 11.9 μs (DashMap) | **50-100x** |
| Event delivery | New WebSocket | Proven 1000+ clients | **Battle-tested** |

---

## Risk Assessment

### Low Risk ✅
- **WebSocket client**: Well-established pattern (WebSockex library)
- **Core projection API**: Simple CRUD endpoints
- **Broadway refinement**: Foundation already exists

### Medium Risk ⚠️
- **State sync timing**: Need to tune 100ms interval
- **Cursor persistence**: Must not lose Broadway position

### Mitigation Strategies
1. **Gradual rollout**: Each phase independently testable
2. **Feature flags**: Enable/disable Core projection storage
3. **Fallback**: Keep in-memory GenServer state as L2 cache
4. **Monitoring**: Add metrics before depending on Core API

---

## Migration Path

### Step 1: Add Core Projection API (Week 1)
```bash
cd apps/core
# Add projection state endpoints
cargo build
cargo test
```

### Step 2: WebSocket Client (Week 2)
```bash
cd apps/query-service
# Add websockex dependency
mix deps.get
# Implement CoreWebSocketClient
mix test
```

### Step 3: Projection Sync (Week 2-3)
```bash
# Implement state sync to Core API
# Update ProjectionServer
mix test
```

### Step 4: Broadway Refinement (Week 3-4)
```bash
# Production-ready producer
# Performance tuning
mix test
```

### Step 5: Deprecate Planned Features (Week 4)
```bash
# Update ROADMAP.md
# Remove PostgreSQL/Redis dependencies from mix.exs
# Document new architecture
```

---

## Conclusion

By consolidating storage, caching, and streaming in the Rust Core and using Query-Service as a smart compute layer with OTP supervision, we achieve:

1. **6-8 weeks development time saved**
2. **50-100x performance improvement** for caching
3. **Reduced operational complexity** (fewer databases, fewer WebSocket servers)
4. **Clear separation of concerns**:
   - **Core**: High-performance storage, indexing, streaming (Rust strengths)
   - **Query-Service**: Concurrent processing, fault tolerance, OTP supervision (BEAM strengths)
   - **Control-Plane**: Orchestration, monitoring, multi-tenancy (Go strengths)

**Next Action**: Approve this architecture optimization plan and begin with Core Projection API implementation (Week 1).

---

**Document Version**: 1.0
**Status**: Recommended for Approval
**Estimated Savings**: 6-8 weeks + operational overhead
