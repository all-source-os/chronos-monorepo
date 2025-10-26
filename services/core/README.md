# AllSource Core - High-Performance Event Store

> AI-native event store built in Rust with columnar storage, schema validation, event replay, and stream processing

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-242%20passing-brightgreen.svg)]()
[![Performance](https://img.shields.io/badge/throughput-469K%20events%2Fsec-blue.svg)]()
[![Architecture](https://img.shields.io/badge/clean%20architecture-Phase%203%20Started-success.svg)]()
[![License](https://img.shields.io/badge/license-MIT-blue.svg)]()

## 🎯 What is AllSource?

AllSource is a high-performance event store designed for modern event-sourcing and CQRS architectures. Built with a **polyglot architecture**:

- **Rust Core** (this service): High-performance event store engine with columnar storage
- **Go Control Plane** (`services/control-plane`): Orchestration, monitoring, and management layer

The Rust core provides blazing-fast event ingestion (469K events/sec) and sub-microsecond queries, while the Go control plane handles cluster coordination and operational tasks.

**Current Version**: v0.6.0 (Rust Core - Clean Architecture Phase 2) | v0.1.0 (Go Control Plane)

## ✨ Features

### 🚀 Core Event Store (v0.1)

- **Immutable Event Log**: Append-only storage with complete audit trail
- **Time-Travel Queries**: Query entity state as of any timestamp
- **Concurrent Indexing**: Lock-free indexing using `DashMap` for O(1) lookups
- **Real-time Projections**: Built-in materialized views with custom projection support
- **High Performance**: 469K+ events/sec throughput, sub-millisecond queries
- **Type-Safe API**: Strong typing with Rust's ownership system

### 💾 Persistence & Durability (v0.2)

- **Parquet Columnar Storage**: Apache Arrow-based storage for analytics
- **Write-Ahead Log (WAL)**: Crash recovery with full durability guarantees
- **Snapshot System**: Point-in-time snapshots with automatic optimization
- **Automatic Compaction**: Background file merging for storage efficiency
- **WebSocket Streaming**: Real-time event broadcasting to connected clients
- **Advanced Analytics**: Event frequency, correlation analysis, statistical summaries

### 📋 Schema Registry (v0.5)

- **JSON Schema Validation**: Enforce event contracts at ingestion time
- **Schema Versioning**: Automatic version management with compatibility checking
- **Compatibility Modes**: Backward, Forward, Full, or None
- **Breaking Change Prevention**: Validate schema evolution before deployment
- **Subject Organization**: Group schemas by domain or event type

### 🔄 Event Replay & Projections (v0.5)

- **Point-in-Time Replay**: Replay events from any timestamp
- **Projection Rebuilding**: Reconstruct materialized views with progress tracking
- **Batch Processing**: Configurable batch sizes for optimal performance
- **Async Execution**: Non-blocking background replay operations
- **Cancellable Operations**: Stop replays gracefully with proper cleanup
- **Progress Metrics**: Real-time statistics (events/sec, percentage complete)

### ⚡ Stream Processing (v0.5)

- **6 Built-in Operators**:
  - **Filter**: eq, ne, gt, lt, contains operations
  - **Map**: Transform field values (uppercase, lowercase, trim, math)
  - **Reduce**: Aggregations (count, sum, avg, min, max) with grouping
  - **Window**: Time-based aggregations (tumbling, sliding, session)
  - **Enrich**: External data lookup and enrichment
  - **Branch**: Conditional event routing
- **Stateful Processing**: Thread-safe state management for aggregations
- **Window Buffers**: Automatic time-based event eviction
- **Pipeline Statistics**: Track processing metrics per pipeline
- **Integrated Processing**: Events flow through pipelines during ingestion

### 🏔️ SierraDB-Inspired Production Patterns (NEW)

Based on battle-tested patterns from [SierraDB](https://github.com/cablehead/xs), a production-grade event store:

**PartitionKey** - Fixed Partition Architecture
- ✅ 32 fixed partitions for single-node deployment (scalable to 1024+)
- ✅ Consistent hashing ensures same entity always maps to same partition
- ✅ Sequential writes within partitions for ordering guarantees
- ✅ Ready for horizontal scaling with node assignment
- ✅ 6 comprehensive tests covering distribution and consistency

**EventStream** - Gapless Version Guarantees
- ✅ Watermark system tracks "highest continuously confirmed sequence"
- ✅ Prevents gaps that would break event sourcing guarantees
- ✅ Optimistic locking prevents concurrent modification conflicts
- ✅ Version numbers start at 1 and increment sequentially
- ✅ 9 tests covering versioning, concurrency, and gap detection

**EventStreamRepository** - Infrastructure Implementation
- ✅ Thread-safe in-memory implementation with parking_lot RwLock
- ✅ Partition-aware stream storage and retrieval
- ✅ Watermark tracking and gapless verification
- ✅ Optimistic locking enforcement at repository level
- ✅ 8 comprehensive tests covering all operations

**Why These Patterns?**
| Pattern | SierraDB's Lesson | Our Benefit |
|---------|-------------------|-------------|
| Fixed Partitions | Sequential writes enable gapless sequences | Horizontal scaling without complex coordination |
| Gapless Versions | Watermark prevents data gaps | Consistent event sourcing guarantees |
| Optimistic Locking | Detect concurrent modifications | Safe concurrent access without heavy locks |

**Coming Next**:
- 📦 Storage integrity checksums (prevent silent corruption)
- 🧪 7-day continuous stress tests (production confidence)
- ⚡ Zero-copy deserialization (performance optimization)

## 📊 Performance Benchmarks

Measured on Apple Silicon M-series (release build):

| Operation | Throughput/Latency | Details |
|-----------|-------------------|---------|
| Event Ingestion | 442-469K events/sec | Single-threaded |
| Entity Query | 11.9 μs | Indexed lookup |
| Type Query | 2.4 ms | Cross-entity scan |
| State Reconstruction | 3.5 μs | With snapshots |
| State Reconstruction | 3.8 μs | Without snapshots |
| Concurrent Writes (8 workers) | 8.0 ms/batch | 100 events/batch |
| Parquet Batch Write | 3.5 ms | 1000 events |
| Snapshot Creation | 130 μs | Per entity |
| WAL Sync Writes | 413 ms | 100 syncs |

**Test Coverage**: 242 tests - 100% passing
- Domain Layer: 177 tests (Value Objects, Entities, Business Rules, **SierraDB Patterns**)
  - **PartitionKey**: 6 tests (consistent hashing, distribution, node assignment)
  - **EventStream**: 9 tests (gapless versioning, optimistic locking, watermarks)
- Application Layer: 20 tests (Use Cases, DTOs)
- Infrastructure Layer: 45 tests (API, Storage, Services, **Repository Implementations**)
  - **InMemoryEventStreamRepository**: 8 tests (SierraDB pattern implementation)

## 🔧 API Endpoints (38 Total)

### Core Event Store

```bash
# Health check
GET /health

# Ingest event
POST /api/v1/events

# Query events
GET /api/v1/events/query?entity_id=user-123
GET /api/v1/events/query?event_type=user.created
GET /api/v1/events/query?since=2024-01-15T00:00:00Z&limit=100

# Entity state
GET /api/v1/entities/:entity_id/state
GET /api/v1/entities/:entity_id/state?as_of=2024-01-15T10:00:00Z
GET /api/v1/entities/:entity_id/snapshot

# Statistics
GET /api/v1/stats
```

### WebSocket Streaming (v0.2)

```bash
# Real-time event stream
WS /api/v1/events/stream
```

### Analytics (v0.2)

```bash
# Event frequency analysis
GET /api/v1/analytics/frequency?event_type=user.created&bucket_size=3600

# Statistical summary
GET /api/v1/analytics/summary?entity_id=user-123

# Event correlation
GET /api/v1/analytics/correlation?event_a=user.created&event_b=order.placed
```

### Snapshots (v0.2)

```bash
# Create snapshot
POST /api/v1/snapshots

# List snapshots
GET /api/v1/snapshots
GET /api/v1/snapshots?entity_id=user-123

# Get latest snapshot
GET /api/v1/snapshots/:entity_id/latest
```

### Compaction (v0.2)

```bash
# Trigger manual compaction
POST /api/v1/compaction/trigger

# Get compaction stats
GET /api/v1/compaction/stats
```

### Schema Registry (v0.5)

```bash
# Register schema
POST /api/v1/schemas

# List subjects
GET /api/v1/schemas

# Get schema
GET /api/v1/schemas/:subject
GET /api/v1/schemas/:subject?version=2

# List versions
GET /api/v1/schemas/:subject/versions

# Validate event
POST /api/v1/schemas/validate

# Set compatibility mode
PUT /api/v1/schemas/:subject/compatibility
```

### Event Replay (v0.5)

```bash
# Start replay
POST /api/v1/replay

# List replays
GET /api/v1/replay

# Get progress
GET /api/v1/replay/:replay_id

# Cancel replay
POST /api/v1/replay/:replay_id/cancel

# Delete replay
DELETE /api/v1/replay/:replay_id
```

### Stream Processing Pipelines (v0.5)

```bash
# Register pipeline
POST /api/v1/pipelines

# List pipelines
GET /api/v1/pipelines

# Get pipeline
GET /api/v1/pipelines/:pipeline_id

# Remove pipeline
DELETE /api/v1/pipelines/:pipeline_id

# Get all stats
GET /api/v1/pipelines/stats

# Get pipeline stats
GET /api/v1/pipelines/:pipeline_id/stats

# Reset pipeline state
PUT /api/v1/pipelines/:pipeline_id/reset
```

## 📁 Project Structure (Clean Architecture)

Following **Clean Architecture** principles with clear separation of concerns:

```
services/core/src/
├── main.rs                    # Application entry point
├── lib.rs                     # Library exports
├── error.rs                   # Error types and Result
│
├── domain/                    # 🏛️ DOMAIN LAYER (Business Logic)
│   ├── entities/              # Core business entities
│   │   ├── event.rs          # Event entity (162 tests)
│   │   ├── tenant.rs         # Multi-tenancy entity
│   │   ├── schema.rs         # Schema registry entity
│   │   ├── projection.rs     # Projection entity
│   │   └── event_stream.rs   # 🆕 Gapless event stream (9 tests)
│   └── value_objects/         # Self-validating value objects
│       ├── tenant_id.rs      # Tenant identifier
│       ├── event_type.rs     # Event type validation
│       ├── entity_id.rs      # Entity identifier
│       └── partition_key.rs  # 🆕 Fixed partitioning (6 tests)
│
├── application/               # 🎯 APPLICATION LAYER (Use Cases)
│   ├── dto/                   # Data Transfer Objects
│   │   ├── event_dto.rs      # Event request/response DTOs
│   │   ├── tenant_dto.rs     # Tenant DTOs
│   │   ├── schema_dto.rs     # Schema DTOs
│   │   └── projection_dto.rs # Projection DTOs
│   └── use_cases/             # Application business logic
│       ├── ingest_event.rs   # Event ingestion (3 tests)
│       ├── query_events.rs   # Event queries (4 tests)
│       ├── manage_tenant.rs  # Tenant management (5 tests)
│       ├── manage_schema.rs  # Schema operations (4 tests)
│       └── manage_projection.rs # Projection ops (4 tests)
│
└── infrastructure/            # 🔧 INFRASTRUCTURE LAYER (Technical)
    ├── repositories/         # 🆕 Repository implementations (SierraDB)
    │   └── in_memory_event_stream_repository.rs  # Thread-safe, partitioned (8 tests)
    ├── api.rs                # REST API endpoints (38 endpoints)
    ├── store.rs              # Event store implementation
    ├── storage.rs            # Parquet columnar storage
    ├── wal.rs                # Write-ahead log
    ├── snapshot.rs           # Snapshot management
    ├── compaction.rs         # Storage compaction
    ├── index.rs              # High-performance indexing
    ├── projection.rs         # Real-time projections
    ├── analytics.rs          # Analytics engine
    ├── websocket.rs          # WebSocket streaming
    ├── schema.rs             # Schema validation service
    ├── replay.rs             # Event replay engine
    ├── pipeline.rs           # Stream processing
    ├── backup.rs             # Backup management
    ├── auth.rs               # Authentication/Authorization
    ├── rate_limit.rs         # Rate limiting
    ├── tenant.rs             # Tenant service
    ├── metrics.rs            # Metrics collection
    ├── middleware.rs         # HTTP middleware
    ├── config.rs             # Configuration
    ├── tenant_api.rs         # Tenant API handlers
    └── auth_api.rs           # Auth API handlers

tests/
└── integration_tests.rs      # End-to-end tests

benches/
└── performance_benchmarks.rs # Performance benchmarks
```

### 🏗️ Clean Architecture Benefits

**Domain Layer** (Core Business Logic)
- ✅ Pure business rules with zero external dependencies
- ✅ Value Objects enforce invariants at construction time
- ✅ Entities contain rich domain behavior
- ✅ **SierraDB patterns** for production-grade event streaming
- ✅ 177 comprehensive domain tests (including 15 SierraDB tests)

**Application Layer** (Orchestration)
- ✅ Use Cases coordinate domain entities
- ✅ DTOs isolate external contracts from domain
- ✅ Clear input/output boundaries
- ✅ 20 use case tests covering all scenarios

**Infrastructure Layer** (Technical Details)
- ✅ Pluggable implementations (can swap storage, APIs)
- ✅ Framework and library dependencies isolated
- ✅ Domain and Application layers remain pure
- ✅ 37 infrastructure integration tests

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone <repo-url>
cd chronos-monorepo/services/core

# Build
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Running the Server

```bash
# Development mode
cargo run

# Production mode (optimized)
cargo run --release

# With debug logging
RUST_LOG=debug cargo run

# Custom port (modify main.rs)
# Default: 0.0.0.0:8080
```

### Example: Ingest Events

```bash
curl -X POST http://localhost:8080/api/v1/events \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "user.created",
    "entity_id": "user-123",
    "payload": {
      "name": "Alice",
      "email": "alice@example.com"
    }
  }'
```

### Example: Query Events

```bash
# Get all events for an entity
curl "http://localhost:8080/api/v1/events/query?entity_id=user-123"

# Time-travel query
curl "http://localhost:8080/api/v1/events/query?entity_id=user-123&as_of=2024-01-15T10:00:00Z"

# Query by type
curl "http://localhost:8080/api/v1/events/query?event_type=user.created&limit=10"
```

### Example: Register Schema (v0.5)

```bash
curl -X POST http://localhost:8080/api/v1/schemas \
  -H "Content-Type: application/json" \
  -d '{
    "subject": "user.created",
    "schema": {
      "type": "object",
      "required": ["name", "email"],
      "properties": {
        "name": {"type": "string"},
        "email": {"type": "string", "format": "email"}
      }
    }
  }'
```

### Example: Start Replay (v0.5)

```bash
curl -X POST http://localhost:8080/api/v1/replay \
  -H "Content-Type: application/json" \
  -d '{
    "projection_name": "user_snapshot",
    "from_timestamp": "2024-01-01T00:00:00Z",
    "config": {
      "batch_size": 1000,
      "emit_progress": true
    }
  }'
```

### Example: Create Pipeline (v0.5)

```bash
curl -X POST http://localhost:8080/api/v1/pipelines \
  -H "Content-Type: application/json" \
  -d '{
    "name": "user_analytics",
    "source_event_types": ["user.created", "user.updated"],
    "operators": [
      {
        "type": "filter",
        "field": "country",
        "value": "US",
        "op": "eq"
      },
      {
        "type": "reduce",
        "field": "id",
        "function": "count",
        "group_by": "country"
      }
    ],
    "enabled": true
  }'
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests
cargo test --test integration_tests

# Run specific test
cargo test test_replay_progress_tracking

# Run with output
cargo test -- --nocapture

# Run with logging
RUST_LOG=debug cargo test
```

## 📊 Benchmarking

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark suite
cargo bench ingestion_throughput
cargo bench query_performance
cargo bench state_reconstruction
cargo bench concurrent_writes

# View results
open target/criterion/report/index.html
```

## 🏗️ Architecture

### System Architecture

AllSource uses a **polyglot architecture** with specialized services:

```
┌─────────────────────────────────────────┐
│   Go Control Plane (Port 8081)         │
│   • Cluster Management                  │
│   • Metrics Aggregation                 │
│   • Operation Orchestration             │
│   • Health Monitoring                   │
└─────────────────────────────────────────┘
              │ HTTP Client
              ▼
┌─────────────────────────────────────────┐
│   Rust Event Store (Port 8080)         │
│   • Event Ingestion (469K/sec)          │
│   • Query Engine (<12μs)                │
│   • Schema Registry                     │
│   • Stream Processing                   │
│   • Event Replay                        │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│   Storage Layer                         │
│   • Parquet (Columnar)                  │
│   • WAL (Durability)                    │
│   • Snapshots (Point-in-time)           │
│   • In-Memory Indexes                   │
└─────────────────────────────────────────┘
```

### Event Flow

When an event is ingested:

1. **Validation** - Check event integrity
2. **WAL Write** - Durable write-ahead log (v0.2)
3. **Indexing** - Update entity/type indexes
4. **Projections** - Update materialized views
5. **Pipelines** - Real-time stream processing (v0.5)
6. **Parquet Storage** - Columnar persistence (v0.2)
7. **In-Memory Store** - Fast access layer
8. **WebSocket Broadcast** - Real-time streaming (v0.2)
9. **Auto-Snapshots** - Create snapshots if needed (v0.2)

### Storage Architecture

```
Storage Layer:
├── In-Memory Events (Vec<Event>)
├── DashMap Indexes (entity_id, event_type)
├── Parquet Files (columnar storage)
├── WAL Segments (append-only logs)
└── Snapshots (point-in-time state)
```

### Concurrency Model

- **Lock-free Indexes**: DashMap for entity/type lookups
- **RwLock**: parking_lot RwLock for event list
- **Atomic Counters**: Lock-free statistics tracking
- **Async Runtime**: Tokio for background tasks (replay, compaction)

## 🎯 Roadmap

### ✅ v0.1 - Core Event Store (COMPLETED)
- [x] In-memory event storage
- [x] DashMap-based indexing
- [x] Entity state reconstruction
- [x] Basic projections
- [x] REST API
- [x] Query by entity/type/time

### ✅ v0.2 - Persistence & Durability (COMPLETED)
- [x] Parquet columnar storage
- [x] Write-ahead log (WAL)
- [x] Snapshot system
- [x] Automatic compaction
- [x] WebSocket streaming
- [x] Advanced analytics

### ✅ v0.5 - Schema & Processing (COMPLETED)
- [x] Schema registry with versioning
- [x] Event replay engine
- [x] Projection rebuilding
- [x] Stream processing pipelines
- [x] Stateful aggregations
- [x] Window operations

### ✅ v0.6 - Clean Architecture Refactoring (PHASE 2 COMPLETED)
- [x] **Phase 1**: Domain Layer (162 tests)
  - [x] Value Objects (TenantId, EventType, EntityId)
  - [x] Domain Entities (Event, Tenant, Schema, Projection)
  - [x] Business rule enforcement
  - [x] Self-validating types
- [x] **Phase 2**: Application Layer (20 tests)
  - [x] DTOs for all operations
  - [x] Tenant management use cases
  - [x] Schema management use cases
  - [x] Projection management use cases
  - [x] Clean separation from domain
- [ ] **Phase 3**: Infrastructure Layer (IN PROGRESS)
  - [ ] Repository pattern implementation
  - [ ] API layer refactoring
  - [ ] Service layer extraction
  - [ ] Dependency injection

### 📋 v0.7 - Performance & Optimization (PLANNED)
- [ ] Zero-copy deserialization optimization
- [ ] SIMD-accelerated queries
- [ ] Memory-mapped Parquet files
- [ ] Adaptive indexing strategies
- [ ] Query result caching
- [ ] Compression tuning

### 📋 v0.8 - Advanced Features (PLANNED)
- [x] Multi-tenancy support (Domain layer complete)
- [ ] Event encryption at rest
- [ ] Audit logging
- [ ] Retention policies
- [ ] Data archival
- [ ] Backup/restore (Partially implemented)

### 🌐 v1.0 - Distributed & Cloud-Native (PLANNED)
- [ ] Distributed replication
- [ ] Multi-region support
- [ ] Consensus protocol (Raft)
- [ ] Arrow Flight RPC
- [ ] Kubernetes operators
- [ ] Cloud-native deployment
- [ ] Horizontal scaling
- [ ] Load balancing

### 🔮 Future Enhancements (BACKLOG)
- [ ] GraphQL API
- [ ] WASM plugin system
- [ ] Change Data Capture (CDC)
- [ ] Time-series optimization
- [ ] Machine learning integrations
- [ ] Real-time anomaly detection
- [ ] Event sourcing templates
- [ ] Visual query builder

## 🔬 Technical Decisions

### Why Rust?

- **Performance**: Zero-cost abstractions, no GC pauses
- **Safety**: Ownership model prevents data races
- **Concurrency**: Fearless concurrency with Send/Sync
- **Ecosystem**: Excellent libraries (Tokio, Axum, Arrow)

### Why DashMap?

- Lock-free concurrent HashMap
- Better than `RwLock<HashMap>` for reads
- Sharded internally for minimal contention
- O(1) lookups with concurrent access

### Why Apache Arrow/Parquet?

- Industry-standard columnar format
- Zero-copy data access
- SIMD-accelerated operations
- Excellent compression ratios
- Interoperable with DataFusion, Polars, DuckDB

### Why Tokio + Axum?

- High-performance async runtime
- Type-safe request handlers
- Excellent ecosystem integration
- Low overhead, fast routing

### Why parking_lot?

- Smaller and faster than std::sync::RwLock
- No poisoning - simpler error handling
- Better performance under contention
- Widely used in production Rust

## 📚 Usage Examples

### Programmatic API

```rust
use allsource_core::{EventStore, Event, QueryEventsRequest};
use serde_json::json;

// Create store
let store = EventStore::new();

// Ingest events
let event = Event::new(
    "user.created".to_string(),
    "user-123".to_string(),
    json!({
        "name": "Alice",
        "email": "alice@example.com"
    })
);
store.ingest(event)?;

// Query events
let request = QueryEventsRequest {
    entity_id: Some("user-123".to_string()),
    ..Default::default()
};
let events = store.query(request)?;

// Reconstruct state
let state = store.reconstruct_state("user-123", None)?;
println!("Current state: {}", state);

// Time-travel query
let timestamp = chrono::Utc::now() - chrono::Duration::hours(1);
let past_state = store.reconstruct_state("user-123", Some(timestamp))?;
println!("State 1 hour ago: {}", past_state);
```

### Custom Projection

```rust
use allsource_core::projection::Projection;
use allsource_core::event::Event;
use serde_json::Value;
use parking_lot::RwLock;
use std::collections::HashMap;

struct RevenueProjection {
    totals: RwLock<HashMap<String, f64>>,
}

impl Projection for RevenueProjection {
    fn name(&self) -> &str {
        "revenue_by_customer"
    }

    fn process(&self, event: &Event) -> allsource_core::Result<()> {
        if event.event_type == "order.completed" {
            if let Some(amount) = event.payload["amount"].as_f64() {
                let mut totals = self.totals.write();
                *totals.entry(event.entity_id.clone()).or_insert(0.0) += amount;
            }
        }
        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.totals.read()
            .get(entity_id)
            .map(|total| json!({ "total_revenue": total }))
    }

    fn clear(&self) {
        self.totals.write().clear();
    }
}
```

## 🐛 Troubleshooting

### Port Already in Use

```bash
# Find process using port 8080
lsof -i :8080

# Kill process
kill -9 <PID>
```

### Slow Performance

```bash
# Always use release mode for benchmarks
cargo run --release
cargo bench

# Check system resources
top -o cpu
```

### Memory Issues

```bash
# Monitor memory usage
RUST_LOG=allsource_core=debug cargo run --release

# Reduce batch sizes in config
# Adjust snapshot_config.max_events_before_snapshot
```

### Test Failures

```bash
# Clean build
cargo clean
cargo test

# Check for stale processes
killall allsource-core

# Verbose test output
cargo test -- --nocapture --test-threads=1
```

## 📖 Resources

- [Event Sourcing Pattern](https://martinfowler.com/eaaDev/EventSourcing.html)
- [CQRS Pattern](https://martinfowler.com/bliki/CQRS.html)
- [Apache Arrow](https://arrow.apache.org/)
- [Parquet Format](https://parquet.apache.org/)
- [Tokio Async Runtime](https://tokio.rs/)
- [Axum Web Framework](https://docs.rs/axum)

## 🤝 Contributing

Contributions are welcome! Areas of interest:

- Performance optimizations
- Additional projection types
- Query optimization
- Documentation improvements
- Bug fixes and tests

## 📄 License

MIT License - see LICENSE file for details

---

<div align="center">

**AllSource Core** - *Event sourcing, done right*

Built with 🦀 Rust | Clean Architecture | Made for Production

Version 0.6.0 | 469K events/sec | 219 tests passing | Phase 2 Complete

</div>
