# Query Service Roadmap & Status

**Last Updated**: November 4, 2025
**Status**: Production-Ready Core ✅ | Enhancement Phase 📋

---

## Executive Summary

The Elixir Query Service has successfully completed its **core migration** from Clojure with enhanced capabilities. We now have a production-ready foundation with 281 passing tests and comprehensive Phoenix HTTP API.

**Current State**: Phase 1 Complete (Core Features)
**Next Phase**: Phase 2 (Real-time & Distributed Features)

---

## Phase 1: Core Features ✅ COMPLETE

### 1.1 Query DSL ✅
**Status**: Production-ready with 54 tests passing

- ✅ Fluent query building with Elixir pipes
- ✅ Predicate system (eq, gt, lt, gte, lte, between, in, not_in)
- ✅ Time helpers (days_ago, hours_ago, minutes_ago, since, until)
- ✅ Sorting and limiting
- ✅ Field projection
- ✅ Entity and type filtering
- ✅ Complex query composition

**Example**:
```elixir
from_events()
|> where(event_type: "order.placed")
|> since(days_ago(7))
|> sort_by(:timestamp, :desc)
|> limit(100)
```

### 1.2 Projections ✅
**Status**: Production-ready with OTP supervision (37 tests + 24 GenServer tests)

- ✅ Projection definitions with versioning
- ✅ State management via GenServer
- ✅ Event application to projections
- ✅ Snapshot support (in-memory)
- ✅ OTP supervision for fault tolerance
- ✅ Automatic recovery from failures
- ✅ Current/historical state queries

**Example**:
```elixir
projection = Projection.Definition.new(
  name: :user_statistics,
  version: 1,
  initial_state: %{total_orders: 0},
  project_fn: fn state, event -> ... end
)

{:ok, pid} = ProjectionServer.start_link(
  projection: projection,
  entity_id: "user-123"
)
```

### 1.3 Event Pipelines ✅
**Status**: Production-ready (57 entity tests + 24 processor tests)

- ✅ Pipeline definitions with operators
- ✅ All 6 operator types:
  - Filter (event filtering)
  - Transform (data transformation)
  - Enrich (context addition)
  - Validate (validation rules)
  - Route (conditional routing)
  - Aggregate (window-based aggregation)
- ✅ Pipeline processor with batch support
- ✅ Statistics tracking
- ✅ Error handling

**Example**:
```elixir
pipeline = Pipeline.Definition.new(
  name: :payment_processor,
  version: 1,
  operators: [
    filter_op,
    transform_op,
    enrich_op,
    aggregate_op
  ]
)

{:ok, processor} = PipelineProcessor.start_link(pipeline: pipeline)
```

### 1.4 HTTP Client Integration ✅
**Status**: Production-ready with Tesla (34 tests)

- ✅ RustCoreClient module
- ✅ Health checks
- ✅ Event creation (single & batch)
- ✅ Event queries
- ✅ Connection pooling
- ✅ Error handling
- ✅ Timeout management

### 1.5 Phoenix HTTP API ✅
**Status**: Production-ready on port 3902 (5 controller tests)

**Endpoints**:
- ✅ `GET /api/health` - Health check with backend status
- ✅ `GET /api/metrics` - Runtime metrics
- ✅ `GET /api/events` - List events with filters
- ✅ `POST /api/events` - Create single event
- ✅ `POST /api/events/batch` - Batch create
- ✅ `GET /api/events/entity/:id` - Events by entity
- ✅ `GET /api/events/type/:type` - Events by type
- ✅ `POST /api/query` - Execute query (DSL or simple)
- ✅ `GET /api/projections` - List projections
- ✅ `GET /api/projections/:name` - Get projection details
- ✅ `POST /api/projections` - Create projection

### 1.6 Production Readiness ✅
**Status**: Docker-ready with health checks

- ✅ Dockerfile with multi-stage build
- ✅ Mix releases configuration
- ✅ Health check endpoint
- ✅ Metrics endpoint
- ✅ Environment-based configuration (dev/test/prod)
- ✅ Logging and telemetry foundation
- ✅ Error handling

### 1.7 Testing ✅
**Status**: Comprehensive coverage

- ✅ **281 tests passing** (7 doctests + 274 tests)
- ✅ 0 failures
- ✅ Unit tests for all entities
- ✅ Integration tests (tagged)
- ✅ Controller tests
- ✅ GenServer tests
- ✅ Pipeline tests

---

## Phase 2: Real-time & Streaming 📋 PLANNED (REVISED)

> **⚠️ ARCHITECTURE OPTIMIZATION**: After reviewing core capabilities, Phase 2 has been significantly revised to eliminate duplication and leverage existing infrastructure. See [ARCHITECTURE_OPTIMIZATION.md](/ARCHITECTURE_OPTIMIZATION.md) for full analysis.

### 2.1 Core WebSocket Integration (REVISED) 📋
**Priority**: HIGH
**Status**: Changed from "Build Phoenix Channels" to "Use Core's WebSocket"
**Effort**: 1 week (down from 2-3 weeks)

**Why Changed**:
- ❌ **Original Plan**: Build Phoenix Channels WebSocket infrastructure (2-3 weeks)
- ✅ **New Plan**: Use Core's existing production WebSocket (1 week)
- **Rationale**: Core already has `/api/v1/events/stream` handling 1000+ concurrent clients

**Goals**:
- Subscribe to Core's WebSocket stream
- Distribute events to local GenServers via PubSub
- Auto-reconnect with exponential backoff
- Optional: Thin Phoenix Channels relay (if clients need WebSocket from query-service)

**Technical Requirements**:
- Add `websockex` dependency
- Implement `CoreWebSocketClient` GenServer
- Connect to `ws://localhost:3900/api/v1/events/stream`
- Broadcast to Phoenix.PubSub for projection updates
- Handle disconnections and reconnections

**Deliverables**:
- [ ] `CoreWebSocketClient` module
- [ ] PubSub integration
- [ ] Auto-reconnect logic
- [ ] Tests (target: 15+ tests)
- [ ] Optional: Phoenix Channels relay layer

**Example Implementation**:
```elixir
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

    # Broadcast to local PubSub
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

# ProjectionServers subscribe to PubSub
defmodule ProjectionServer do
  def init(opts) do
    Phoenix.PubSub.subscribe(
      QueryServiceEx.PubSub,
      "events:#{opts[:entity_id]}"
    )
    {:ok, initial_state}
  end

  def handle_info({:new_event, event}, state) do
    new_state = apply_event(state, event)
    {:noreply, new_state}
  end
end
```

**Benefits**:
- ✅ Reuse Core's production-tested WebSocket (1000+ clients)
- ✅ 1 week effort vs 2-3 weeks building Phoenix Channels
- ✅ Single WebSocket infrastructure (Core)
- ✅ Focus on OTP supervision, not WebSocket implementation

**Dependencies**: Core's WebSocket endpoint (already exists)

---

### 2.2 Broadway Producer Integration 📋
**Priority**: HIGH (elevated from MEDIUM)
**Status**: Foundation implemented, needs production refinement

**Goals**:
- Complete Broadway pipeline integration
- Custom producer for Rust Core events
- High-throughput event processing
- Automatic acknowledgment

**Technical Requirements**:
- Refine RustCoreProducer implementation
- Add rate limiting and backpressure
- Implement acknowledge callback
- Add Broadway pipeline tests

**Deliverables**:
- [ ] Production-ready `RustCoreProducer`
- [ ] Broadway pipeline example
- [ ] Performance benchmarks (target: >10K events/sec)
- [ ] Broadway integration tests (target: 15+ tests)
- [ ] Documentation and examples

**Current Foundation**:
```elixir
# Basic structure exists in:
# lib/application/use_cases/event_pipeline_broadway.ex
```

**Dependencies**: WebSocket streaming (for real-time ingestion)

---

### 2.3 Core Projection API Integration (REVISED) 📋
**Priority**: HIGH
**Status**: Changed from "Add PostgreSQL/Redis" to "Sync with Core API"
**Effort**: 1 week (down from 3-4 weeks + ops overhead)

**Why Changed**:
- ❌ **Original Plan**: Add separate PostgreSQL + Redis to query-service
- ✅ **New Plan**: Sync projection state to Core's API
- **Rationale**:
  - Core has DashMap (11.9 μs) - 50-100x faster than Redis
  - Core has optional PostgreSQL (feature-gated)
  - Eliminates operational complexity (separate databases)

**Goals**:
- Sync projection state to Core's storage API
- Use GenServer/ETS as fast local cache (L2)
- Restore state from Core on restart
- No separate PostgreSQL or Redis instances

**Technical Requirements**:
- Implement projection state sync to Core API
- Periodic sync (every 100ms for dirty projections)
- ETS cache for fast local reads
- Core API enhancement (Rust side):
  - POST `/api/v1/projections/:name/:entity_id/state`
  - GET `/api/v1/projections/:name/:entity_id/state`

**Deliverables**:
- [ ] `ProjectionSync` GenServer module
- [ ] ETS cache for local reads
- [ ] Sync strategy (100ms interval for dirty state)
- [ ] Restore from Core on restart
- [ ] Tests (target: 20+ tests)
- [ ] Core API endpoints (Rust side - separate task)

**Caching Hierarchy**:
```
L1: Core DashMap (11.9 μs) ← Source of truth
L2: Query GenServer/ETS (sub-ms) ← Local cache
L3: Core Parquet/PostgreSQL (ms) ← Persistent
```

**Example Implementation**:
```elixir
defmodule ProjectionSync do
  use GenServer

  @sync_interval 100  # 100ms

  def init(opts) do
    projection = opts[:projection]
    entity_id = opts[:entity_id]

    # Load from Core on startup
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

  # Sync dirty state to Core
  def handle_info(:sync, %{dirty: true} = state) do
    RustCoreClient.save_projection_state(
      state.projection.name,
      state.entity_id,
      state.state
    )

    schedule_sync()
    {:noreply, %{state | dirty: false}}
  end
end
```

**Benefits**:
- ✅ No separate PostgreSQL instance (use Core's)
- ✅ No Redis needed (DashMap + ETS is faster)
- ✅ 50-100x faster cache access (11.9 μs vs 0.5-1ms)
- ✅ Single source of truth (Core)
- ✅ Simpler operations

**Dependencies**: Core Projection API (needs Rust implementation - 1 week)

---

### 2.4 Distributed Mode (libcluster) 📋
**Priority**: MEDIUM
**Status**: Not started

**Goals**:
- Multi-node deployment
- Distributed projection management
- Load balancing across nodes
- Cluster-aware PubSub

**Technical Requirements**:
- Add libcluster dependency
- Configure cluster strategy (Kubernetes, Gossip, etc.)
- Use distributed registry (via Horde or :pg)
- Update ProjectionServer for distributed mode
- Add cluster health checks

**Deliverables**:
- [ ] libcluster integration
- [ ] Distributed registry
- [ ] Node discovery configuration
- [ ] Cluster monitoring
- [ ] Distributed tests (target: 15+ tests)
- [ ] Deployment guide

**Example Config**:
```elixir
config :libcluster,
  topologies: [
    k8s: [
      strategy: Cluster.Strategy.Kubernetes,
      config: [
        kubernetes_selector: "app=query-service",
        kubernetes_node_basename: "query-service"
      ]
    ]
  ]
```

**Dependencies**: State Persistence (for shared state)

---

### 2.5 Advanced Analytics & Aggregations 📋
**Priority**: LOW
**Status**: Basic aggregation exists in pipelines

**Goals**:
- Time-series aggregations (hourly, daily, weekly)
- Windowed aggregations
- Trend detection
- Anomaly detection
- Custom metric definitions

**Technical Requirements**:
- Extend aggregate operator
- Add time-window support
- Implement sliding windows
- Add statistical functions (mean, median, percentiles)
- Create analytics API endpoints

**Deliverables**:
- [ ] Time-window aggregation operators
- [ ] Statistical functions library
- [ ] Analytics API endpoints
- [ ] Analytics tests (target: 30+ tests)
- [ ] Visualization examples

**Example Usage**:
```elixir
# Hourly order totals for last 7 days
from_events()
|> where(event_type: "order.placed")
|> since(days_ago(7))
|> aggregate(:sum, :amount, window: :hourly)
```

**Dependencies**: None (can start after Phase 2.1-2.3)

---

## Phase 3: Enterprise Features 📋 FUTURE

### 3.1 Message Queue Integrations 📋
**Priority**: MEDIUM
**Status**: Not started

**Goals**:
- Kafka producer/consumer
- RabbitMQ integration
- Event forwarding to external systems

**Technical Requirements**:
- Add broadway_kafka or kaffe
- Add broadway_rabbitmq
- Configure producers and consumers

**Deliverables**:
- [ ] Kafka integration
- [ ] RabbitMQ integration
- [ ] Configuration examples
- [ ] Integration tests

---

### 3.2 API Documentation (OpenAPI/Swagger) 📋
**Priority**: LOW
**Status**: Basic docs in README

**Goals**:
- Interactive API documentation
- OpenAPI 3.0 specification
- Swagger UI

**Technical Requirements**:
- Add open_api_spex or similar
- Annotate controllers
- Generate spec

**Deliverables**:
- [ ] OpenAPI spec
- [ ] Swagger UI endpoint
- [ ] API examples

---

### 3.3 Monitoring & Observability 📋
**Priority**: MEDIUM
**Status**: Basic telemetry exists

**Goals**:
- Prometheus metrics exporter
- Grafana dashboards
- APM integration (New Relic, Datadog)
- Distributed tracing

**Technical Requirements**:
- Add telemetry_metrics_prometheus
- Create custom metrics
- Add distributed tracing spans

**Deliverables**:
- [ ] Prometheus exporter
- [ ] Grafana dashboard templates
- [ ] APM integration guide
- [ ] Tracing implementation

---

## Implementation Priority & Roadmap (REVISED)

> **💡 OPTIMIZATION**: After architecture review, timeline reduced from 8-12 weeks to 3-4 weeks by eliminating duplication and leveraging Core capabilities.

### Q1 2025 (Immediate - Next Month)
**Focus**: Integration with Core's existing infrastructure

1. **Core WebSocket Integration (1 week)** 📋
   - WebSocket client to Core's `/api/v1/events/stream`
   - PubSub distribution to GenServers
   - Auto-reconnect logic
   - **Replaces**: Building Phoenix Channels from scratch
   - **Saves**: 1-2 weeks development

2. **Broadway Producer Refinement (1 week)** 📋
   - Production-ready polling producer
   - Cursor tracking & persistence
   - Performance tuning (target: 10K events/sec)
   - **Keeps**: Original plan (adds value)

3. **Core Projection API Integration (1 week)** 📋
   - Sync projection state to Core API
   - ETS cache for local reads
   - Restore from Core on restart
   - **Replaces**: PostgreSQL + Redis setup
   - **Saves**: 3-4 weeks development + ops overhead

4. **Core Projection API Implementation (1 week - Rust side)** 📋
   - Add projection state endpoints to Core
   - DashMap cache for 11.9 μs reads
   - Optional Parquet/PostgreSQL persistence
   - **Enables**: Query-service state sync

**Outcome**: Production-ready real-time query service integrated with Core infrastructure

**Total Effort**: 3-4 weeks (vs 8-12 weeks original plan)
**Development Savings**: 6-8 weeks
**Operational Savings**: No separate PostgreSQL, Redis, or WebSocket servers

---

### Q2 2025 (2-5 Months)
**Focus**: Distributed deployment and advanced features

5. **Distributed Mode (2-3 weeks)** 📋
   - libcluster integration
   - Distributed registry
   - Cluster monitoring

6. **Advanced Analytics (2-3 weeks)** 📋
   - Leverage Core's existing analytics APIs
   - Time-window aggregations (using Core's window operators)
   - Analytics API endpoints
   - **Note**: Core already has `/api/v1/analytics/*` endpoints

7. **Message Queue Integrations (2-3 weeks)** 📋
   - Kafka integration
   - RabbitMQ integration

**Outcome**: Enterprise-grade distributed event processing platform

---

### Q3 2025 (6-9 Months)
**Focus**: Observability and documentation

7. **Monitoring & Observability (2 weeks)** 📋
   - Prometheus exporter
   - Grafana dashboards
   - APM integration

8. **API Documentation (1 week)** 📋
   - OpenAPI spec
   - Swagger UI

**Outcome**: Fully documented and observable production system

---

## Success Metrics

### Phase 1 (✅ Complete)
- [x] 100% feature parity with Clojure (EXCEEDED)
- [x] 200+ tests passing (ACHIEVED: 281 tests)
- [x] HTTP API endpoints (ACHIEVED: 11 endpoints)
- [x] Production Docker deployment (ACHIEVED)
- [x] Health and metrics (ACHIEVED)

### Phase 2 (📋 Planned - REVISED)
- [ ] Real-time event streaming via Core's WebSocket
- [ ] <100ms latency for event delivery
- [ ] 1000+ concurrent connections (via Core)
- [ ] Persistent projection state via Core API (DashMap + Parquet/PostgreSQL)
- [ ] 99.9% uptime (with OTP supervision)
- [ ] Broadway processing >10K events/sec
- [ ] Sub-microsecond cache access (11.9 μs via Core's DashMap)

### Phase 3 (📋 Future)
- [ ] Multi-node distributed deployment
- [ ] Kafka/RabbitMQ integration
- [ ] Prometheus + Grafana monitoring
- [ ] OpenAPI documentation

---

## Risk Assessment

### Low Risk ✅
- **State Persistence**: Well-understood Ecto patterns
- **Phoenix Channels**: Built-in Phoenix feature
- **Broadway Producer**: Foundation already exists

### Medium Risk ⚠️
- **Distributed Mode**: Complexity in consensus and state
- **Message Queue Integration**: External system dependencies
- **Performance at Scale**: Need load testing

### Mitigation Strategies
1. **Incremental rollout**: Each phase independently deployable
2. **Comprehensive testing**: Maintain >90% test coverage
3. **Feature flags**: Enable/disable features per environment
4. **Monitoring**: Add metrics before scaling features

---

## Technical Debt & Maintenance

### Current Debt
- None significant (recent migration, clean architecture)

### Preventive Measures
- Maintain test coverage >85%
- Regular dependency updates
- Code review for all changes
- Documentation updates with each feature

---

## Team & Resources

### Skills Needed for Phase 2
- **Elixir/OTP**: GenServer, supervision, Phoenix Channels
- **PostgreSQL**: Schema design, Ecto
- **Redis**: Caching patterns
- **WebSocket**: Real-time protocols
- **Broadway**: Stream processing

### Estimated Effort
- **Phase 2.1-2.3**: ~8-10 weeks (1 developer)
- **Phase 2.4-2.5**: ~6-8 weeks (1 developer)
- **Phase 3**: ~4-6 weeks (1 developer)

**Total for Complete Roadmap**: ~18-24 weeks (4.5-6 months)

---

## Conclusion

The Query Service has successfully completed **Phase 1** with a production-ready foundation. The path forward is clear:

1. ✅ **Core Features**: Complete (281 tests, Phoenix API, OTP supervision)
2. 📋 **Real-time & Persistence**: Next priority (Q1 2025)
3. 📋 **Distributed & Enterprise**: Following phases (Q2-Q3 2025)

**Recommendation**: Proceed with Phase 2.1 (State Persistence) as the first priority, followed by Phase 2.2 (Phoenix Channels) to unlock real-time capabilities.

---

**Document Version**: 1.0
**Status**: Active Roadmap
**Next Review**: After Phase 2.1 completion
