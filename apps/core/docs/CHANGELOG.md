# Changelog

All notable changes to AllSource Core will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2025-12-05

### Added

#### SIMD-Accelerated JSON Parsing (`simd_json.rs`)
- Zero-copy JSON parsing using `simd-json` library
- 2-3x faster parsing with SIMD instructions (AVX2, SSE4.2, NEON)
- `SimdJsonParser` for high-throughput deserialization
- `ZeroCopyJson` for read-only field access without full deserialization
- `BatchEventParser` for efficient batch processing
- `SimdJsonStats` for tracking parsing throughput and errors
- **Performance: 824K events/sec, 112 MB/s throughput**

#### Lock-Free Data Structures (`lock_free/`)
- `LockFreeEventQueue`: MPMC queue using crossbeam ArrayQueue
  - ~10-20ns push/pop operations
  - **Performance: 41M push/sec, 3.4M pop/sec**
- `ShardedEventQueue`: Distributed queue for high contention scenarios
  - 16 shards by default for cache-line optimization
  - Batch push/pop operations
  - **Performance: 1.06M events/sec with 4 threads**
- `LockFreeMetrics`: Atomic counters for zero-contention monitoring
  - Min/max/avg latency tracking with CAS operations
  - ~5-10ns per metric update

#### Batch Processing Pipeline (`batch_processor.rs`)
- `BatchProcessor`: High-throughput ingestion combining SIMD + lock-free queues
- Configurable batch sizes (1K-50K events)
- Three configuration presets:
  - `default()`: Balanced for general use
  - `high_throughput()`: Maximum events/sec (50K batch, 10M queue)
  - `low_latency()`: Quick responses (1K batch, 100K queue)
- `RawEventData`: Efficient event deserialization struct
- **Performance: 335K events/sec (single), 726K events/sec (4 threads)**

#### Arena Memory Pooling (`arena_pool.rs`)
- Thread-local arena pools using `bumpalo` allocator
- `get_arena()`: Get recycled arena from pool (~10-20ns)
- `PooledArena`: RAII wrapper with automatic pool return
- `ScopedArena`: Convenient scoped allocation pattern
- `SizedBufferPool`: Pre-allocated buffers for specific sizes
- 99%+ arena recycle rate
- **Performance: 28.5M allocations/sec**

#### Performance Validation Tests (`performance_test.rs`)
- Comprehensive benchmark suite for all optimizations
- Tests for SIMD parsing, queue throughput, batch processing
- Concurrent performance validation
- Sustained throughput testing

### Changed
- Added `simd-json = "0.14"` dependency for SIMD acceleration
- Added `bumpalo = "3.16"` dependency for arena allocation
- Added `From<SimdJsonError>` impl for `AllSourceError`
- Clean Architecture module organization maintained

### Performance
- **Lock-Free Queue (push)**: 41.1M ops/sec
- **Arena Allocations**: 28.5M allocs/sec
- **Lock-Free Queue (pop)**: 3.4M ops/sec
- **Sharded Queue (concurrent)**: 2.5M ops/sec
- **SIMD JSON Parsing**: 824K events/sec
- **Full Pipeline (4 threads)**: 726K events/sec
- **Sustained Throughput**: 418K events/sec
- **Batch Processor**: 335K events/sec

### Technical Details
- All new modules follow Clean Architecture patterns
- Full test coverage with 57+ new tests
- Debug and release mode performance thresholds
- Thread-safe implementations with `Send + Sync`

---

## [0.5.0] - 2025-10-20

### Added

#### Schema Registry
- JSON Schema-based event validation system
- Automatic schema versioning with compatibility checking
- 4 compatibility modes: None, Backward, Forward, Full
- Subject-based schema organization
- 6 new REST API endpoints for schema management:
  - `POST /api/v1/schemas` - Register schema
  - `GET /api/v1/schemas` - List subjects
  - `GET /api/v1/schemas/:subject` - Get schema
  - `GET /api/v1/schemas/:subject/versions` - List versions
  - `POST /api/v1/schemas/validate` - Validate event
  - `PUT /api/v1/schemas/:subject/compatibility` - Set compatibility mode

#### Event Replay Engine
- Point-in-time event replay functionality
- Projection rebuilding with progress tracking
- Configurable batch processing
- Async background execution using Tokio
- Cancellable replay operations
- 5 replay statuses: Pending, Running, Completed, Failed, Cancelled
- Real-time progress metrics (events/sec, percentage complete)
- 5 new REST API endpoints for replay management:
  - `POST /api/v1/replay` - Start replay
  - `GET /api/v1/replay` - List replays
  - `GET /api/v1/replay/:replay_id` - Get progress
  - `POST /api/v1/replay/:replay_id/cancel` - Cancel replay
  - `DELETE /api/v1/replay/:replay_id` - Delete replay

#### Stream Processing Pipelines
- 6 pipeline operators:
  - **Filter**: eq, ne, gt, lt, contains operations
  - **Map**: uppercase, lowercase, trim, multiply, add transformations
  - **Reduce**: count, sum, avg, min, max aggregations with grouping
  - **Window**: tumbling, sliding, session windows for time-based aggregations
  - **Enrich**: external data lookup and enrichment (placeholder)
  - **Branch**: conditional event routing
- Stateful processing with thread-safe state management
- Window buffers with automatic time-based eviction
- Pipeline statistics tracking
- Integrated pipeline processing into event ingestion flow
- 7 new REST API endpoints for pipeline management:
  - `POST /api/v1/pipelines` - Register pipeline
  - `GET /api/v1/pipelines` - List pipelines
  - `GET /api/v1/pipelines/:pipeline_id` - Get pipeline
  - `DELETE /api/v1/pipelines/:pipeline_id` - Remove pipeline
  - `GET /api/v1/pipelines/stats` - All pipeline stats
  - `GET /api/v1/pipelines/:pipeline_id/stats` - Pipeline stats
  - `PUT /api/v1/pipelines/:pipeline_id/reset` - Reset state

### Changed
- Enhanced event ingestion flow to include pipeline processing
- Updated `ProjectionManager::get_projection()` to return cloned `Arc` instead of reference
- Improved event ingestion performance by 4-14% with pipeline integration optimizations

### Performance
- Ingestion: 442-469K events/sec (single-threaded)
- Entity query: 11.9 μs
- State reconstruction: 3.5 μs (with snapshots)
- 48 tests passing (33 unit + 15 integration)

---

## [0.2.0] - 2025-01-15

### Added

#### Persistent Storage
- Apache Parquet columnar storage for events
- Write-Ahead Log (WAL) for crash recovery and durability
- Automatic compaction with 3 strategies:
  - Size-based compaction
  - Count-based compaction
  - Age-based compaction
- Point-in-time snapshot system
- Automatic snapshot creation based on configurable thresholds

#### Real-time Streaming
- WebSocket server for real-time event broadcasting
- Client connection management
- Event subscription and filtering

#### Advanced Analytics
- Event frequency analysis with time bucketing
- Event correlation analysis
- Statistical summaries (count, avg, min, max)
- Time-window aggregations

#### API Endpoints
- 18 new REST API endpoints:
  - WebSocket: `WS /api/v1/events/stream`
  - Analytics: `/api/v1/analytics/*` (3 endpoints)
  - Snapshots: `/api/v1/snapshots/*` (3 endpoints)
  - Compaction: `/api/v1/compaction/*` (2 endpoints)

### Changed
- Event ingestion now writes to WAL first for durability
- State reconstruction optimized with snapshot fallback
- Enhanced storage architecture with multiple layers

### Performance
- 10-15% improvement in ingestion throughput with Parquet batching
- 100x faster state reconstruction with snapshots
- Zero data loss on crashes with WAL

---

## [0.1.0] - 2024-12-01

### Added

#### Core Event Store
- In-memory event storage with `Vec<Event>`
- Immutable append-only event log
- Event ID generation with UUID v7
- ISO 8601 timestamp support
- JSON payload storage

#### Indexing System
- DashMap-based concurrent indexing
- Entity ID index (O(1) lookup)
- Event type index (O(1) lookup)
- Event ID index (direct access)
- Thread-safe concurrent updates

#### Query Engine
- Query by entity ID
- Query by event type
- Time-travel queries with `as_of` parameter
- Time range filtering with `since`/`until`
- Result limiting
- Entity state reconstruction

#### Projections
- Real-time projection system
- Built-in projections:
  - EntitySnapshotProjection (current state per entity)
  - EventCounterProjection (event type statistics)
- Custom projection trait for user-defined projections

#### REST API
- 8 initial REST API endpoints:
  - `GET /health` - Health check
  - `POST /api/v1/events` - Ingest event
  - `GET /api/v1/events/query` - Query events
  - `GET /api/v1/entities/:entity_id/state` - Get entity state
  - `GET /api/v1/entities/:entity_id/snapshot` - Get snapshot
  - `GET /api/v1/stats` - System statistics

#### Error Handling
- Comprehensive error types
- HTTP status code mapping
- Type-safe error handling with `Result<T, AllSourceError>`

#### Testing
- 10+ unit tests
- 5+ integration tests
- Performance benchmarks with Criterion

### Performance
- 100K+ events/sec ingestion
- Sub-millisecond entity queries
- Concurrent read/write support

---

## [Unreleased]

### Planned for v0.7 - Advanced Features

- [ ] Multi-tenancy support
- [ ] Event encryption at rest
- [ ] Audit logging
- [ ] Retention policies
- [ ] Data archival
- [ ] Backup/restore utilities
- [ ] RBAC (Role-Based Access Control)
- [ ] API rate limiting

### Planned for v1.0 - Distributed & Cloud-Native

- [ ] Distributed replication (Raft consensus)
- [ ] Multi-region support
- [ ] Horizontal scaling
- [ ] Arrow Flight RPC
- [ ] Kubernetes operators
- [ ] Helm charts
- [ ] Load balancing
- [ ] Health checks and readiness probes
- [ ] Prometheus metrics
- [ ] OpenTelemetry tracing

Target: 10M+ events/sec (distributed), 99.99% availability

### Future Considerations

- [ ] GraphQL API
- [ ] WASM plugin system
- [ ] Change Data Capture (CDC)
- [ ] Time-series optimization
- [ ] Machine learning integrations
- [ ] Real-time anomaly detection
- [ ] Event sourcing templates
- [ ] Visual query builder

---

## Version History

| Version | Date | Status | Highlights |
|---------|------|--------|------------|
| [0.6.0] | 2025-12-05 | ✅ Current | SIMD JSON, lock-free queues, batch processing, arena pools |
| [0.5.0] | 2025-10-20 | ✅ Stable | Schema registry, event replay, stream processing |
| [0.2.0] | 2025-01-15 | ✅ Stable | Parquet storage, WAL, snapshots, analytics |
| [0.1.0] | 2024-12-01 | ✅ Stable | Core event store, indexing, projections |

---

## Upgrade Notes

### Upgrading from 0.5.0 to 0.6.0

**Breaking Changes**: None

**New Features**: All performance optimizations are opt-in and transparent.

**Dependencies**:
- Added `simd-json = "0.14"` for SIMD-accelerated JSON parsing
- Added `bumpalo = "3.16"` for arena memory allocation

**New Modules** (all in `infrastructure::persistence`):
- `simd_json`: SIMD JSON parsing utilities
- `lock_free::sharded_queue`: High-throughput sharded queue
- `batch_processor`: Batch processing pipeline
- `arena_pool`: Thread-local arena memory pools
- `performance_test`: Performance validation tests

**Migration Steps**:
1. Update dependencies: `cargo update`
2. Rebuild: `cargo build --release`
3. Run tests: `cargo test`
4. No data migration required
5. Optionally integrate new batch processing APIs for higher throughput

**Performance Improvements**:
- JSON parsing: 2-3x faster with SIMD
- Queue operations: 10-100x faster with lock-free structures
- Memory allocation: 5-10x faster with arena pooling
- Full pipeline: 726K events/sec (up from 469K)

### Upgrading from 0.2.0 to 0.5.0

**Breaking Changes**: None

**New Features**: All new features are opt-in and don't affect existing functionality.

**Configuration**:
- New `SchemaRegistryConfig` added to `EventStoreConfig` (defaults provided)
- New managers: `ReplayManager` and `PipelineManager` (automatically initialized)

**API Changes**:
- 12 new API endpoints (all additive)
- Existing endpoints unchanged

**Migration Steps**:
1. Update dependencies: `cargo update`
2. Rebuild: `cargo build --release`
3. Run tests: `cargo test`
4. No data migration required

### Upgrading from 0.1.0 to 0.2.0

**Breaking Changes**: None

**New Features**: All new features are opt-in.

**Configuration**:
- New optional storage configuration for Parquet persistence
- New optional WAL configuration
- New optional snapshot configuration

**Migration Steps**:
1. Update dependencies
2. Rebuild application
3. Optionally configure persistent storage
4. No data migration required for in-memory mode

---

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
