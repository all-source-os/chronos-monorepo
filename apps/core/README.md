---
title: "AllSource Core"
status: CURRENT
last_updated: 2026-02-15
category: service
port: 3900
technology: Rust
version: "0.10.7"
---

# AllSource Core - High-Performance Event Store

> Purpose-built event store for modern event-sourcing and CQRS architectures. Zero external database dependencies — AllSource Core **is** the database.

[![crates.io](https://img.shields.io/crates/v/allsource-core.svg)](https://crates.io/crates/allsource-core)
[![docs.rs](https://docs.rs/allsource-core/badge.svg)](https://docs.rs/allsource-core)
[![CI](https://github.com/all-source-os/all-source/actions/workflows/ci.yml/badge.svg)](https://github.com/all-source-os/all-source/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-1%2C300%20passing-brightgreen.svg)]()
[![Performance](https://img.shields.io/badge/throughput-469K%20events%2Fsec-blue.svg)]()
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../../LICENSE)

**Current Version**: v0.10.7 | [crates.io](https://crates.io/crates/allsource-core) | [docs.rs](https://docs.rs/allsource-core)

## What is AllSource Core?

AllSource Core is a Rust-native event store that replaces the traditional "app + database" stack with a single high-performance binary. It provides durable event storage, real-time projections, schema validation, stream processing, and built-in replication — all without PostgreSQL, Redis, or Kafka.

**Use it as a library** embedded in your Rust application, or **run it as a standalone server** with a REST API and WebSocket streaming.

### Why AllSource?

| Traditional Stack | AllSource Core |
|-------------------|---------------|
| App + PostgreSQL + Kafka + Redis | Single binary, single process |
| 469K events/sec requires tuning across services | 469K events/sec out of the box |
| Schema changes require migrations | Schema registry with compatibility modes |
| Stream processing needs Kafka/Flink | 6 built-in operators, zero setup |
| Replication via PgBouncer + logical replication | WAL shipping with automatic failover |

## Quick Start

### 30-Second Demo

```bash
# Start the server
docker run -p 3900:3900 ghcr.io/all-source-os/allsource-core:0.10.7

# Ingest an event
curl -X POST http://localhost:3900/api/v1/events \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "user.created",
    "entity_id": "user-123",
    "payload": {
      "name": "Alice",
      "email": "alice@example.com"
    }
  }'

# Query it back
curl "http://localhost:3900/api/v1/events/query?entity_id=user-123"

# Time-travel: see entity state at any point in time
curl "http://localhost:3900/api/v1/entities/user-123/state?as_of=2026-01-01T00:00:00Z"

# Check health
curl http://localhost:3900/health
```

### Install as a Library

```bash
cargo add allsource-core@0.10
```

Or in `Cargo.toml`:

```toml
[dependencies]
allsource-core = "0.10"
```

### Build from Source

```bash
git clone https://github.com/all-source-os/all-source.git
cd all-source/apps/core
cargo run --release
```

### Binaries

| Binary | Description |
|--------|-------------|
| `allsource-core` | Main event store server (port 3900) |
| `allsource-admin` | Admin CLI for maintenance operations |
| `allsource-sentinel` | Automated leader election and failover |

## Performance

Measured on Apple Silicon M-series (release build):

| Operation | Throughput / Latency |
|-----------|---------------------|
| Event ingestion | **469K events/sec** (single-threaded) |
| Entity query | **11.9 us** (indexed lookup) |
| Type query | 2.4 ms (cross-entity scan) |
| State reconstruction | 3.5 us (with snapshots) |
| Concurrent writes (8 workers) | 8.0 ms / 100-event batch |
| Parquet batch write | 3.5 ms / 1,000 events |
| Snapshot creation | 130 us per entity |
| WAL sync writes | 413 ms / 100 syncs |

**Why it's fast:**
- Lock-free indexing via DashMap (sharded concurrent HashMap)
- Arena-based memory pooling (2-5ns allocations)
- SIMD-accelerated JSON parsing
- Zero-copy field access on hot path
- Columnar storage with Apache Arrow/Parquet

## Storage & Durability

AllSource Core provides full durability with no external database required. Event data survives process restarts, crashes, and power loss.

| Layer | Purpose | Detail |
|-------|---------|--------|
| **WAL** | Crash recovery | CRC32 checksums, configurable fsync (default 100ms), segment rotation |
| **Parquet** | Columnar persistence | Snappy compression, periodic flush, analytics-optimized |
| **DashMap** | In-memory reads | Lock-free concurrent map, O(1) lookups |
| **Snapshots** | Point-in-time state | Automatic creation, entity-level, optimized reconstruction |

**Data flow on ingestion:**

```
Event In → WAL Write → Index Update → Projection Update → Pipeline Processing
                                                              ↓
              WebSocket Broadcast ← Parquet Flush ← Auto-Snapshot
```

### Automatic Compaction

Background compaction merges WAL segments and Parquet files, reclaiming storage and improving read performance without downtime.

```bash
# Trigger manual compaction
curl -X POST http://localhost:3900/api/v1/compaction/trigger

# Check compaction stats
curl http://localhost:3900/api/v1/compaction/stats
```

## Replication

Leader-follower replication via WAL shipping. No Raft, no ZooKeeper, no external coordination service.

```
Leader (port 3900, replication port 3910)
  |-- WAL Ship --> Follower 1 (port 3900)
  |-- WAL Ship --> Follower 2 (port 3900)
```

| Mode | Behavior | Use Case |
|------|----------|----------|
| `async` (default) | Fire-and-forget, lowest latency | Most workloads |
| `semi-sync` | Wait for 1 follower ACK | Balances durability and speed |
| `sync` | Wait for all follower ACKs | Maximum durability |

The `allsource-sentinel` binary monitors follower health and triggers automatic failover when the leader becomes unavailable.

```bash
# Configure via environment
ALLSOURCE_ROLE=leader        # or "follower"
ALLSOURCE_REPLICATION_PORT=3910
ALLSOURCE_LEADER_URL=http://leader:3910  # followers only
```

## Features

### Core Event Store
- **Immutable event log**: Append-only storage with complete audit trail
- **Time-travel queries**: Reconstruct entity state as of any timestamp
- **Batch ingestion**: High-throughput bulk event loading
- **Real-time projections**: Built-in materialized views updated on every event
- **Multi-tenancy**: Per-tenant isolation with configurable quotas and rate limits

### Schema Registry

Enforce event contracts at ingestion time. Prevent breaking changes before they reach production.

```bash
# Register a schema
curl -X POST http://localhost:3900/api/v1/schemas \
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

# Now invalid events are rejected at ingestion
```

- JSON Schema validation at ingestion time
- Automatic schema versioning with compatibility checking
- **Compatibility modes**: Backward, Forward, Full, or None
- Subject-based organization by domain or event type

### Stream Processing

Six built-in operators that process events in real time during ingestion — no external stream processor needed.

```bash
# Create a pipeline that counts US users by country
curl -X POST http://localhost:3900/api/v1/pipelines \
  -H "Content-Type: application/json" \
  -d '{
    "name": "us_user_analytics",
    "source_event_types": ["user.created", "user.updated"],
    "operators": [
      {"type": "filter", "field": "country", "value": "US", "op": "eq"},
      {"type": "reduce", "field": "id", "function": "count", "group_by": "country"}
    ],
    "enabled": true
  }'
```

| Operator | Description |
|----------|-------------|
| **Filter** | eq, ne, gt, lt, contains — route events by field values |
| **Map** | Transform fields: uppercase, lowercase, trim, math operations |
| **Reduce** | Aggregations: count, sum, avg, min, max with grouping |
| **Window** | Time-based aggregations: tumbling, sliding, session windows |
| **Enrich** | Lookup external data and attach to events |
| **Branch** | Conditional routing to different downstream processors |

### Event Replay

Rebuild projections and materialized views from historical events with full progress tracking.

```bash
# Start a replay from a specific point in time
curl -X POST http://localhost:3900/api/v1/replay \
  -H "Content-Type: application/json" \
  -d '{
    "projection_name": "user_snapshot",
    "from_timestamp": "2026-01-01T00:00:00Z",
    "config": {"batch_size": 1000, "emit_progress": true}
  }'

# Monitor progress
curl http://localhost:3900/api/v1/replay/{replay_id}
```

- Configurable batch sizes for optimal throughput
- Async execution with cancellation support
- Real-time progress metrics (events/sec, percentage complete)

### Analytics Engine

Built-in analytics without exporting to a separate data warehouse.

```bash
# Event frequency over time
curl "http://localhost:3900/api/v1/analytics/frequency?event_type=user.created&bucket_size=3600"

# Statistical summary for an entity
curl "http://localhost:3900/api/v1/analytics/summary?entity_id=user-123"

# Correlation between event types
curl "http://localhost:3900/api/v1/analytics/correlation?event_a=user.created&event_b=order.placed"
```

### Security & Multi-Tenancy

Production-ready security with zero configuration for development.

- **JWT authentication** with role-based access control (Admin, Operator, Reader)
- **API key authentication** for service-to-service communication
- **Per-tenant rate limiting** with configurable quotas
- **IP filtering** with global allowlist/blocklist
- **Audit logging** event-sourced and backed by Core's own WAL
- **Tenant isolation** at the repository level with storage quotas

### Optional Feature Flags

Enable additional capabilities via Cargo features:

| Feature | Flag | Dependencies | Use Case |
|---------|------|-------------|----------|
| Embedded API | `embedded` | — | Use Core as an in-process library (`EmbeddedCore`) |
| Embedded TOON | `embedded-toon` | toon-format | `query_toon()` — TOON-encoded output (~50% fewer tokens for LLMs) |
| Vector search | `vector-search` | fastembed, instant-distance | Semantic event search, similarity matching |
| Keyword search | `keyword-search` | tantivy | Full-text search across event payloads |
| RocksDB storage | `rocksdb-storage` | rocksdb | Alternative persistent storage backend |

```toml
[dependencies]
allsource-core = { version = "0.10", features = ["embedded", "embedded-toon"] }
```

## Programmatic API

Use AllSource Core as an embedded library in your Rust application:

```rust
use allsource_core::{EventStore, Event, QueryEventsRequest};
use serde_json::json;

// Create an event store
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

// Reconstruct current state
let state = store.reconstruct_state("user-123", None)?;

// Time-travel: state as of 1 hour ago
let timestamp = chrono::Utc::now() - chrono::Duration::hours(1);
let past_state = store.reconstruct_state("user-123", Some(timestamp))?;
```

## API Reference

All endpoints use the `/api/v1/` prefix. Core returns wrapped responses: `{"events": [...], "count": N}`.

### Events
```
POST   /api/v1/events                    # Ingest event (returns 200)
POST   /api/v1/events/batch              # Batch ingest
GET    /api/v1/events/query              # Query (?entity_id=, ?event_type=, ?since=, ?limit=)
GET    /api/v1/entities/:id/state        # Entity state (?as_of= for time-travel)
GET    /api/v1/entities/:id/snapshot     # Latest snapshot
GET    /api/v1/stats                     # Store statistics
WS     /api/v1/events/stream             # Real-time WebSocket stream
```

### Schemas
```
POST   /api/v1/schemas                   # Register schema
GET    /api/v1/schemas                   # List subjects
GET    /api/v1/schemas/:subject          # Get schema (?version=)
GET    /api/v1/schemas/:subject/versions # List versions
POST   /api/v1/schemas/validate          # Validate event against schema
PUT    /api/v1/schemas/:subject/compatibility  # Set compatibility mode
```

### Projections & Snapshots
```
GET    /api/v1/projections               # List projections
POST   /api/v1/snapshots                 # Create snapshot
GET    /api/v1/snapshots                 # List snapshots
GET    /api/v1/snapshots/:id/latest      # Get latest snapshot
```

### Pipelines
```
POST   /api/v1/pipelines                 # Register pipeline
GET    /api/v1/pipelines                 # List pipelines
GET    /api/v1/pipelines/:id             # Get pipeline
DELETE /api/v1/pipelines/:id             # Remove pipeline
GET    /api/v1/pipelines/stats           # All pipeline stats
GET    /api/v1/pipelines/:id/stats       # Pipeline stats
PUT    /api/v1/pipelines/:id/reset       # Reset pipeline state
```

### Replay
```
POST   /api/v1/replay                    # Start replay
GET    /api/v1/replay                    # List replays
GET    /api/v1/replay/:id               # Get progress
POST   /api/v1/replay/:id/cancel        # Cancel replay
DELETE /api/v1/replay/:id               # Delete replay
```

### Analytics & Operations
```
GET    /api/v1/analytics/frequency       # Event frequency
GET    /api/v1/analytics/summary         # Statistical summary
GET    /api/v1/analytics/correlation     # Event correlation
POST   /api/v1/compaction/trigger        # Trigger compaction
GET    /api/v1/compaction/stats          # Compaction stats
GET    /health                           # Health check (root path)
GET    /metrics                          # Prometheus metrics
```

### Tenants, Auth, Audit, Config
```
POST   /api/v1/tenants                   # Create tenant
GET    /api/v1/tenants                   # List tenants
GET    /api/v1/tenants/:id              # Get tenant
PUT    /api/v1/tenants/:id              # Update tenant
DELETE /api/v1/tenants/:id              # Delete tenant
POST   /api/v1/auth/login               # Login
POST   /api/v1/auth/api-keys            # Create API key
GET    /api/v1/audit/events             # Query audit log
GET    /api/v1/config                    # Get configuration
PUT    /api/v1/config                    # Update configuration
```

## Architecture

Clean Architecture with strict layered dependencies. Inner layers never depend on outer layers. Domain has zero external dependencies.

```
src/
├── domain/                 # Business logic (zero external deps)
│   ├── entities/           # Event, Tenant, Schema, Projection, EventStream, AuditEvent
│   ├── value_objects/      # TenantId, EntityId, EventType, PartitionKey, Money
│   └── repositories/       # Trait definitions only
│
├── application/            # Use cases & orchestration
│   ├── use_cases/          # IngestEvent, QueryEvents, ManageTenant, ManageSchema
│   ├── services/           # Analytics, Pipeline, Replay, Schema validation
│   └── dto/                # Request/Response DTOs
│
├── infrastructure/         # Technical implementations
│   ├── web/                # HTTP handlers, WebSocket, API routes
│   ├── persistence/        # WAL, Parquet, Snapshots, Compaction, Indexes, SIMD
│   ├── replication/        # WAL shipper, WAL receiver, protocol
│   ├── security/           # Auth, rate limiting, middleware
│   ├── repositories/       # In-memory, event-sourced implementations
│   ├── search/             # Vector search, keyword search (optional)
│   ├── cluster/            # Cluster coordination
│   ├── observability/      # Metrics, tracing
│   ├── config/             # Configuration management
│   └── di/                 # Dependency injection (ServiceContainer)
│
├── store.rs                # EventStore core implementation
├── main.rs                 # Server entry point
├── lib.rs                  # Library exports
└── bin/
    ├── allsource-admin.rs  # Admin CLI
    └── allsource-sentinel.rs # Failover sentinel
```

### Why These Technology Choices?

| Technology | Why |
|-----------|-----|
| **Rust** | Zero-cost abstractions, no GC pauses, fearless concurrency |
| **DashMap** | Lock-free concurrent HashMap — better than `RwLock<HashMap>` for read-heavy workloads |
| **Apache Arrow/Parquet** | Industry-standard columnar format, SIMD-accelerated, interoperable with DataFusion, Polars, DuckDB |
| **Tokio + Axum** | High-performance async runtime with type-safe request handlers |
| **parking_lot** | Faster, smaller mutexes than std — no poisoning, less overhead |

## Development

### Prerequisites

- Rust 1.92+ (MSRV)
- Cargo

### Commands

```bash
cargo build --release       # Build
cargo test --lib            # Run unit tests (1,300 tests)
cargo test                  # Run all tests including integration
cargo bench                 # Run benchmarks
RUST_LOG=debug cargo run    # Run with debug logging
```

### Quality Gates

```bash
make check          # Run all quality gates
make format         # Auto-fix formatting
make ci             # Full CI pipeline locally
```

Enforced on every PR: rustfmt, clippy (zero warnings), cargo-sort, all tests passing, release build.

### Benchmarking

```bash
cargo bench                           # Run all benchmarks
cargo bench ingestion_throughput      # Specific suite
open target/criterion/report/index.html  # View results
```

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `PORT` | `3900` | HTTP server port |
| `RUST_LOG` | `info` | Log level |
| `ALLSOURCE_ROLE` | `leader` | Node role: `leader` or `follower` |
| `ALLSOURCE_REPLICATION_PORT` | `3910` | Replication listener port (leader) |
| `ALLSOURCE_LEADER_URL` | — | Leader replication URL (follower) |
| `ALLSOURCE_READ_ONLY` | `false` | Read-only mode (alternative to role=follower) |
| `ALLSOURCE_DATA_DIR` | `data/` | Data directory for WAL and Parquet files |

## License

[MIT](../../LICENSE)

---

<div align="center">

**AllSource Core** — Event sourcing without the infrastructure tax

469K events/sec | 11.9us queries | 1,300 tests | Single binary | Zero dependencies

[crates.io](https://crates.io/crates/allsource-core) | [docs.rs](https://docs.rs/allsource-core) | [GitHub](https://github.com/all-source-os/all-source)

</div>
