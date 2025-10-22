# AllSource Architecture

## Overview

AllSource is built as a multi-layer system where each component is optimized for its specific role:

```
┌─────────────────────────────────────────────────────────────┐
│                     Presentation Layer                      │
│                                                              │
│  ┌──────────────┐              ┌────────────────┐          │
│  │   Web UI     │              │   MCP Server   │          │
│  │  (Next.js)   │              │  (TypeScript)  │          │
│  └──────────────┘              └────────────────┘          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Orchestration Layer                      │
│                                                              │
│              ┌────────────────────────────┐                 │
│              │    Control Plane (Go)      │                 │
│              │  • Cluster Management      │                 │
│              │  • Health Monitoring       │                 │
│              │  • Snapshot Coordination   │                 │
│              └────────────────────────────┘                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       Data Layer                             │
│                                                              │
│              ┌────────────────────────────┐                 │
│              │   Event Store (Rust)       │                 │
│              │  • Columnar Storage        │                 │
│              │  • SIMD Vectorization      │                 │
│              │  • Time-Travel Queries     │                 │
│              │  • Entity Indexing         │                 │
│              └────────────────────────────┘                 │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Details

### 1. Event Store Core (Rust)

**Location:** `services/core/`

**Responsibilities:**
- High-performance event ingestion
- Columnar storage (in-memory demo, Arrow/Parquet for production)
- Time-travel query execution
- Entity state reconstruction
- Indexing by entity_id and event_type

**Key Modules:**

```rust
// event.rs - Core event types and schemas
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub entity_id: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub version: i64,
}

// store.rs - Event storage and querying
pub struct EventStore {
    events: Vec<Event>,
    entity_index: HashMap<String, Vec<usize>>,
    type_index: HashMap<String, Vec<usize>>,
}

// api.rs - REST API endpoints
- POST /api/v1/events - Ingest event
- GET /api/v1/events/query - Query events
- GET /api/v1/entities/:id/state - Reconstruct state
- GET /api/v1/stats - Get statistics
```

**Performance Characteristics:**
- **Ingestion:** O(1) append + O(1) index updates
- **Entity Query:** O(log n) via HashMap index
- **Time-Travel:** O(m) where m = events for entity
- **Full Scan:** O(n) with SIMD potential

**Future Optimizations:**
- Apache Arrow columnar layout
- Parquet file storage
- SIMD-accelerated filtering
- Memory-mapped file access

---

### 2. Control Plane (Go)

**Location:** `services/control-plane/`

**Responsibilities:**
- Cluster health monitoring
- Node coordination
- Snapshot orchestration
- Metrics aggregation
- API gateway for management operations

**Key Endpoints:**

```go
GET  /health                    // Health check
GET  /health/core               // Check core service
GET  /api/v1/cluster/status     // Cluster information
GET  /api/v1/metrics            // Aggregated metrics
POST /api/v1/operations/snapshot // Trigger snapshot
POST /api/v1/operations/replay   // Initiate replay
```

**Why Go?**
- Excellent Kubernetes ecosystem integration
- Strong standard library for distributed systems
- Fast compilation for rapid iteration
- Native support for concurrent operations

**Future Features:**
- Kubernetes operator for auto-scaling
- Raft consensus for distributed coordination
- Terraform provider for infrastructure-as-code
- Prometheus metrics export

---

### 3. MCP Server (TypeScript)

**Location:** `packages/mcp-server/`

**Responsibilities:**
- Model Context Protocol interface
- Natural language query translation
- Structured data formatting for LLMs
- AI context batching and optimization

**Available Tools:**

```typescript
1. query_events
   - Filter by entity, type, time range
   - Time-travel support via as_of

2. reconstruct_state
   - Replay events to rebuild entity state
   - Temporal snapshots

3. ingest_event
   - Programmatic event creation
   - Metadata support

4. get_stats
   - Event store metrics

5. get_cluster_status
   - Cluster health and topology
```

**Integration Example:**

```typescript
// LLM can call this via MCP
const result = await mcp.callTool('query_events', {
  entity_id: 'user-123',
  as_of: '2024-01-15T10:30:00Z'
});

// Returns structured temporal data
{
  events: [...],
  count: 5
}
```

**Why TypeScript + MCP?**
- Native AI integration via standardized protocol
- Auto-generated SDK support (Speakeasy)
- Rich type system for data validation
- Seamless LLM context management

---

### 4. Web UI (Next.js)

**Location:** `apps/web/`

**Responsibilities:**
- Visual event exploration
- Real-time statistics dashboard
- Interactive query builder
- Event payload inspection
- Demo data generation

**Key Features:**

```typescript
// Real-time stats
<StatCard title="Total Events" value={stats.total_events} />

// Query filters
- Entity ID filter
- Event type filter
- Time range selection

// Event visualization
- Expandable event cards
- JSON payload viewing
- Timestamp formatting
```

**Why Next.js?**
- Server-side rendering for fast initial load
- React Server Components for optimized data fetching
- Built-in API routes for BFF pattern
- Excellent TypeScript support

---

## Data Flow

### Event Ingestion Flow

```
User/System → HTTP POST
              ↓
         API Handler (Axum)
              ↓
         Event Creation
              ↓
    ┌────────┴────────┐
    │                 │
    ▼                 ▼
 Entity Index    Type Index
    │                 │
    └────────┬────────┘
             ▼
      Events Vector
             ▼
        Response
```

### Time-Travel Query Flow

```
User → Query(entity_id, as_of)
           ↓
    Entity Index Lookup  ← O(1) HashMap
           ↓
    Filter by Timestamp  ← O(m) sequential
           ↓
    Sort by Timestamp    ← O(m log m)
           ↓
    Reconstruct State    ← O(m) replay
           ↓
       Response
```

---

## Storage Strategy

### Current (Demo)

```
In-Memory Vector
├── Events: Vec<Event>
├── Entity Index: HashMap<String, Vec<usize>>
└── Type Index: HashMap<String, Vec<usize>>
```

**Pros:** Fast, simple, great for demos
**Cons:** Not persistent, limited by RAM

### Future (Production)

```
Parquet Files (Columnar)
├── events/
│   ├── 2024-01-15-00.parquet
│   ├── 2024-01-15-01.parquet
│   └── ...
├── indices/
│   ├── entity_index.arrow
│   └── type_index.arrow
└── snapshots/
    ├── snapshot-1705334400.parquet
    └── ...
```

**Pros:**
- Persistent storage
- 10× compression
- SIMD-accelerated queries
- Incremental backups

**Cons:**
- More complex implementation
- Requires Arrow/Parquet expertise

---

## Scalability Plan

### Phase 1: Single Node (Current)
- In-memory storage
- Single Rust process
- Suitable for: 1M events, <10 concurrent users

### Phase 2: Vertical Scaling
- Parquet-backed storage
- Memory-mapped files
- Suitable for: 100M events, <100 concurrent users

### Phase 3: Horizontal Scaling
- Multi-node cluster
- Sharding by entity_id hash
- Raft consensus
- Suitable for: 1B+ events, 1000+ concurrent users

### Phase 4: Cloud-Native
- Kubernetes operator
- Auto-scaling based on ingestion rate
- Distributed query execution
- S3/GCS for cold storage

---

## Security Considerations

### Current Demo
- No authentication (localhost only)
- No encryption
- No access control

### Production Requirements

1. **Authentication**
   - JWT tokens
   - API key management
   - OAuth 2.0 integration

2. **Authorization**
   - Role-based access control (RBAC)
   - Entity-level permissions
   - Audit logging

3. **Encryption**
   - TLS for all HTTP traffic
   - At-rest encryption for Parquet files
   - Key management via KMS

4. **Compliance**
   - GDPR: Right to be forgotten (event tombstones)
   - SOC 2: Audit trail integrity
   - HIPAA: PHI data handling

---

## Monitoring & Observability

### Metrics to Track

```
Event Store:
- events_ingested_total (counter)
- events_ingested_per_second (gauge)
- query_latency_seconds (histogram)
- active_entities (gauge)
- storage_bytes_used (gauge)

Control Plane:
- cluster_nodes_healthy (gauge)
- snapshot_duration_seconds (histogram)
- api_request_duration_seconds (histogram)

MCP Server:
- mcp_calls_total (counter)
- mcp_latency_seconds (histogram)
```

### Logging Strategy

```
Rust (tracing):
- INFO: Event ingestion, query results
- DEBUG: Index operations, cache hits
- ERROR: API errors, storage failures

Go (structured logging):
- INFO: Cluster events, health checks
- WARN: Degraded node performance
- ERROR: Coordination failures
```

---

## Testing Strategy

### Unit Tests
- Event creation and validation
- Index operations
- State reconstruction logic

### Integration Tests
- API endpoint testing
- Multi-service communication
- MCP tool invocation

### Performance Tests
- Ingestion throughput benchmarks
- Query latency under load
- Concurrent user simulation

### Demo Tests
```bash
# Verify all services are healthy
make test-health

# Run demo script end-to-end
./demo-script.sh

# Load test with vegeta
echo "POST http://localhost:8080/api/v1/events" | \
  vegeta attack -duration=10s -rate=1000 | \
  vegeta report
```

---

## Future Architecture Enhancements

1. **WASM Plugin System**
   - Custom event processors
   - User-defined projections
   - Sandboxed execution

2. **GraphQL Layer**
   - Type-safe queries
   - Real-time subscriptions
   - Schema introspection

3. **Blockchain Integration**
   - Event hash notarization
   - Tamper-proof audit trail
   - Cross-organization verification

4. **Federated Queries**
   - Query across multiple AllSource instances
   - Privacy-preserving data sharing
   - Cross-company analytics

---

## Development Workflow

```bash
# 1. Make changes
vim services/core/src/store.rs

# 2. Test locally
cd services/core && cargo test

# 3. Run full system
pnpm dev

# 4. Verify in browser
open http://localhost:3000

# 5. Run demo script
./demo-script.sh

# 6. Commit
git add -A
git commit -m "feat: add time-travel optimization"
```

---

## Performance Targets

| Metric | Target (v0.5) | Target (v1.0) |
|--------|---------------|---------------|
| Ingestion Rate | 100K events/sec | 1M+ events/sec |
| Query Latency (p99) | <100ms | <10ms |
| Time-Travel Latency | <500ms | <50ms |
| Storage Efficiency | 5:1 compression | 10:1 compression |
| Concurrent Queries | 100 | 10,000 |

---

<div align="center">

**AllSource Architecture** - *Built for speed, designed for intelligence*

</div>
