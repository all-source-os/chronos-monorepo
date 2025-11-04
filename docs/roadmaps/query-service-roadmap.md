# Query Service Roadmap

**Date**: November 4, 2025
**Status**: ✅ CURRENT
**Version**: 2.0 (Optimized)

> **📋 Document Status**: Current, authoritative roadmap
> **⚠️ Supersedes**: [QUERY_SERVICE_ROADMAP_v1.md](../../docs/archive/2025-11-04/QUERY_SERVICE_ROADMAP_v1.md)
> **🔗 Related**: [Architecture Optimization](../../docs/current/ARCHITECTURE_OPTIMIZATION.md)

---

## Executive Summary

The Elixir Query Service has successfully completed its **Phase 1 (Core Features)** with 281 passing tests. After comprehensive architecture review, **Phase 2 has been significantly optimized** to eliminate duplication and leverage existing Core infrastructure.

**Key Changes from v1.0:**
- ❌ Cancelled: PostgreSQL/Redis (Core uses NO databases)
- ❌ Cancelled: Phoenix Channels rebuild (use Core's WebSocket)
- ✅ Optimized: 3-4 weeks timeline (from 8-12 weeks)
- ✅ Simplified: Zero external database dependencies

---

## Phase 1: Core Features ✅ COMPLETE

### Status: Production-Ready
- **Tests**: 281/281 passing (100%)
- **Coverage**: 7 doctests + 274 tests
- **Failures**: 0

### 1.1 Query DSL ✅ (54 tests)
- Fluent query building with Elixir pipes
- Predicates: eq, gt, lt, gte, lte, between, in, not_in
- Time helpers: days_ago, hours_ago, since, until
- Sorting, limiting, field projection

### 1.2 Projections ✅ (61 tests)
- GenServer-based state management
- OTP supervision for fault tolerance
- Event application to projections
- Snapshot support (in-memory)
- Current/historical state queries

### 1.3 Event Pipelines ✅ (81 tests)
- 6 operator types: Filter, Transform, Enrich, Validate, Route, Aggregate
- Batch processing
- Statistics tracking
- Error handling

### 1.4 HTTP Client ✅ (34 tests)
- Tesla-based client to Core
- Connection pooling
- Event CRUD, queries, snapshots
- Error handling

### 1.5 Phoenix HTTP API ✅ (5 tests, 11 endpoints)
- GET/POST /api/events
- POST /api/query
- GET/POST /api/projections
- GET /api/health, /api/metrics

### 1.6 Production Readiness ✅
- Docker with multi-stage build
- Mix releases
- Health checks & metrics
- Environment-based configuration

---

## Phase 2: Core Integration 📋 OPTIMIZED (3-4 weeks)

> **⚠️ MAJOR REVISION**: After architecture review, Phase 2 eliminates all external database dependencies by integrating with Core's existing infrastructure.

### 2.1 WebSocket Integration (Week 2)

**Status**: High Priority
**Effort**: 1 week (down from 2-3 weeks for Phoenix Channels)

**What Changed:**
- ❌ Original: Build Phoenix Channels WebSocket
- ✅ Optimized: Subscribe to Core's existing WebSocket

**Rationale:**
- Core has production WebSocket at `/api/v1/events/stream`
- Tested with 1000+ concurrent clients
- Saves 1-2 weeks development

**Implementation:**
```elixir
# Add dependency
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
    {:reconnect, state}  # Auto-reconnect
  end
end
```

**Deliverables:**
- [ ] CoreWebSocketClient module
- [ ] PubSub integration
- [ ] Auto-reconnect with backoff
- [ ] Tests (15+ tests)
- [ ] Optional: Thin Phoenix Channels relay (if clients need it)

**Benefits:**
- ✅ Reuse Core's production WebSocket
- ✅ 1 week vs 2-3 weeks
- ✅ Single WebSocket infrastructure
- ✅ Focus on OTP strengths

---

### 2.2 Projection State Sync (Week 2-3)

**Status**: High Priority
**Effort**: 1 week (down from 3-4 weeks for PostgreSQL/Redis)

**What Changed:**
- ❌ Original: Add PostgreSQL + Redis to query-service
- ✅ Optimized: Sync state to Core's DashMap API

**Rationale:**
- Core uses NO PostgreSQL (it's optional, feature-gated, not used)
- Core's DashMap (11.9 μs) is 50-100x faster than Redis (0.5-1ms)
- Eliminates operational complexity (no external databases)

**Architecture:**
```
L1: Core DashMap (11.9 μs) ← Source of truth
L2: Query GenServer/ETS (sub-ms) ← Local cache
L3: Core Parquet (optional) ← Cold storage

External Databases: ZERO ✅
```

**Implementation:**
```elixir
defmodule ProjectionSync do
  use GenServer

  @sync_interval 100  # 100ms

  def init(opts) do
    projection = opts[:projection]
    entity_id = opts[:entity_id]

    # Load initial state from Core on startup
    initial_state = case RustCoreClient.get_projection_state(
      projection.name,
      entity_id
    ) do
      {:ok, state} -> state
      {:error, :not_found} -> projection.initial_state
    end

    schedule_sync()

    {:ok, %{
      projection: projection,
      entity_id: entity_id,
      state: initial_state,
      dirty: false
    }}
  end

  # Sync dirty state to Core's DashMap
  def handle_info(:sync, %{dirty: true} = state) do
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

  # Event application marks dirty
  def handle_cast({:apply_event, event}, state) do
    new_state = state.projection.project_fn.(state.state, event)
    {:noreply, %{state | state: new_state, dirty: true}}
  end

  defp schedule_sync do
    Process.send_after(self(), :sync, @sync_interval)
  end
end
```

**Core API (Rust side - Week 1):**
```rust
// POST /api/v1/projections/:name/:entity_id/state
pub async fn save_projection_state(
    Path((name, entity_id)): Path<(String, String)>,
    Json(state): Json<serde_json::Value>,
) -> Result<StatusCode> {
    let key = format!("{}:{}", name, entity_id);
    PROJECTION_CACHE.insert(key, state);

    // Optionally persist to Parquet (not PostgreSQL)
    if config.persist_projections {
        parquet_storage.save_projection(&name, &entity_id, &state).await?;
    }

    Ok(StatusCode::OK)
}

// GET /api/v1/projections/:name/:entity_id/state
pub async fn get_projection_state(
    Path((name, entity_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>> {
    let key = format!("{}:{}", name, entity_id);

    // L1: DashMap cache (11.9 μs)
    if let Some(state) = PROJECTION_CACHE.get(&key) {
        return Ok(Json(state.clone()));
    }

    // L2: Load from Parquet if available
    if let Some(state) = parquet_storage.load_projection(&name, &entity_id).await? {
        PROJECTION_CACHE.insert(key, state.clone());
        return Ok(Json(state));
    }

    Err(Error::NotFound)
}
```

**Deliverables:**
- [ ] Core API endpoints (Rust - Week 1)
- [ ] ProjectionSync GenServer (Elixir)
- [ ] ETS cache for local reads
- [ ] Restore from Core on restart
- [ ] Tests (20+ Elixir, 15+ Rust)

**Benefits:**
- ✅ No PostgreSQL instance needed
- ✅ No Redis instance needed
- ✅ 50-100x faster (11.9 μs vs 0.5-1ms)
- ✅ Single source of truth (Core)
- ✅ Zero external dependencies

---

### 2.3 Broadway Integration (Week 3-4)

**Status**: High Priority (elevated from Medium)
**Effort**: 1-2 weeks (unchanged)

**What Changed:**
- ✅ Keep this plan (adds unique value)

**Rationale:**
- Broadway complements Core's 469K events/sec with BEAM concurrency
- OTP supervision & fault tolerance
- Automatic backpressure
- Adds unique value beyond Core capabilities

**Implementation:**
```elixir
# Production-ready polling producer
defmodule QueryServiceEx.CoreProducer do
  use GenStage

  @poll_interval 100
  @batch_size 1000

  def init(_opts) do
    state = %{
      cursor: load_cursor(),
      demand: 0
    }

    schedule_poll()

    {:producer, state}
  end

  def handle_demand(demand, state) do
    {:noreply, [], %{state | demand: state.demand + demand}}
  end

  def handle_info(:poll, state) do
    if state.demand > 0 do
      # Fetch events from Core
      {:ok, events} = RustCoreClient.query_events(%{
        since: state.cursor,
        limit: min(state.demand, @batch_size)
      })

      messages = Enum.map(events, &to_broadway_message/1)
      new_cursor = List.last(events)[:timestamp] || state.cursor

      save_cursor(new_cursor)

      schedule_poll()
      {:noreply, messages, %{
        cursor: new_cursor,
        demand: state.demand - length(messages)
      }}
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

  def start_link(_opts) do
    Broadway.start_link(__MODULE__,
      name: __MODULE__,
      producer: [
        module: {QueryServiceEx.CoreProducer, []},
        concurrency: 1
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
          batch_timeout: 1000
        ]
      ]
    )
  end

  def handle_message(_processor, message, _context) do
    event = message.data

    # Apply to all relevant projections
    ProjectionRegistry.apply_event(event)

    message
    |> Message.put_batcher(:projection_updates)
  end

  def handle_batch(:projection_updates, messages, _batch_info, _context) do
    # Batch sync to Core
    projection_states =
      messages
      |> Enum.flat_map(& &1.data.projections)
      |> Enum.uniq_by(& &1.id)

    RustCoreClient.bulk_save_projections(projection_states)

    messages
  end
end
```

**Deliverables:**
- [ ] Production CoreProducer
- [ ] EventPipeline Broadway
- [ ] Cursor tracking & persistence
- [ ] Performance benchmarks (10K events/sec)
- [ ] Tests (15+ tests)

**Benefits:**
- ✅ High-throughput batch processing
- ✅ BEAM's lightweight processes (10K+ concurrent)
- ✅ Automatic error handling & retries
- ✅ Complements Core's ingestion

---

## Phase 3: Advanced Features 📋 FUTURE (Q2-Q3 2025)

### 3.1 Distributed Mode (2-3 weeks)
- libcluster for multi-node
- Distributed registry
- Consistent hashing

### 3.2 Advanced Analytics (2-3 weeks)
- Leverage Core's `/api/v1/analytics/*` endpoints
- Time-window aggregations
- Statistical functions

### 3.3 Message Queue Integration (2-3 weeks)
- Kafka integration
- RabbitMQ integration

### 3.4 Monitoring & Observability (1-2 weeks)
- Prometheus exporter
- Grafana dashboards
- APM integration

### 3.5 API Documentation (1 week)
- OpenAPI spec
- Swagger UI

---

## Timeline Summary

### Original Plan (v1.0)
- Phase 2.1: PostgreSQL/Redis (5-7 weeks)
- Phase 2.2: Phoenix Channels (2-3 weeks)
- Phase 2.3: Broadway (1-2 weeks)
- **Total: 8-12 weeks**

### Optimized Plan (v2.0)
- Week 1: Core Projection API (Rust)
- Week 2: WebSocket Integration (Elixir)
- Week 2-3: Projection State Sync (Elixir)
- Week 3-4: Broadway Refinement (Elixir)
- **Total: 3-4 weeks**

**Time Savings: 6-8 weeks (67% reduction)**

---

## Success Metrics

### Phase 1 ✅ COMPLETE
- [x] 100% feature parity with Clojure
- [x] 281 tests passing (100%)
- [x] 11 HTTP API endpoints
- [x] Docker deployment
- [x] Health & metrics

### Phase 2 📋 PLANNED
- [ ] Real-time event streaming (via Core's WebSocket)
- [ ] <100ms event delivery latency
- [ ] Persistent projection state (via Core's DashMap/Parquet)
- [ ] 99.9% uptime (OTP supervision)
- [ ] Broadway >10K events/sec
- [ ] 11.9 μs projection reads (via Core)
- [ ] Zero external databases

### Phase 3 📋 FUTURE
- [ ] Multi-node distributed deployment
- [ ] Kafka/RabbitMQ integration
- [ ] Prometheus + Grafana monitoring
- [ ] OpenAPI documentation

---

## Infrastructure Requirements

### Current (Phase 1) ✅
```
- 0 PostgreSQL instances
- 0 Redis instances
- 0 External databases
- Local Parquet files (Core)
- WAL files (Core)
```

### Phase 2 (Optimized) 📋
```
- 0 PostgreSQL instances (unchanged)
- 0 Redis instances (unchanged)
- 0 External databases (unchanged)
- WebSocket client to Core
- ETS cache (local)
```

**Zero external database dependencies throughout! ✅**

---

## Risk Assessment

### Low Risk ✅
- WebSocket client (proven WebSockex library)
- Core API endpoints (simple CRUD)
- Broadway refinement (foundation exists)

### Medium Risk ⚠️
- State sync timing (may need tuning beyond 100ms)
- Network failures (need robust retry)
- Cursor persistence (must not lose position)

### Mitigation
1. Gradual rollout (test each phase independently)
2. Feature flags (enable/disable Core integration)
3. Fallback (keep GenServer in-memory as backup)
4. Monitoring (add metrics before production)
5. Load testing (validate 10K events/sec)

---

## Technical Debt & Maintenance

### Current Debt
- None (recent migration, clean architecture)

### Preventive Measures
- Maintain test coverage >85%
- Regular dependency updates
- Code review for all changes
- Documentation updates with features

---

## Team & Resources

### Skills Needed
- **Elixir/OTP**: GenServer, supervision, WebSocket clients
- **Rust**: API endpoints, DashMap, Parquet
- **WebSocket**: Real-time protocols
- **Broadway**: Stream processing

### Estimated Effort
- **Phase 2**: 3-4 weeks (1 Elixir + 1 Rust developer)
- **Phase 3**: 6-8 weeks (1 developer)

---

## Conclusion

The Query Service has successfully completed **Phase 1** with production-ready core features (281 tests, Phoenix API, OTP supervision).

**Phase 2 has been significantly optimized** to:
- ✅ Eliminate PostgreSQL/Redis (not needed)
- ✅ Reuse Core's WebSocket (don't rebuild)
- ✅ Reduce timeline from 8-12 weeks to 3-4 weeks
- ✅ Simplify to zero external database dependencies

**Recommendation**: Proceed with optimized Phase 2, starting with Core Projection API (Rust, Week 1).

---

**Document Version**: 2.0 (Optimized)
**Status**: ✅ CURRENT
**Last Updated**: November 4, 2025
**Next Review**: After Phase 2 completion
