# AllSource Core - Rust Event Store

> High-performance event store engine built in Rust with columnar storage architecture

## 🚀 Features

### ⚡ High-Performance Architecture

- **Concurrent Indexing**: Lock-free indexing using `DashMap` for entity and event type lookups
- **Zero-Copy Operations**: Leveraging Apache Arrow for efficient data manipulation
- **SIMD-Ready**: Prepared for vectorized operations with Arrow/Parquet integration
- **Optimistic Locking**: `parking_lot` RwLock for minimal contention

### 📊 Event Sourcing Capabilities

- **Immutable Event Log**: Append-only storage with complete audit trail
- **Time-Travel Queries**: Query entity state as of any timestamp
- **Event Replay**: Reconstruct state by replaying events
- **Projections**: Real-time aggregations and materialized views

### 🔍 Indexing System

- **Entity Index**: O(1) lookup by entity_id
- **Event Type Index**: O(1) lookup by event_type
- **Event ID Index**: Direct event access by UUID
- **Concurrent Updates**: Thread-safe index modifications

### 📈 Projections

Built-in projections for real-time aggregations:

1. **Entity Snapshots**: Current state of each entity
2. **Event Counters**: Event type statistics

Custom projections can be implemented using the `Projection` trait.

### 🛡️ Error Handling

Comprehensive error types with automatic HTTP status code mapping:

- `EventNotFound` → 404
- `EntityNotFound` → 404
- `InvalidEvent` → 400
- `ValidationError` → 400
- `StorageError` → 500
- `InternalError` → 500

## 📁 Module Overview

```
src/
├── main.rs         # Application entry point
├── lib.rs          # Library exports for benchmarks
├── error.rs        # Error types and Result
├── event.rs        # Event data structures
├── index.rs        # High-performance indexing
├── projection.rs   # Real-time aggregations
├── store.rs        # Core event store implementation
└── api.rs          # REST API endpoints
```

## 🔧 API Endpoints

### Health Check
```
GET /health
```

### Ingest Event
```
POST /api/v1/events
Content-Type: application/json

{
  "event_type": "user.created",
  "entity_id": "user-123",
  "payload": {
    "name": "Alice",
    "email": "alice@example.com"
  },
  "metadata": {}
}
```

### Query Events
```
GET /api/v1/events/query?entity_id=user-123
GET /api/v1/events/query?event_type=user.created
GET /api/v1/events/query?entity_id=user-123&as_of=2024-01-15T10:00:00Z
GET /api/v1/events/query?since=2024-01-15T00:00:00Z&limit=100
```

### Reconstruct Entity State
```
GET /api/v1/entities/:entity_id/state
GET /api/v1/entities/:entity_id/state?as_of=2024-01-15T10:00:00Z
```

### Get Entity Snapshot (Fast)
```
GET /api/v1/entities/:entity_id/snapshot
```

### Statistics
```
GET /api/v1/stats
```

Returns:
```json
{
  "total_events": 1234,
  "total_entities": 456,
  "total_event_types": 12,
  "total_ingested": 1234
}
```

## 🏃 Running

### Development
```bash
cargo run
```

### Production (Release Build)
```bash
cargo run --release
```

### With Custom Log Level
```bash
RUST_LOG=debug cargo run
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_index_event
```

## 📊 Benchmarking

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench event_ingestion

# View benchmark results
open target/criterion/report/index.html
```

### Benchmark Suites

1. **Single Event Ingestion**: Measures single-threaded write performance
2. **Batch Ingestion**: Tests throughput for 100, 1K, and 10K events
3. **Concurrent Ingestion**: Multi-threaded write performance (100 threads)
4. **Query Performance**: Entity and type-based query benchmarks
5. **State Reconstruction**: Event replay vs. snapshot retrieval

## 🎯 Performance Targets

| Operation | Target (v0.1) | Target (v1.0) |
|-----------|---------------|---------------|
| Event Ingestion | 100K/sec | 1M+/sec |
| Entity Query (indexed) | <1ms | <100μs |
| State Reconstruction | <10ms | <1ms |
| Concurrent Writes | 50K/sec | 500K/sec |

## 🔬 Architecture Decisions

### Why DashMap?

- Lock-free concurrent HashMap
- Better performance than `RwLock<HashMap>` for multi-threaded access
- Sharded internally for minimal contention

### Why parking_lot?

- Smaller and faster than std::sync::RwLock
- No poisoning - simpler error handling
- Better performance under contention

### Why Apache Arrow?

- Industry-standard columnar format
- Zero-copy data access
- SIMD-accelerated operations
- Interoperability with DataFusion, Polars, etc.

### Why Axum?

- Built on Tokio - excellent async performance
- Type-safe extractors
- Composable middleware
- Low overhead

## 🚀 Next Steps (v0.2)

- [ ] Persistent Parquet file storage
- [ ] Write-ahead log (WAL) for durability
- [ ] Snapshot creation and loading
- [ ] Compaction strategy
- [ ] Multi-version concurrency control (MVCC)
- [ ] Distributed replication
- [ ] Arrow Flight RPC integration

## 📝 Code Examples

### Custom Projection

```rust
use allsource_core::projection::Projection;
use allsource_core::event::Event;

struct MyCustomProjection {
    // Your state here
}

impl Projection for MyCustomProjection {
    fn name(&self) -> &str {
        "my_projection"
    }

    fn process(&self, event: &Event) -> Result<()> {
        // Process event and update state
        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        // Return current state for entity
        None
    }

    fn clear(&self) {
        // Clear projection state
    }
}
```

### Using the Event Store Programmatically

```rust
use allsource_core::{EventStore, Event};
use serde_json::json;

let store = EventStore::new();

// Ingest an event
let event = Event::new(
    "user.created".to_string(),
    "user-123".to_string(),
    json!({ "name": "Alice" })
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
```

## 🐛 Debugging

### Enable Detailed Logging

```bash
RUST_LOG=allsource_core=trace cargo run
```

### Common Issues

**Issue**: Events not appearing in queries
**Solution**: Check that entity_id and event_type match exactly (case-sensitive)

**Issue**: Slow performance
**Solution**: Ensure you're running in release mode (`cargo run --release`)

**Issue**: Port 8080 already in use
**Solution**: Kill the existing process or change the port in `main.rs`

## 📚 Learn More

- [Event Sourcing Pattern](https://martinfowler.com/eaaDev/EventSourcing.html)
- [Apache Arrow Format](https://arrow.apache.org/)
- [DashMap Documentation](https://docs.rs/dashmap)
- [Axum Web Framework](https://docs.rs/axum)

---

<div align="center">

**AllSource Core** - *Built for speed, designed for scale*

Made with 🦀 Rust

</div>
