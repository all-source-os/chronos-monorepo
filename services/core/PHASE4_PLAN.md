# Phase 4: Performance & Persistence - Production Scale

**Target**: v0.8.0
**Status**: 🚧 In Progress
**Timeline**: 8-10 weeks
**Goal**: Production-scale performance and persistent storage

---

## 🎯 Overview

Phase 4 builds on the SierraDB patterns from Phase 3 to add:
1. **Performance optimizations** (lock-free structures, zero-copy operations)
2. **Persistent storage** (PostgreSQL, RocksDB repositories)
3. **Distributed systems foundation** (clustering, node coordination)
4. **Production monitoring** (metrics, health checks, observability)

## 📊 Current State

**✅ Completed (Phase 3)**:
- Fixed 32-partition architecture
- Gapless versioning with watermark system
- Optimistic locking for concurrency
- Storage integrity checksums
- 7-day stress test infrastructure
- Performance utilities (BatchWriter, MemoryPool)
- In-memory EventStreamRepository
- Total: 257+ tests passing

**🎯 Phase 4 Goals**:
- Add persistent storage backends
- Optimize for 1M+ events/sec throughput
- Support multi-node clustering
- Production-ready observability
- Zero-copy operations where possible
- Lock-free critical paths

---

## 🗓️ Implementation Timeline

### Phase 4A: Lock-Free Optimizations (Weeks 1-2)

**Goal**: Remove contention in hot paths using lock-free data structures

#### Step 1: Lock-Free Event Ingestion Queue

**Create**: `src/infrastructure/persistence/lock_free_queue.rs`

**Features**:
- Lock-free MPMC queue for event ingestion
- Based on crossbeam-queue bounded/unbounded queues
- Zero-copy where possible
- Backpressure handling

**Benefits**:
- Removes RwLock contention in hot path
- Better multi-threaded throughput
- Predictable latency (no lock waiting)

**Implementation**:
```rust
use crossbeam::queue::ArrayQueue;
use crate::domain::entities::Event;
use crate::error::Result;

/// Lock-free bounded event queue
///
/// Uses lock-free MPMC queue for high-throughput event ingestion.
/// Multiple producers (API handlers) can push events concurrently
/// without blocking. Single or multiple consumers can drain the queue.
pub struct LockFreeEventQueue {
    queue: ArrayQueue<Event>,
    capacity: usize,
}

impl LockFreeEventQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: ArrayQueue::new(capacity),
            capacity,
        }
    }

    /// Try to push an event (non-blocking)
    pub fn try_push(&self, event: Event) -> Result<()> {
        self.queue.push(event).map_err(|_| {
            AllSourceError::QueueFull("Event queue at capacity".to_string())
        })
    }

    /// Pop event from queue (non-blocking)
    pub fn try_pop(&self) -> Option<Event> {
        self.queue.pop()
    }

    /// Current queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        self.queue.len() == self.capacity
    }
}
```

**Tests**: 8 tests
- Push/pop operations
- Concurrent producers/consumers
- Queue full handling
- Backpressure scenarios

#### Step 2: Lock-Free Metrics Collection

**Create**: `src/infrastructure/metrics/lock_free_metrics.rs`

**Features**:
- Atomic counters for metrics
- No lock contention on metric updates
- Memory-efficient aggregation

**Implementation**:
```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Lock-free metrics collector
pub struct LockFreeMetrics {
    events_ingested: AtomicU64,
    events_queried: AtomicU64,
    total_latency_ns: AtomicU64,
    started_at: Instant,
}

impl LockFreeMetrics {
    pub fn new() -> Self {
        Self {
            events_ingested: AtomicU64::new(0),
            events_queried: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            started_at: Instant::now(),
        }
    }

    pub fn record_ingest(&self) {
        self.events_ingested.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_query(&self, latency: Duration) {
        self.events_queried.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn throughput_per_sec(&self) -> f64 {
        let elapsed = self.started_at.elapsed().as_secs_f64();
        if elapsed == 0.0 { return 0.0; }
        self.events_ingested.load(Ordering::Relaxed) as f64 / elapsed
    }

    pub fn avg_query_latency(&self) -> Duration {
        let total = self.total_latency_ns.load(Ordering::Relaxed);
        let count = self.events_queried.load(Ordering::Relaxed);
        if count == 0 { return Duration::ZERO; }
        Duration::from_nanos(total / count)
    }
}
```

**Tests**: 6 tests
- Concurrent metric updates
- Throughput calculations
- Latency tracking
- Edge cases (zero counts)

**Estimated Work**: ~400 lines of code, 14 tests

---

### Phase 4B: Persistent Storage - PostgreSQL (Weeks 3-5)

**Goal**: Production-grade persistent EventStreamRepository

#### Step 1: PostgreSQL Schema Design

**Create**: `migrations/001_event_streams.sql`

```sql
-- Event Streams table
CREATE TABLE event_streams (
    stream_id VARCHAR(255) PRIMARY KEY,
    partition_id INTEGER NOT NULL,
    current_version BIGINT NOT NULL DEFAULT 0,
    watermark BIGINT NOT NULL DEFAULT 0,
    expected_version BIGINT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Events table
CREATE TABLE events (
    id BIGSERIAL PRIMARY KEY,
    stream_id VARCHAR(255) NOT NULL REFERENCES event_streams(stream_id),
    version BIGINT NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    event_type VARCHAR(255) NOT NULL,
    entity_id VARCHAR(255) NOT NULL,
    payload JSONB NOT NULL,
    metadata JSONB,
    timestamp TIMESTAMP NOT NULL DEFAULT NOW(),

    UNIQUE(stream_id, version)
);

-- Indexes for common queries
CREATE INDEX idx_events_stream_version ON events(stream_id, version);
CREATE INDEX idx_events_stream_id ON events(stream_id);
CREATE INDEX idx_events_tenant ON events(tenant_id);
CREATE INDEX idx_events_entity ON events(entity_id);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_stream_partition ON event_streams(partition_id);

-- Partition statistics view
CREATE VIEW partition_stats AS
SELECT
    partition_id,
    COUNT(*) as stream_count,
    SUM(current_version) as total_events
FROM event_streams
GROUP BY partition_id
ORDER BY partition_id;
```

#### Step 2: PostgreSQL Repository Implementation

**Create**: `src/infrastructure/repositories/postgres_event_stream_repository.rs`

```rust
use sqlx::{PgPool, Row};
use async_trait::async_trait;
use crate::domain::entities::{Event, EventStream};
use crate::domain::value_objects::{EntityId, PartitionKey};
use crate::domain::repositories::EventStreamRepository;
use crate::error::Result;

pub struct PostgresEventStreamRepository {
    pool: PgPool,
}

impl PostgresEventStreamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventStreamRepository for PostgresEventStreamRepository {
    async fn get_or_create_stream(&self, stream_id: &EntityId) -> Result<EventStream> {
        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Try to get existing stream
        let maybe_stream = sqlx::query(
            "SELECT stream_id, partition_id, current_version, watermark,
                    expected_version, created_at, updated_at
             FROM event_streams WHERE stream_id = $1"
        )
        .bind(stream_id.as_str())
        .fetch_optional(&mut *tx)
        .await?;

        let stream = if let Some(row) = maybe_stream {
            // Load events
            let events = sqlx::query(
                "SELECT tenant_id, event_type, entity_id, payload, metadata, timestamp
                 FROM events WHERE stream_id = $1 ORDER BY version"
            )
            .bind(stream_id.as_str())
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|row| {
                Event::from_strings(
                    row.get("tenant_id"),
                    row.get("event_type"),
                    row.get("entity_id"),
                    row.get("payload"),
                )
            })
            .collect::<Result<Vec<_>>>()?;

            EventStream::reconstruct(
                stream_id.clone(),
                PartitionKey::from_partition_id(row.get("partition_id"), 32)?,
                row.get("current_version"),
                row.get("watermark"),
                events,
                row.get("expected_version"),
                row.get("created_at"),
                row.get("updated_at"),
            )?
        } else {
            // Create new stream
            let stream = EventStream::new(stream_id.clone());

            sqlx::query(
                "INSERT INTO event_streams
                 (stream_id, partition_id, current_version, watermark, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(stream_id.as_str())
            .bind(stream.partition_key().partition_id() as i32)
            .bind(stream.current_version() as i64)
            .bind(stream.watermark() as i64)
            .bind(stream.created_at())
            .bind(stream.updated_at())
            .execute(&mut *tx)
            .await?;

            stream
        };

        tx.commit().await?;
        Ok(stream)
    }

    async fn append_to_stream(&self, stream: &mut EventStream, event: Event) -> Result<u64> {
        let mut tx = self.pool.begin().await?;

        // Optimistic locking check
        let current_version: i64 = sqlx::query_scalar(
            "SELECT current_version FROM event_streams WHERE stream_id = $1 FOR UPDATE"
        )
        .bind(stream.stream_id().as_str())
        .fetch_one(&mut *tx)
        .await?;

        if let Some(expected) = stream.expected_version() {
            if expected != current_version as u64 {
                return Err(AllSourceError::ConcurrencyError(format!(
                    "Version conflict: expected {}, got {}",
                    expected, current_version
                )));
            }
        }

        // Append event to domain entity
        let new_version = stream.append_event(event.clone())?;

        // Insert event
        sqlx::query(
            "INSERT INTO events
             (stream_id, version, tenant_id, event_type, entity_id, payload, metadata, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(stream.stream_id().as_str())
        .bind(new_version as i64)
        .bind(event.tenant_id().as_str())
        .bind(event.event_type().as_str())
        .bind(event.entity_id().as_str())
        .bind(serde_json::to_value(event.payload())?)
        .bind(serde_json::to_value(event.metadata())?)
        .bind(event.timestamp())
        .execute(&mut *tx)
        .await?;

        // Update stream metadata
        sqlx::query(
            "UPDATE event_streams
             SET current_version = $1, watermark = $2, updated_at = $3
             WHERE stream_id = $4"
        )
        .bind(stream.current_version() as i64)
        .bind(stream.watermark() as i64)
        .bind(stream.updated_at())
        .bind(stream.stream_id().as_str())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(new_version)
    }

    async fn get_streams_by_partition(&self, partition_key: &PartitionKey) -> Result<Vec<EventStream>> {
        // Implementation...
        Ok(Vec::new())
    }

    async fn get_watermark(&self, stream_id: &EntityId) -> Result<u64> {
        let watermark: i64 = sqlx::query_scalar(
            "SELECT watermark FROM event_streams WHERE stream_id = $1"
        )
        .bind(stream_id.as_str())
        .fetch_one(&self.pool)
        .await?;

        Ok(watermark as u64)
    }

    async fn verify_gapless(&self, stream_id: &EntityId) -> Result<bool> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) as gaps
             FROM generate_series(1,
                (SELECT watermark FROM event_streams WHERE stream_id = $1)
             ) AS expected_version
             LEFT JOIN events ON events.stream_id = $1 AND events.version = expected_version
             WHERE events.version IS NULL"
        )
        .bind(stream_id.as_str())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(gaps,)| gaps == 0).unwrap_or(true))
    }

    async fn partition_stats(&self) -> Result<Vec<(u32, usize)>> {
        let stats = sqlx::query_as::<_, (i32, i64)>(
            "SELECT partition_id, COUNT(*) FROM event_streams GROUP BY partition_id ORDER BY partition_id"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(stats.into_iter().map(|(p, c)| (p as u32, c as usize)).collect())
    }
}
```

**Dependencies**:
```toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "json", "chrono", "uuid"] }
```

**Tests**: 15 tests
- Connection pooling
- Transaction management
- Optimistic locking
- Concurrent appends
- Watermark verification
- Partition queries
- Edge cases (empty streams, large streams)

**Estimated Work**: ~600 lines of code, 15 tests

---

### Phase 4C: RocksDB Implementation (Weeks 6-7)

**Goal**: High-performance embedded storage option

#### RocksDB Repository

**Create**: `src/infrastructure/repositories/rocksdb_event_stream_repository.rs`

**Features**:
- Embedded key-value store (no separate database)
- Ultra-low latency (<1μs reads)
- LSM-tree based (optimized for writes)
- Column families for different data types
- Snapshot support

**Key Design**:
- Column Family 1: Stream metadata (`stream_id` -> `EventStream`)
- Column Family 2: Events (`stream_id:version` -> `Event`)
- Column Family 3: Indexes (partition -> stream_ids)

**Benefits**:
- No external dependencies
- Embedded in process
- Excellent single-node performance
- Simple deployment

**Tests**: 12 tests

**Estimated Work**: ~500 lines of code, 12 tests

---

### Phase 4D: Distributed Partitioning (Weeks 8-10)

**Goal**: Multi-node clustering with partition distribution

#### Step 1: Node Registry

**Create**: `src/infrastructure/cluster/node_registry.rs`

**Features**:
- Node discovery (static config or service discovery)
- Health checking
- Partition assignment
- Automatic rebalancing

```rust
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;

pub struct Node {
    pub id: u32,
    pub address: String,
    pub healthy: bool,
    pub assigned_partitions: Vec<u32>,
}

pub struct NodeRegistry {
    nodes: Arc<RwLock<HashMap<u32, Node>>>,
    partition_count: u32,
}

impl NodeRegistry {
    pub fn new(partition_count: u32) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            partition_count,
        }
    }

    /// Register a node in the cluster
    pub fn register_node(&self, node: Node) {
        let mut nodes = self.nodes.write();
        nodes.insert(node.id, node);
        self.rebalance_partitions(&mut nodes);
    }

    /// Rebalance partitions across healthy nodes
    fn rebalance_partitions(&self, nodes: &mut HashMap<u32, Node>) {
        let healthy_nodes: Vec<u32> = nodes.values()
            .filter(|n| n.healthy)
            .map(|n| n.id)
            .collect();

        if healthy_nodes.is_empty() {
            return;
        }

        // Distribute partitions evenly
        for partition_id in 0..self.partition_count {
            let node_idx = (partition_id as usize) % healthy_nodes.len();
            let node_id = healthy_nodes[node_idx];

            if let Some(node) = nodes.get_mut(&node_id) {
                if !node.assigned_partitions.contains(&partition_id) {
                    node.assigned_partitions.push(partition_id);
                }
            }
        }
    }

    /// Find node responsible for partition
    pub fn node_for_partition(&self, partition_id: u32) -> Option<u32> {
        let nodes = self.nodes.read();
        nodes.values()
            .find(|n| n.assigned_partitions.contains(&partition_id))
            .map(|n| n.id)
    }
}
```

#### Step 2: Distributed Request Router

**Create**: `src/infrastructure/cluster/request_router.rs`

**Features**:
- Route requests to correct node based on partition
- Load balancing within partition
- Failover on node failure

**Tests**: 10 tests

**Estimated Work**: ~400 lines of code, 10 tests

---

## 📊 Success Metrics

### Performance Targets

| Metric | Current | Phase 4 Target |
|--------|---------|----------------|
| Ingestion Throughput | 469K events/sec | 1M+ events/sec |
| Query Latency (p99) | <1ms | <500μs |
| Concurrent Writers | 8 | 64+ |
| Storage Backend | In-memory | PostgreSQL + RocksDB |
| Multi-node Support | No | Yes (clustering) |

### Test Coverage

| Component | Target Tests | Status |
|-----------|--------------|--------|
| Lock-free queue | 8 tests | Pending |
| Lock-free metrics | 6 tests | Pending |
| PostgreSQL repo | 15 tests | Pending |
| RocksDB repo | 12 tests | Pending |
| Node registry | 10 tests | Pending |
| **Phase 4 Total** | **51 tests** | **0/51** |
| **Grand Total** | **308+ tests** | **257/308** |

---

## 🏗️ Architecture Evolution

### Before Phase 4 (Current)
```
┌─────────────────────────────────────┐
│   Application Layer (Use Cases)    │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│        Domain Layer (Entities)      │
│  - PartitionKey, EventStream        │
│  - Gapless versioning, Optimistic   │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│    Infrastructure (Repository)      │
│  - InMemoryEventStreamRepository    │
│  - Storage: In-Memory Only          │
└─────────────────────────────────────┘
```

### After Phase 4
```
┌─────────────────────────────────────────────┐
│     Application Layer (Use Cases)           │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│          Domain Layer (Entities)            │
│  - PartitionKey, EventStream                │
│  - Gapless versioning, Optimistic locking   │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│      Infrastructure (Repositories)          │
│  ┌─────────────────────────────────────┐    │
│  │  InMemoryEventStreamRepository      │    │
│  │  PostgresEventStreamRepository  ✨  │    │
│  │  RocksDBEventStreamRepository   ✨  │    │
│  └─────────────────────────────────────┘    │
│                                              │
│  ┌─────────────────────────────────────┐    │
│  │  Lock-Free Components           ✨  │    │
│  │  - LockFreeEventQueue               │    │
│  │  - LockFreeMetrics                  │    │
│  └─────────────────────────────────────┘    │
│                                              │
│  ┌─────────────────────────────────────┐    │
│  │  Clustering                      ✨  │    │
│  │  - NodeRegistry                     │    │
│  │  - RequestRouter                    │    │
│  │  - Partition Assignment             │    │
│  └─────────────────────────────────────┘    │
└──────────────────────────────────────────────┘
```

---

## 📦 Dependencies to Add

```toml
[dependencies]
# Lock-free data structures
crossbeam = "0.8"
crossbeam-queue = "0.3"

# PostgreSQL
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "json", "chrono", "uuid"] }

# RocksDB
rocksdb = "0.21"

# Clustering (future)
raft = "0.7"  # Optional: for consensus
```

---

## 🎯 Phase 4 Deliverables

### Week 1-2: Lock-Free Optimizations
- ✅ LockFreeEventQueue implementation
- ✅ LockFreeMetrics implementation
- ✅ 14 new tests
- ✅ Benchmark comparisons (before/after)

### Week 3-5: PostgreSQL Repository
- ✅ Database schema design
- ✅ Migration scripts
- ✅ PostgresEventStreamRepository
- ✅ 15 new tests
- ✅ Integration tests with real PostgreSQL

### Week 6-7: RocksDB Repository
- ✅ RocksDBEventStreamRepository
- ✅ Column family design
- ✅ 12 new tests
- ✅ Performance benchmarks

### Week 8-10: Distributed Partitioning
- ✅ NodeRegistry implementation
- ✅ RequestRouter implementation
- ✅ 10 new tests
- ✅ Multi-node test scenarios

---

## 🚀 Getting Started

### Step 1: Review Current Architecture
```bash
# Review Phase 3 implementation
cat IMPLEMENTATION_SUMMARY.md
cat SIERRADB_IMPLEMENTATION_PLAN.md

# Run existing tests
cargo test --lib

# Check benchmarks
cargo bench --bench performance_benchmarks
```

### Step 2: Set Up Development Environment
```bash
# Install PostgreSQL (for Phase 4B)
brew install postgresql@14  # macOS
# OR
sudo apt-get install postgresql-14  # Linux

# Start PostgreSQL
brew services start postgresql@14

# Create test database
createdb allsource_test
```

### Step 3: Start with Lock-Free Optimizations
```bash
# Create directories
mkdir -p src/infrastructure/persistence/lock_free

# Create first file
touch src/infrastructure/persistence/lock_free/mod.rs
touch src/infrastructure/persistence/lock_free/queue.rs
touch src/infrastructure/persistence/lock_free/metrics.rs

# Add dependencies
echo 'crossbeam = "0.8"' >> Cargo.toml
echo 'crossbeam-queue = "0.3"' >> Cargo.toml
```

---

## 📈 Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| PostgreSQL complexity | Medium | Start with simple schema, iterate |
| RocksDB operational overhead | Low | Provide embedded option |
| Distributed systems bugs | High | Extensive testing, start single-node |
| Lock-free correctness | High | Use battle-tested crossbeam crates |
| Performance regression | Medium | Continuous benchmarking |

---

## 🎓 Learning Resources

### Lock-Free Programming
- [Crossbeam Documentation](https://docs.rs/crossbeam/)
- [Rust Atomics and Locks](https://marabos.nl/atomics/)

### PostgreSQL
- [SQLx Documentation](https://docs.rs/sqlx/)
- [PostgreSQL Performance Tuning](https://wiki.postgresql.org/wiki/Performance_Optimization)

### RocksDB
- [RocksDB Rust Bindings](https://docs.rs/rocksdb/)
- [RocksDB Tuning Guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide)

### Distributed Systems
- [Raft Consensus](https://raft.github.io/)
- [Designing Data-Intensive Applications](https://dataintensive.net/)

---

## ✅ Definition of Done

Phase 4 is complete when:

1. ✅ All 51 new tests passing (308+ total tests)
2. ✅ PostgreSQL repository production-ready
3. ✅ RocksDB repository benchmarked
4. ✅ Lock-free queue integrated in ingestion path
5. ✅ Throughput target achieved (1M+ events/sec)
6. ✅ Multi-node clustering foundation ready
7. ✅ Documentation updated
8. ✅ Migration guides written
9. ✅ Backward compatibility maintained
10. ✅ Performance benchmarks updated

---

**Next**: Phase 5 - Security & Multi-tenancy (v1.0 features)
