# AllSource Event Store - Comprehensive Roadmap

**Last Updated**: 2025-10-29
**Version**: 1.0 → 2.0
**Vision**: A high-performance, AI-native event store combining Rust, Go, and Clojure
**Influenced By**:
- SierraDB architecture patterns for production readiness
- Agentic Postgres principles for AI-first design

---

## 🎯 Mission Statement

Build a production-grade, **AI-native** event store that combines:
- **Rust** for ultra-high-performance core operations (469K+ events/sec)
- **Go** for robust control plane and operational tooling
- **Clojure** for expressive data processing and interactive development
- **Clean Architecture** principles across all codebases
- **SOLID principles** for maintainability and extensibility
- **🆕 SierraDB-inspired patterns** for production readiness and simplified scaling
- **🤖 Agentic Postgres principles** for AI-first, agent-friendly design

---

## 🎓 Key Learnings from SierraDB (Integrated into Roadmap)

**SierraDB** is a production-grade event store that learned valuable lessons the hard way. We've integrated their insights to shortcut our path to production:

### 1. ✅ Partition-Based Architecture (Added to v1.1)
**Their lesson**: Fixed partitions (32 single-node, 1024+ cluster) enable sequential writes, gapless sequences, and horizontal scaling without complex coordination.

**Our integration**:
- Add `PartitionKey` value object in domain layer
- Partition-aware indexing in infrastructure
- Foundation for v1.8 clustering (saves rework later)

### 2. ✅ Gapless Version Guarantees (Added to v1.1)
**Their lesson**: Event sourcing demands gapless stream versions. Solved with watermark system tracking "highest continuously confirmed sequence."

**Our integration**:
- `EventStream` aggregate with optimistic locking
- Watermark tracking for consistent reads
- Prevents data inconsistencies in production

### 3. ✅ Long-Running Stress Tests (Added to v1.1)
**Their lesson**: "SierraDB runs stably under long-running stress tests, with major corruption issues ironed out" - they found corruption through 7-day stress tests.

**Our integration**:
- 7-day continuous ingestion tests
- Storage integrity verification
- Corruption detection on startup
- Partition load balancing tests

### 4. ✅ Redis Protocol Compatibility (Optional in v1.2)
**Their lesson**: "Every language with a Redis client works immediately" - RESP3 protocol adoption eliminated driver development.

**Our integration**:
- Optional RESP3 server (2-3 weeks)
- Instant multi-language support
- Debug with redis-cli
- HTTP API remains primary

### 5. ✅ Simplified Consensus (v1.8 redesign)
**Their lesson**: "Term-based consensus inspired by Raft with deterministic leader selection" - avoided full Raft complexity.

**Our integration**:
- Term-based consensus (not full Raft)
- Deterministic leader selection (no elections)
- Manual failover in v1.8, automatic in v1.9
- **Saves 3-5 weeks** vs full Raft implementation

### 6. ✅ Production Readiness Priorities
**Their lesson**: Documentation and test coverage were deprioritized, causing pain later.

**Our advantage**: We already have:
- 219 tests (98.9% pass rate) ✅
- Comprehensive documentation ✅
- Clean architecture progress ✅

**Our additions** (from their lessons):
- Storage checksums (prevent silent corruption)
- WAL integrity verification
- Partition monitoring
- Automated stress testing in CI

---

## 📊 SierraDB Comparison

| Feature | SierraDB | AllSource (Current) | AllSource (Post-v1.1) |
|---------|----------|---------------------|----------------------|
| **Partitions** | ✅ 32 (fixed) | ❌ Not yet | ✅ 32 (fixed) |
| **Gapless Versions** | ✅ Watermarks | 🟡 Ordering only | ✅ Watermarks |
| **Protocol** | ✅ RESP3 | ❌ HTTP only | 🟡 HTTP + RESP3 (opt) |
| **Consensus** | ✅ Term-based | ❌ Single-node | ✅ Term-based (v1.8) |
| **Stress Tests** | ✅ 7-day | ❌ Short benchmarks | ✅ 7-day (v1.1) |
| **Test Coverage** | 🟡 Weak | ✅ 219 tests (99%) | ✅ Maintained |
| **Documentation** | 🟡 Incomplete | ✅ Comprehensive | ✅ Enhanced |
| **Clean Arch** | ❌ Not mentioned | 🟡 67% (Rust pending) | ✅ 100% (v1.1) |

**Our Advantage**: Strong testing + documentation foundation, now adding SierraDB's production-hardened patterns.

---

## 🤖 Key Learnings from Agentic Postgres (Integrated into Roadmap)

**Agentic Postgres** pioneered the concept of databases built specifically for AI agents, not humans. We're integrating their agent-first principles into AllSource.

### Core Philosophy: "Agents Don't Behave Like Humans"

**Their insight**: AI agents require fundamentally different interfaces - they need **embedded expertise**, **instant experimentation**, and **machine-speed operations** without human friction.

### 1. ✅ Model Context Protocol with Embedded Expertise (Added to Phase 1.5)
**Their lesson**: "The MCP server embeds 10+ years of Postgres expertise through built-in prompts, enabling agents to handle schema design, query optimization, and migrations autonomously."

**Our integration**:
- Enhance MCP server with embedded event sourcing expertise
- Add agent guidance prompts to each tool (best practices, common patterns, performance tips)
- Create `get_query_advice` tool for use-case-specific recommendations
- Multi-turn conversational context for iterative agent exploration
- Self-documenting, composable operations

### 2. ✅ Instant Experimentation via Copy-on-Write Forks (Added to v1.1)
**Their lesson**: "A copy-on-write storage layer enables agents to spin up isolated production-data environments in seconds without data duplication costs."

**Our integration**:
- Copy-on-write event store forks for sandboxed testing
- Agents can test queries, projections, and migrations without affecting production
- Time-to-live (TTL) for automatic cleanup
- `create_sandbox_fork` and `run_experiment` MCP tools
- Perfect for agent learning and iterative refinement

### 3. ✅ Native Search Capabilities (Added to v1.2)
**Their lesson**: "pgvectorscale + pg_textsearch provide vector and keyword search, eliminating external dependencies."

**Our integration**:
- Vector search for semantic event queries (find similar events by meaning)
- BM25-based keyword search for full-text payload search
- Hybrid search combining semantic + keyword + metadata filters
- `semantic_search_events` MCP tool for natural language queries
- All search integrated natively in Rust core (no external services)

### 4. ✅ Function Over Interface (Design Principle)
**Their lesson**: "Prioritize function over interface, autonomy over guidance, experimentation over caution."

**Our integration**:
- MCP-first design (programmatic before visual)
- Agent-optimized query language (declarative, composable)
- Autonomous operations (auto-scaling, self-tuning, auto-schema-evolution)
- Quick-stats and sampling tools for rapid exploration
- Streaming results for real-time agent processing

### 5. ✅ Agent-Optimized Pricing Model (Design Consideration)
**Their lesson**: "Pay-per-use suits agent experimentation, where rapid iteration and teardown are expected behaviors."

**Our consideration**:
- Design for ephemeral workloads (agents spinning up/down forks)
- Resource quotas per agent/tenant
- Efficient cleanup of abandoned experiments
- Cost tracking for agent operations

### 6. ✅ Fluid Storage Infrastructure (v1.1+)
**Their lesson**: "Distributed block store delivering high IOPS, appearing as local disk while scaling like cloud storage."

**Our integration**:
- Parquet + WAL already provide efficient storage
- Partition-based architecture (from SierraDB) enables fluid distribution
- Copy-on-write forks minimize storage duplication
- Future: S3-compatible storage tier for archival

---

## 🆚 Agentic Postgres Comparison

| Feature | Agentic Postgres | AllSource (Current) | AllSource (Post-v1.2) |
|---------|------------------|---------------------|----------------------|
| **MCP Integration** | ✅ Core feature | ✅ Basic MCP server | ✅ Enhanced w/ expertise |
| **Embedded Prompts** | ✅ 10+ years expertise | ❌ Basic descriptions | ✅ Agent guidance (v1.5) |
| **Vector Search** | ✅ pgvectorscale | ❌ Not yet | ✅ Native (v1.2) |
| **Keyword Search** | ✅ BM25 | ❌ Not yet | ✅ Native (v1.2) |
| **Instant Forks** | ✅ Copy-on-write | ❌ Not yet | ✅ Copy-on-write (v1.1) |
| **Agent Autonomy** | ✅ Full autonomy | 🟡 Manual operations | ✅ Auto-ops (v1.8+) |
| **Multi-Turn Context** | 🟡 Limited | ❌ Stateless | ✅ Conversational (v1.5) |
| **Event Sourcing** | ❌ Traditional DB | ✅ Native | ✅ Enhanced |
| **Temporal Queries** | 🟡 Limited | ✅ Time-travel | ✅ Advanced (v1.3+) |
| **Clean Architecture** | ❌ Not mentioned | 🟡 In progress | ✅ 100% (v1.1) |

**Our Advantage**: Purpose-built for event sourcing + temporal queries, now adding Agentic Postgres's AI-native interface design.

---

## 📐 Architectural Philosophy

### Clean Architecture Principles

```
┌─────────────────────────────────────────────────────────┐
│              Frameworks & Drivers Layer                 │
│     (Web, DB, External APIs, CLI, Message Queue)        │
└───────────────────────┬─────────────────────────────────┘
                        │ Adapters
┌───────────────────────┴─────────────────────────────────┐
│           Interface Adapters Layer                      │
│      (Controllers, Presenters, Gateways, APIs)          │
└───────────────────────┬─────────────────────────────────┘
                        │ Use Cases
┌───────────────────────┴─────────────────────────────────┐
│            Application Business Rules                   │
│         (Use Cases, Application Services)               │
└───────────────────────┬─────────────────────────────────┘
                        │ Entities
┌───────────────────────┴─────────────────────────────────┐
│           Enterprise Business Rules                     │
│         (Entities, Value Objects, Domain)               │
└─────────────────────────────────────────────────────────┘
```

### SOLID Principles Application

| Principle | Implementation |
|-----------|---------------|
| **Single Responsibility** | Each module/struct/class has one reason to change |
| **Open/Closed** | Open for extension (traits/interfaces), closed for modification |
| **Liskov Substitution** | Abstractions can be swapped without breaking code |
| **Interface Segregation** | Small, focused interfaces/traits instead of large ones |
| **Dependency Inversion** | Depend on abstractions, not concrete implementations |

---

## ✅ Phase 1: Foundation (v1.0) - **COMPLETED** ✅

### Status: Production Ready (2025-10-21)

#### Rust Core (469K events/sec)
- ✅ High-performance event ingestion
- ✅ Write-ahead log (WAL) with durability
- ✅ Parquet storage for efficient queries
- ✅ Multi-tenant isolation with quotas
- ✅ Event indexing (entity, type-based)
- ✅ Snapshot system for state reconstruction
- ✅ Real-time WebSocket streaming
- ✅ Compaction for storage optimization
- ✅ JWT authentication & RBAC
- ✅ Rate limiting (token bucket)
- ✅ Backup & restore capabilities

#### Go Control Plane
- ✅ JWT authentication client
- ✅ Role-based access control (RBAC)
- ✅ Policy engine with 5 default policies
- ✅ Comprehensive audit logging
- ✅ Prometheus metrics integration
- ✅ OpenTelemetry tracing (Jaeger)
- ✅ Health checks and cluster status
- ✅ RESTful management API (12 endpoints)

#### Quality & Testing
- ✅ 176+ tests passing (98.9% pass rate)
- ✅ 17 performance benchmarks
- ✅ Comprehensive documentation
- ✅ Integration test suite

**Technical Debt**: Some modules lack clean architecture boundaries (to be addressed in v1.1)

---

## 🔄 Phase 1.5: Architectural Refactoring (v1.1-1.2) - **HIGH PRIORITY**

### Timeline: Q1 2026 (10-12 weeks total, includes production-readiness enhancements)

---

### 🤖 IMMEDIATE: MCP Server AI-Native Enhancements (2-3 weeks)
**Priority**: CRITICAL (Can run in parallel with Rust refactoring)
**Inspiration**: Agentic Postgres's embedded expertise approach
**Dependencies**: None

**Goal**: Transform MCP server from basic tool provider to AI-native interface with embedded event sourcing expertise

#### Tasks:

**1. Embedded Expertise in Tool Descriptions (1 week)**
```typescript
// packages/mcp-server/src/tools/enhanced-tools.ts

// Enhance each tool with:
// - Agent guidance (best practices, common patterns)
// - Performance tips (when to use limit, indexing hints)
// - Use case examples (audit, analytics, debugging)
// - Decision trees (which tool for which scenario)

const enhancedTools: Tool[] = [
  {
    name: 'query_events',
    description: `Query events with flexible filters.

    💡 AGENT GUIDANCE:
    - entity_id: Track specific user/order/resource lifecycle
    - event_type: Analyze behavior patterns (e.g., "user.created")
    - Temporal analysis: Combine 'since' with pattern detection
    - Time-travel: Use 'as_of' for historical state reconstruction

    🎯 COMMON PATTERNS:
    - User journey: entity_id + time range → lifecycle analysis
    - System health: event_type + frequency → detect anomalies
    - Compliance: 'as_of' + entity_id → point-in-time audit

    ⚠️ PERFORMANCE:
    - Add 'limit' for exploration (avoids full scans)
    - Use 'until' to bound queries (faster execution)
    - event_type filters use indexes (prefer over payload filters)`,
    // ... schema
  }
];
```

**2. Agent Advisory Tool (1 week)**
```typescript
// New tool: get_query_advice
{
  name: 'get_query_advice',
  description: 'Get expert advice on event sourcing queries. Embeds 10+ years of expertise.',
  inputSchema: {
    use_case: {
      enum: [
        'audit_trail',        // Compliance/governance queries
        'user_analytics',     // Behavioral analysis
        'debugging',          // Root cause analysis
        'compliance',         // Regulatory reporting
        'performance_analysis' // System health monitoring
      ]
    },
    context: { type: 'string' }  // Additional context
  }
}

// Implementation provides:
// - Recommended tool combinations
// - Query patterns for use case
// - Performance optimization tips
// - Common pitfalls to avoid
```

**3. Multi-Turn Conversational Context (1 week)**
```typescript
// packages/mcp-server/src/context/conversation-context.ts

class ConversationContext {
  private sessions: Map<string, QuerySession>;

  // Enable iterative refinement:
  // Agent: "Show users created yesterday"
  // Agent: "Filter to premium tier"     // Understands context
  // Agent: "Compare with last month"    // Builds on previous

  buildQuery(input: string, sessionId: string): EnhancedQuery {
    const session = this.sessions.get(sessionId);
    // Merge with previous query context
    // Intelligent query composition
  }
}
```

**4. Quick Exploration Tools (1 week)**
```typescript
// Fast sampling for agent exploration
{
  name: 'sample_events',
  description: 'Get representative sample (not exhaustive). Fast exploration of unknown data.',
  inputSchema: {
    sample_size: { type: 'number', default: 1000 },
    stratified_by: { enum: ['event_type', 'entity_id', 'time'] }
  }
},

{
  name: 'quick_stats',
  description: 'Fast approximate statistics. Trades precision for speed.',
  inputSchema: {
    metric: { enum: ['event_count', 'unique_entities', 'event_types', 'time_range'] }
  }
}
```

**Deliverables**:
- [ ] Enhanced tool descriptions with embedded guidance (200 LOC)
- [ ] `get_query_advice` tool implementation (300 LOC)
- [ ] Conversational context manager (400 LOC)
- [ ] Quick exploration tools (300 LOC)
- [ ] Updated documentation with agent usage patterns (20 pages)

**Benefits**:
- ✅ Agents can query autonomously without human guidance
- ✅ Embedded expertise reduces trial-and-error
- ✅ Multi-turn conversations enable iterative refinement
- ✅ Fast sampling enables rapid exploration
- ✅ Zero breaking changes to existing MCP tools

---

### 🏗️ v1.1: Production-Ready Foundation (6-7 weeks)

**Goal**: Refactor to Clean Architecture + Add critical production-readiness features inspired by SierraDB

**Inspiration**: SierraDB's hard-learned lessons about partition architecture, version guarantees, and stress testing

#### Rust Core Refactoring (4-6 weeks)
**Priority**: CRITICAL
**Dependencies**: None

**Current Issues**:
- Some modules mix business logic with infrastructure
- Tight coupling between storage and domain logic
- Direct dependencies on concrete implementations
- ⚠️ **No partition-based architecture** (needed for scaling)
- ⚠️ **No gapless version guarantees** (SierraDB's "version guarantee problem")
- ⚠️ **No long-running stress tests** (SierraDB found corruption via this)

**Refactoring Tasks**:

1. **Domain Layer (Innermost)**

   **🆕 SierraDB-Inspired Additions**:
   - Partition-based architecture for scaling
   - Gapless version guarantees (watermark system)
   - Event stream aggregates with optimistic locking

   **🤖 Agentic Postgres-Inspired Additions**:
   - Copy-on-write event store forks for agent experimentation
   - Fork lifecycle management (TTL, cleanup)
   - Isolation guarantees for sandboxed queries

   ```rust
   // src/domain/
   ├── entities/
   │   ├── event.rs          // Core Event entity (no external deps)
   │   ├── tenant.rs         // Tenant entity
   │   ├── user.rs           // User entity
   │   ├── snapshot.rs       // Snapshot entity
   │   ├── event_stream.rs   // 🆕 Event stream aggregate (SierraDB pattern)
   │   └── event_store_fork.rs // 🤖 Fork entity (Agentic Postgres pattern)
   ├── value_objects/
   │   ├── event_id.rs       // Strongly-typed IDs
   │   ├── timestamp.rs      // Time value objects
   │   ├── tenant_id.rs      // Tenant identifier
   │   ├── partition_key.rs  // 🆕 Partition key (32 partitions initially)
   │   ├── stream_version.rs // 🆕 Gapless version tracking
   │   └── fork_id.rs        // 🤖 Fork identifier
   ├── aggregates/
   │   ├── event_stream.rs   // Event stream aggregate with watermarks
   │   └── tenant_config.rs  // Tenant configuration
   └── repositories/         // Repository traits (abstractions)
       ├── event_repository.rs
       ├── tenant_repository.rs
       ├── snapshot_repository.rs
       └── fork_repository.rs // 🤖 Fork management
   ```

   **New Event Stream Aggregate** (SierraDB watermark pattern):
   ```rust
   // src/domain/aggregates/event_stream.rs
   pub struct EventStream {
       entity_id: EntityId,
       partition: PartitionKey,
       current_version: u64,
       watermark: u64,  // Highest continuously confirmed sequence
   }

   impl EventStream {
       /// Append event with optimistic locking
       /// Ensures gapless version numbers (SierraDB pattern)
       pub fn append_event(&mut self, expected_version: u64) -> Result<u64> {
           if expected_version != self.current_version + 1 {
               return Err(OptimisticLockError);
           }
           self.current_version += 1;
           Ok(self.current_version)
       }

       /// Update watermark (highest continuous sequence)
       pub fn update_watermark(&mut self, confirmed_version: u64) {
           if confirmed_version == self.watermark + 1 {
               self.watermark = confirmed_version;
           }
       }
   }
   ```

   **🤖 New Event Store Fork Entity** (Agentic Postgres copy-on-write pattern):
   ```rust
   // src/domain/entities/event_store_fork.rs
   pub struct EventStoreFork {
       fork_id: ForkId,
       parent_store_id: StoreId,
       created_at: Timestamp,
       expires_at: Timestamp,  // TTL for auto-cleanup
       isolation_level: IsolationLevel,
       metadata: ForkMetadata,  // Purpose, created_by (agent), etc.
   }

   impl EventStoreFork {
       /// Create instant fork using copy-on-write
       /// Agents can test queries/projections without affecting production
       pub fn create_from_parent(
           parent: &EventStore,
           ttl_seconds: u64,
           metadata: ForkMetadata
       ) -> Result<EventStoreFork> {
           Ok(EventStoreFork {
               fork_id: ForkId::new(),
               parent_store_id: parent.id.clone(),
               created_at: Timestamp::now(),
               expires_at: Timestamp::now().add_seconds(ttl_seconds),
               isolation_level: IsolationLevel::Snapshot,
               metadata,
           })
       }

       /// Check if fork has expired (for auto-cleanup)
       pub fn is_expired(&self) -> bool {
           Timestamp::now() > self.expires_at
       }

       /// Cleanup expired forks (background task)
       pub async fn cleanup_expired_forks(
           fork_repo: &dyn ForkRepository
       ) -> Result<Vec<ForkId>> {
           fork_repo.delete_expired_forks().await
       }
   }
   ```

2. **Application Layer (Use Cases)**
   ```rust
   // src/application/
   ├── use_cases/
   │   ├── ingest_event.rs        // Single use case per file
   │   ├── query_events.rs
   │   ├── create_snapshot.rs
   │   ├── replay_events.rs
   │   ├── manage_tenant.rs
   │   ├── create_fork.rs         // 🤖 Create event store fork
   │   ├── query_fork.rs          // 🤖 Query within fork context
   │   └── cleanup_forks.rs       // 🤖 TTL-based fork cleanup
   ├── services/
   │   ├── event_service.rs       // Application service
   │   ├── projection_service.rs
   │   ├── analytics_service.rs
   │   └── fork_service.rs        // 🤖 Fork lifecycle management
   └── dto/                       // Data Transfer Objects
       ├── event_dto.rs
       ├── query_dto.rs
       └── fork_dto.rs            // 🤖 Fork metadata
   ```

3. **Infrastructure Layer (Outermost)**
   ```rust
   // src/infrastructure/
   ├── persistence/
   │   ├── parquet_event_repository.rs   // Concrete implementation
   │   ├── wal_event_repository.rs
   │   └── postgres_tenant_repository.rs
   ├── web/
   │   ├── handlers/                     // HTTP handlers
   │   ├── middleware/
   │   └── routes.rs
   ├── messaging/
   │   ├── websocket_publisher.rs
   │   └── kafka_publisher.rs (future)
   └── cache/
       └── redis_cache.rs (future)
   ```

4. **Dependency Injection Setup**
   ```rust
   // src/lib.rs
   pub struct AppContainer {
       event_repository: Arc<dyn EventRepository>,
       tenant_repository: Arc<dyn TenantRepository>,
       event_service: Arc<EventService>,
       // ... other dependencies
   }

   impl AppContainer {
       pub fn new(config: Config) -> Self {
           // Wire up dependencies here
           let event_repo = Arc::new(ParquetEventRepository::new(config));
           let event_service = Arc::new(EventService::new(event_repo.clone()));
           // ...
       }
   }
   ```

**Benefits**:
- ✅ Testable in isolation (mock dependencies)
- ✅ Swap implementations easily (e.g., Parquet → S3)
- ✅ Business logic independent of frameworks
- ✅ Clear dependency direction (inward)
- 🆕 Partition-based scaling ready (SierraDB pattern)
- 🆕 Gapless version guarantees (prevents inconsistencies)
- 🆕 Foundation for simplified clustering (no complex coordination)

**Performance Impact**: Negligible (<1% overhead from trait dispatch)

---

#### 🆕 Production Readiness Enhancements (2-3 weeks)
**Priority**: CRITICAL
**Dependencies**: None (can run in parallel with refactoring)
**Inspiration**: SierraDB's lessons from production corruption and stress testing

**1. Long-Running Stress Tests** (1 week)
```rust
// benches/stress_tests.rs
#[bench]
fn stress_test_7_day_continuous_ingestion() {
    // Target: 7 days continuous at 469K events/sec
    // = ~285 billion events
    // Detect: memory leaks, corruption, performance degradation
}

#[bench]
fn stress_test_partition_load_balancing() {
    // Ensure 32 partitions distribute evenly
    // Detect: hot partitions, skewed distribution
}

#[bench]
fn stress_test_concurrent_tenants() {
    // 1000 tenants, mixed workload
    // Detect: tenant isolation issues, cascading failures
}
```

**2. Storage Integrity Checks** (1 week)
```rust
// src/infrastructure/storage.rs
impl ParquetStorage {
    /// Add checksums to Parquet files (SierraDB learned this the hard way)
    pub fn write_with_checksum(&self, events: &[Event]) -> Result<()> {
        let data = serialize_events(events)?;
        let checksum = crc32::hash(&data);

        // Write data + checksum
        self.write_parquet(&data)?;
        self.write_checksum_file(checksum)?;
        Ok(())
    }

    /// Verify integrity on startup (catch corruption early)
    pub fn verify_integrity_on_startup(&self) -> Result<Vec<CorruptionReport>> {
        let mut corrupted_files = Vec::new();

        for file in self.list_parquet_files()? {
            if !self.verify_checksum(&file)? {
                corrupted_files.push(CorruptionReport {
                    file: file.clone(),
                    error: "Checksum mismatch".into(),
                });
            }
        }

        Ok(corrupted_files)
    }
}

// src/infrastructure/wal.rs
impl WriteAheadLog {
    /// WAL integrity verification
    pub fn verify_wal_integrity(&self) -> Result<()> {
        // Check for:
        // - Missing segments
        // - Corrupted entries
        // - Incomplete writes
    }
}
```

**3. Partition Monitoring** (1 week)
```rust
// src/infrastructure/metrics.rs
pub struct PartitionMetrics {
    /// Track per-partition health (SierraDB pattern)
    partition_event_counts: HashMap<PartitionKey, AtomicU64>,
    partition_write_latencies: HashMap<PartitionKey, Histogram>,
    partition_error_rates: HashMap<PartitionKey, AtomicU64>,
}

impl PartitionMetrics {
    /// Detect hot partitions or skew
    pub fn detect_partition_imbalance(&self) -> Vec<PartitionAlert> {
        // Alert if any partition has >2x average load
    }
}
```

**Deliverables**:
- [ ] 7-day stress test suite (500 LOC)
- [ ] Storage checksum system (300 LOC)
- [ ] WAL integrity verification (200 LOC)
- [ ] Partition monitoring (400 LOC)
- [ ] Corruption detection on startup (200 LOC)
- [ ] Automated stress test CI job

**Time Saved Later**: Prevents production corruption issues (SierraDB's painful lesson)

---

#### Go Control Plane Refactoring (3-4 weeks)
**Priority**: HIGH
**Dependencies**: None

**Current Issues**:
- All logic in main file and flat structure
- No clear separation of concerns
- Direct dependencies on Gin framework

**Refactoring Tasks**:

1. **Domain Layer**
   ```go
   // internal/domain/
   ├── entities/
   │   ├── user.go
   │   ├── tenant.go
   │   └── audit_event.go
   ├── repositories/         // Interfaces only
   │   ├── user_repository.go
   │   ├── tenant_repository.go
   │   └── audit_repository.go
   └── services/             // Domain services
       └── policy_service.go
   ```

2. **Application Layer**
   ```go
   // internal/application/
   ├── usecases/
   │   ├── authenticate_user.go
   │   ├── authorize_request.go
   │   ├── manage_tenant.go
   │   └── audit_operation.go
   ├── dto/
   │   ├── auth_dto.go
   │   └── tenant_dto.go
   └── ports/               // Input/output ports (interfaces)
       ├── auth_port.go
       └── audit_port.go
   ```

3. **Infrastructure Layer**
   ```go
   // internal/infrastructure/
   ├── persistence/
   │   ├── file_audit_repository.go      // File-based audit
   │   ├── postgres_user_repository.go   // Future
   │   └── redis_cache_repository.go     // Future
   ├── web/
   │   ├── handlers/
   │   │   ├── auth_handler.go
   │   │   ├── tenant_handler.go
   │   │   └── operations_handler.go
   │   ├── middleware/
   │   │   ├── auth_middleware.go
   │   │   └── tracing_middleware.go
   │   └── router.go
   ├── clients/
   │   ├── rust_core_client.go
   │   └── jaeger_client.go
   └── config/
       └── config.go
   ```

4. **Dependency Injection (Wire)**
   ```go
   // cmd/control-plane/main.go
   package main

   import (
       "github.com/google/wire"
       "allsource/internal/infrastructure"
       "allsource/internal/application"
   )

   // wire.go (generated by Wire)
   func InitializeApp(config Config) (*App, error) {
       wire.Build(
           infrastructure.NewUserRepository,
           infrastructure.NewAuditRepository,
           application.NewAuthUseCase,
           application.NewTenantUseCase,
           NewApp,
       )
       return &App{}, nil
   }
   ```

**Benefits**:
- ✅ Framework-independent business logic
- ✅ Easy to test (mock interfaces)
- ✅ Swap web framework (Gin → Fiber)
- ✅ Clear dependency flow

**Performance Impact**: Minimal (<0.5ms per request)

---

#### Clojure Services Architecture (Initial Setup, 2-3 weeks)
**Priority**: MEDIUM
**Dependencies**: None

**Structure** (Component + Mount):

```clojure
;; src/allsource/
├── domain/
│   ├── entities/
│   │   ├── event.clj
│   │   └── tenant.clj
│   ├── protocols/          ;; Interfaces (like Java interfaces)
│   │   ├── event_repository.clj
│   │   └── query_engine.clj
│   └── services/
│       └── query_service.clj
├── application/
│   ├── use_cases/
│   │   ├── execute_query.clj
│   │   └── build_projection.clj
│   └── handlers/           ;; Ring handlers
│       ├── query_handler.clj
│       └── projection_handler.clj
├── infrastructure/
│   ├── adapters/
│   │   ├── http_client.clj      ;; Rust core client
│   │   └── postgres_repo.clj    ;; Repository implementation
│   ├── web/
│   │   ├── routes.clj
│   │   └── middleware.clj
│   └── config/
│       └── system.clj             ;; Component/Mount system
└── utils/
    └── logging.clj
```

**Dependency Management** (Component pattern):
```clojure
(defrecord QueryService [event-repository config]
  component/Lifecycle
  (start [this]
    (assoc this :query-engine (create-engine config)))
  (stop [this]
    (dissoc this :query-engine)))

(defn new-query-service [config]
  (map->QueryService {:config config}))

;; System composition
(defn system [config]
  (component/system-map
    :config config
    :event-repository (new-event-repository config)
    :query-service (component/using
                     (new-query-service config)
                     [:event-repository :config])))
```

**Benefits**:
- ✅ Functional, immutable architecture
- ✅ Easy REPL-driven development
- ✅ Test each component in isolation
- ✅ Clear dependency graph
- ✅ Hot-reloadable system

---

### 🎯 v1.2: Performance Optimization, Search & Protocol Expansion

**Goal**: Optimize critical paths + Add native search capabilities (Agentic Postgres) + Add Redis protocol compatibility (SierraDB lesson)

---

#### 🤖 Native Search Capabilities (3-4 weeks)
**Priority**: HIGH
**Inspiration**: Agentic Postgres's integrated vector + keyword search
**Dependencies**: Rust Core v1.1 (Clean Architecture)

**Rationale** (from Agentic Postgres):
> "pgvectorscale + pg_textsearch provide vector and keyword search, eliminating external dependencies. Every language with a Redis client works immediately."

**Our Implementation**: Native Rust search eliminating need for Elasticsearch/external services

**1. Vector Search for Semantic Queries (2 weeks)**
```rust
// src/infrastructure/search/vector_search.rs
use fastembed::{EmbeddingModel, InitOptions};

pub struct VectorSearchEngine {
    model: EmbeddingModel,
    index: HnswIndex,  // Hierarchical Navigable Small World graph
}

impl VectorSearchEngine {
    /// Generate embedding for event payload
    pub fn embed_event(&self, event: &Event) -> Result<Vec<f32>> {
        let text = self.serialize_for_embedding(event);
        self.model.embed(vec![text], None)
            .map(|embeddings| embeddings[0].clone())
    }

    /// Semantic search: find events by meaning
    pub async fn search_similar(
        &self,
        query: &str,
        limit: usize,
        threshold: f32
    ) -> Result<Vec<(EventId, f32)>> {
        let query_embedding = self.model.embed(vec![query.to_string()], None)?;
        self.index.search(&query_embedding[0], limit, threshold)
    }

    /// Serialize event for embedding
    fn serialize_for_embedding(&self, event: &Event) -> String {
        format!(
            "{} {} {}",
            event.event_type,
            serde_json::to_string(&event.payload).unwrap_or_default(),
            event.metadata.description.unwrap_or_default()
        )
    }
}
```

**2. BM25 Keyword Search (1 week)**
```rust
// src/infrastructure/search/keyword_search.rs
use tantivy::{Index, IndexWriter, Document};

pub struct KeywordSearchEngine {
    index: Index,
    writer: IndexWriter,
}

impl KeywordSearchEngine {
    /// Index event payload for full-text search
    pub async fn index_event(&mut self, event: &Event) -> Result<()> {
        let mut doc = Document::new();
        doc.add_text(self.event_type_field, &event.event_type);
        doc.add_text(self.payload_field, &serde_json::to_string(&event.payload)?);
        doc.add_text(self.entity_id_field, &event.entity_id.to_string());

        self.writer.add_document(doc)?;
        Ok(())
    }

    /// BM25 keyword search
    pub async fn search_keywords(
        &self,
        query: &str,
        limit: usize
    ) -> Result<Vec<EventId>> {
        let searcher = self.index.reader()?.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![
            self.event_type_field,
            self.payload_field,
        ]);

        let query = query_parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        Ok(top_docs.into_iter()
            .map(|(_, doc_address)| self.extract_event_id(doc_address))
            .collect())
    }
}
```

**3. Hybrid Search (Semantic + Keyword + Metadata) (1 week)**
```rust
// src/infrastructure/search/hybrid_search.rs

pub struct HybridSearchEngine {
    vector_search: VectorSearchEngine,
    keyword_search: KeywordSearchEngine,
}

impl HybridSearchEngine {
    /// Combine semantic, keyword, and metadata filtering
    pub async fn search(
        &self,
        query: SearchQuery
    ) -> Result<Vec<ScoredEvent>> {
        let mut results = Vec::new();

        // 1. Semantic search (if natural language query)
        if let Some(semantic_query) = query.semantic {
            let semantic_results = self.vector_search
                .search_similar(&semantic_query, query.limit * 2, 0.7).await?;
            results.extend(semantic_results);
        }

        // 2. Keyword search (if keyword query)
        if let Some(keyword_query) = query.keywords {
            let keyword_results = self.keyword_search
                .search_keywords(&keyword_query, query.limit * 2).await?;
            results.extend(keyword_results);
        }

        // 3. Combine and re-rank
        let combined = self.combine_scores(results);

        // 4. Apply metadata filters (event_type, entity_id, time range)
        let filtered = self.apply_filters(combined, query.filters);

        // 5. Return top-k
        Ok(filtered.into_iter().take(query.limit).collect())
    }
}
```

**4. MCP Integration**
```typescript
// packages/mcp-server/src/tools/search-tools.ts

{
  name: 'semantic_search_events',
  description: `Search events by meaning, not exact matches. Uses vector embeddings.

  💡 AGENT GUIDANCE:
  - Use for: "Find events related to user complaints"
  - Use for: "What events are similar to order cancellations"
  - Faster than scanning all events manually

  🎯 BEST FOR:
  - Exploratory analysis of unfamiliar data
  - Finding conceptually similar events
  - Cross-event-type pattern detection`,

  inputSchema: {
    query: { type: 'string', description: 'Natural language query' },
    limit: { type: 'number', default: 100 },
    threshold: { type: 'number', default: 0.7, description: 'Similarity threshold (0-1)' }
  }
},

{
  name: 'hybrid_search',
  description: 'Combine semantic + keyword + metadata filters for precise results.',
  inputSchema: {
    semantic_query: { type: 'string' },
    keywords: { type: 'string' },
    filters: { type: 'object' }  // event_type, entity_id, time_range
  }
}
```

**Technical Requirements**:
- `fastembed` or `candle` for embeddings (pure Rust, no Python)
- `tantivy` for BM25 full-text search (Rust Lucene alternative)
- `usearch` or custom HNSW for vector index
- Background indexing (async, non-blocking)
- Incremental index updates

**Deliverables**:
- [ ] Vector search engine (600 LOC)
- [ ] BM25 keyword search (400 LOC)
- [ ] Hybrid search orchestrator (300 LOC)
- [ ] Background indexing service (400 LOC)
- [ ] MCP search tools (200 LOC)
- [ ] Search benchmarks (100K+ events)
- [ ] 30+ tests (search accuracy, performance)

**Benefits**:
- ✅ **No external dependencies** (Elasticsearch, Algolia, etc.)
- ✅ **Agent-friendly**: natural language queries
- ✅ **Unified stack**: all in Rust
- ✅ **Fast**: in-memory index, native performance
- ✅ **Semantic understanding**: find events by meaning

---

#### 🆕 Redis Protocol Compatibility (OPTIONAL, 2-3 weeks)
**Priority**: MEDIUM
**Inspiration**: SierraDB's RESP3 adoption for instant multi-language support
**Dependencies**: None (can run in parallel with performance work)

**Rationale** (from SierraDB):
> "Rather than building custom drivers, we adopted Redis' RESP3 protocol because every language with a Redis client works immediately. This enables ecosystem compatibility and allows debugging via redis-cli."

**Implementation**:
```rust
// src/infrastructure/redis_api.rs
use redis_protocol::resp3;

pub struct RespServer {
    tcp_listener: TcpListener,
    event_service: Arc<EventService>,
}

impl RespServer {
    /// Handle RESP3 commands
    async fn handle_command(&self, cmd: RespCommand) -> RespResponse {
        match cmd {
            // Event ingestion: XADD stream-name * field value...
            RespCommand::XAdd { stream, fields } => {
                let event = Event::from_redis_fields(fields)?;
                self.event_service.ingest(event).await?;
                RespResponse::BulkString(event_id)
            }

            // Event query: XRANGE stream-name start end
            RespCommand::XRange { stream, start, end } => {
                let events = self.event_service
                    .query_range(stream, start, end).await?;
                RespResponse::Array(events.into_resp3())
            }

            // Subscribe to stream: SUBSCRIBE stream-name
            RespCommand::Subscribe { channels } => {
                // Real-time event streaming
                self.event_service.subscribe(channels).await?;
                RespResponse::Subscribe(channels)
            }

            _ => RespResponse::Error("Unknown command".into())
        }
    }
}
```

**Benefits**:
- ✅ **Instant multi-language support** (every Redis client works)
- ✅ **Zero driver development** (piggyback on Redis ecosystem)
- ✅ **Debug with redis-cli** (operational simplicity)
- ✅ **Familiar API** (developers already know Redis commands)
- ✅ **Performance** (RESP3 is binary and fast)

**Trade-offs**:
- Additional protocol to maintain (but redis-protocol crate handles parsing)
- Some event store features may not map perfectly to Redis commands
- Optional feature (HTTP API remains primary)

**Deliverables**:
- [ ] RESP3 server implementation (400 LOC)
- [ ] Redis command mapping (300 LOC)
- [ ] Integration tests with redis-cli (200 LOC)
- [ ] Documentation for Redis clients (20 pages)

**Recommendation**: Implement as **optional feature** after core refactoring

---

#### Rust Performance Optimizations (4-5 weeks)
**Priority**: HIGH
**Target**: 1M+ events/sec (from 469K)

**Optimization Areas**:

1. **Zero-Copy Deserialization**
   ```rust
   // Before: Copying bytes
   let event: Event = serde_json::from_slice(&bytes)?;

   // After: Zero-copy with simd-json
   let mut bytes_mut = bytes.to_vec();
   let event: Event = simd_json::from_slice(&mut bytes_mut)?;

   // Performance: +20% throughput
   ```

2. **Lock-Free Data Structures**
   ```rust
   // Replace Arc<Mutex<T>> with lock-free alternatives
   use crossbeam::queue::ArrayQueue;
   use dashmap::DashMap; // Lock-free HashMap

   // Before
   let index: Arc<Mutex<HashMap<String, Vec<EventId>>>> = ...;

   // After
   let index: Arc<DashMap<String, Vec<EventId>>> = ...;

   // Performance: +30% on concurrent writes
   ```

3. **Batch Processing**
   ```rust
   // Batch events for Parquet writes
   const BATCH_SIZE: usize = 10_000;

   impl EventRepository {
       async fn batch_write(&self, events: Vec<Event>) -> Result<()> {
           for chunk in events.chunks(BATCH_SIZE) {
               self.write_parquet_batch(chunk).await?;
           }
           Ok(())
       }
   }

   // Performance: +40% write throughput
   ```

4. **Memory Pool for Allocations**
   ```rust
   use bumpalo::Bump;

   thread_local! {
       static EVENT_POOL: Bump = Bump::new();
   }

   // Reuse allocations within request
   // Performance: -50% allocations, +15% throughput
   ```

5. **SIMD for Event Processing**
   ```rust
   #[cfg(target_arch = "x86_64")]
   use std::arch::x86_64::*;

   // Vectorized event filtering
   fn filter_events_simd(events: &[Event], predicate: &Predicate) -> Vec<&Event> {
       // SIMD implementation for common predicates
       // Performance: +2-3x for filtering operations
   }
   ```

**Target Performance**:
- Ingestion: **1M+ events/sec** (current: 469K)
- Query latency: **<5μs p99** (current: 11.9μs)
- Memory: **<2GB for 100M events** (current: ~3GB)

---

#### Go Control Plane Optimizations (2-3 weeks)
**Priority**: MEDIUM
**Target**: <5ms p99 latency

**Optimization Areas**:

1. **Connection Pooling**
   ```go
   // Reuse HTTP connections to Rust core
   client := &http.Client{
       Transport: &http.Transport{
           MaxIdleConns:        100,
           MaxIdleConnsPerHost: 100,
           IdleConnTimeout:     90 * time.Second,
       },
       Timeout: 5 * time.Second,
   }
   ```

2. **Response Caching**
   ```go
   // Cache frequent queries (cluster status, metrics)
   type CachedResponse struct {
       data      []byte
       expiresAt time.Time
   }

   cache := sync.Map{}  // Or use go-cache library
   ```

3. **Async Audit Logging**
   ```go
   // Non-blocking audit writes
   auditChan := make(chan AuditEvent, 10000)

   go func() {
       for event := range auditChan {
           logger.Log(event)  // Async
       }
   }()
   ```

**Target Performance**:
- Latency: **<5ms p99** (current: varies)
- Throughput: **10K+ req/sec** (current: 1K)
- Memory: **<100MB** (current: ~20MB)

---

#### Clojure Services Optimization (2-3 weeks)
**Priority**: MEDIUM (after initial implementation)

**Optimization Areas**:

1. **Transducers for Efficiency**
   ```clojure
   ;; Before: Multiple intermediate collections
   (->> events
        (filter event-predicate)
        (map transform-event)
        (take 100))

   ;; After: Single pass with transducers
   (into []
     (comp
       (filter event-predicate)
       (map transform-event)
       (take 100))
     events)

   ;; Performance: -80% memory allocations
   ```

2. **Reducers for Parallelism**
   ```clojure
   (require '[clojure.core.reducers :as r])

   ;; Parallel processing of large event sets
   (->> events
        (r/filter predicate)
        (r/map transform)
        (r/fold combiner))

   ;; Performance: Utilizes all CPU cores
   ```

3. **Persistent Data Structure Tuning**
   ```clojure
   ;; Use transients for building large collections
   (persistent!
     (reduce
       (fn [acc event]
         (assoc! acc (:id event) event))
       (transient {})
       events))

   ;; Performance: +50% faster than assoc
   ```

**Target Performance**:
- Query execution: **<100ms p99**
- Projection updates: **<10ms lag**
- Memory: **<500MB JVM heap**

---

## 🚀 Phase 2: Clojure Integration Layer (v1.3-1.7) - **PLANNED**

### Timeline: Q1-Q4 2026

---

### 🔷 v1.3: Query DSL + REPL (Q1 2026)

#### 1. Clojure Query DSL (4-6 weeks)
**Priority**: HIGH
**Dependencies**: Rust Core v1.1 (Clean Architecture)

**Features**:
- Declarative query syntax using Clojure data structures
- Temporal operators (at, between, since, until)
- Aggregation functions (count, sum, avg, group-by)
- Join operations across event streams
- Lazy evaluation for memory efficiency
- Query optimization and compilation

**Architecture** (Clean):
```clojure
;; Domain layer: Query language entities
(ns allsource.query.domain.query)

(defrecord Query [select from where order-by limit])
(defrecord Predicate [operator field value])
(defrecord Aggregation [function field alias])

;; Application layer: Query execution
(ns allsource.query.application.executor)

(defprotocol QueryExecutor
  (compile-query [this query])
  (execute-query [this compiled-query])
  (stream-results [this compiled-query]))

;; Infrastructure: HTTP client to Rust core
(ns allsource.query.infrastructure.client)

(defrecord RustCoreClient [base-url auth-token]
  QueryExecutor
  (execute-query [this compiled-query]
    (http/post (str base-url "/api/v1/query")
      {:body (json/encode compiled-query)
       :headers {"Authorization" (str "Bearer " auth-token)}})))
```

**Example Usage**:
```clojure
(require '[allsource.query.dsl :as q])

;; Simple query
(q/query
  {:select [:entity-id :event-type :timestamp :payload]
   :from :events
   :where [:and
           [:= :event-type "user.created"]
           [:> :timestamp (q/days-ago 7)]
           [:contains? :payload.tags "premium"]]
   :order-by [[:timestamp :desc]]
   :limit 100})

;; Complex aggregation
(q/query
  {:select [:event-type (q/count) (q/sum :payload.amount)]
   :from :events
   :where [:between :timestamp
           (q/days-ago 30)
           (q/now)]
   :group-by [:event-type]
   :having [:> (q/count) 100]})

;; Temporal query
(q/at-time (q/days-ago 7)
  (q/query
    {:select [:entity-id :state]
     :from :projections/user-state}))
```

**Query Optimizer**:
```clojure
(ns allsource.query.application.optimizer)

(defn optimize [query]
  (-> query
      (push-down-predicates)      ;; Push filters early
      (reorder-joins)              ;; Optimal join order
      (eliminate-redundant-sorts)  ;; Remove unnecessary sorts
      (use-indices)))              ;; Leverage indices
```

**Technical Requirements**:
- Component-based lifecycle management
- HTTP client with connection pooling
- Query validation with spec
- Error handling with Either monad (cats library)
- Metrics collection (dropwizard-metrics)

**Deliverables**:
- [ ] Query DSL library (1,500 LOC)
- [ ] Query compiler and optimizer (800 LOC)
- [ ] REST API for query execution (400 LOC)
- [ ] Query result streaming (300 LOC)
- [ ] Documentation and examples (comprehensive)
- [ ] 50+ unit tests (90% coverage)

**SOLID Compliance**:
- **SRP**: Query, Compiler, Executor are separate
- **OCP**: New operators via protocol extension
- **LSP**: Multiple executor implementations (HTTP, local)
- **ISP**: Small focused protocols (QueryExecutor, StreamProvider)
- **DIP**: Depend on QueryExecutor protocol, not concrete HTTP client

---

#### 2. Interactive REPL Environment (2-3 weeks)
**Priority**: HIGH
**Dependencies**: Query DSL

**Features**:
- nREPL server for remote development
- Pre-loaded event store client
- Helper functions for common operations
- Pretty printing for events and results
- History and autocomplete
- Namespace for query building
- Connection to live event stream

**REPL Setup**:
```clojure
;; dev/user.clj (development namespace)
(ns user
  (:require [allsource.query.dsl :as q]
            [allsource.repl.helpers :refer :all]
            [mount.core :as mount]))

;; Auto-start system on REPL load
(mount/start)

;; Pre-defined helpers
(defn recent [n]
  "Get n most recent events"
  (q/execute!
    (q/query {:from :events
              :order-by [[:timestamp :desc]]
              :limit n})))

(defn by-type [event-type]
  "Get events by type"
  (q/execute!
    (q/query {:from :events
              :where [:= :event-type event-type]})))

(defn user-events [user-id]
  "Get all events for a user"
  (q/execute!
    (q/query {:from :events
              :where [:= :entity-id user-id]})))

;; Pretty printing
(set! *print-length* 50)
(set! *print-level* 5)
```

**Example REPL Session**:
```clojure
user=> (require '[allsource.repl :refer :all])

user=> (recent 5)
;; Pretty-printed output
({:event-type "user.created"
  :entity-id "user-123"
  :timestamp #inst "2025-10-21T10:30:00Z"
  :payload {:name "John Doe" :email "john@example.com"}}
 ...)

user=> (def my-query
         (-> (q/from-events)
             (q/where [:= :event-type "order.placed"])
             (q/select [:entity-id :payload.amount])
             (q/limit 100)))

user=> (q/execute! my-query)
...

user=> (watch-events "order.placed")
;; Streams events in real-time
```

**Technical Requirements**:
- nREPL server with cider-nrepl middleware
- Custom pretty-printers (fipp library)
- REPL history (reply library)
- Hot-reloading (mount or component)

**Deliverables**:
- [ ] REPL server setup (200 LOC)
- [ ] Helper function library (400 LOC)
- [ ] Pretty-printer configurations (200 LOC)
- [ ] Developer documentation (30 pages)
- [ ] Example notebooks (10+)

---

### 🔷 v1.4: Projection Management (Q2 2026)

#### 3. Clojure Projection Service (6-8 weeks)
**Priority**: HIGH
**Dependencies**: Query DSL, Rust Core v1.1

**Features**:
- Define projections as pure Clojure functions
- Hot-reload projections without service restart
- Projection versioning and migration
- Incremental projection updates
- Projection state snapshots
- Error handling and retry logic
- Projection monitoring and metrics
- Multi-tenant projection isolation

**Architecture** (Clean):
```clojure
;; Domain: Projection entity and protocols
(ns allsource.projection.domain.projection)

(defprotocol Projection
  (project [this state event]
    "Apply event to current state, returns new state")
  (get-version [this]
    "Returns projection version")
  (get-name [this]
    "Returns projection name"))

(defrecord ProjectionDefinition [name version project-fn initial-state])

;; Application: Projection execution
(ns allsource.projection.application.executor)

(defprotocol ProjectionExecutor
  (start-projection [this projection-def])
  (stop-projection [this projection-name])
  (reload-projection [this projection-def])
  (get-state [this projection-name entity-id]))

;; Infrastructure: State persistence
(ns allsource.projection.infrastructure.state-store)

(defprotocol StateStore
  (save-state [this projection-name entity-id state])
  (load-state [this projection-name entity-id])
  (snapshot [this projection-name]))
```

**Example Projection**:
```clojure
(ns allsource.projections.user-statistics)

(defprojection user-stats
  "Maintain aggregate statistics for each user"
  {:version 2
   :source [:events]
   :initial-state {:order-count 0
                   :total-spent 0.0
                   :created-at nil}}

  (fn [state event]
    (case (:event-type event)
      "user.created"
      (assoc state
        :created-at (:timestamp event)
        :entity-id (:entity-id event))

      "order.placed"
      (-> state
          (update :order-count inc)
          (update :total-spent + (get-in event [:payload :amount])))

      "order.refunded"
      (-> state
          (update :order-count dec)
          (update :total-spent - (get-in event [:payload :amount])))

      state)))  ;; Unknown event types pass through

;; Deploy projection
(deploy-projection! user-stats)

;; Query projection state
(get-projection-state :user-statistics "user-123")
;; => {:created-at #inst "2025-10-21"
;;     :order-count 42
;;     :total-spent 12500.00}
```

**Hot-Reloading**:
```clojure
(ns allsource.projection.application.hot-reload)

(defn reload-projection! [projection-name]
  (let [new-def (load-projection-from-disk projection-name)]
    ;; Validate new version
    (validate-projection new-def)
    ;; Atomic swap
    (swap! projection-registry assoc projection-name new-def)
    ;; Log reload
    (log/info "Reloaded projection:" projection-name)))

;; Watch filesystem for changes
(watch-projection-directory
  (fn [changed-file]
    (when (projection-file? changed-file)
      (reload-projection! (parse-projection-name changed-file)))))
```

**Projection Migration**:
```clojure
(defn migrate-projection [old-version new-version]
  (case [old-version new-version]
    [1 2] (fn [old-state]
            (assoc old-state :email-verified false))
    [2 3] (fn [old-state]
            (-> old-state
                (rename-keys {:total-spent :lifetime-value})
                (assoc :tier (calculate-tier old-state))))
    (throw (ex-info "Unknown migration" {:from old-version :to new-version}))))
```

**Technical Requirements**:
- PostgreSQL for projection state (or Redis)
- Event subscription to Rust core
- Incremental catch-up on restart
- Distributed coordination (for multiple instances)
- Metrics (projection lag, throughput, errors)

**Deliverables**:
- [ ] Projection runtime engine (1,200 LOC)
- [ ] Projection DSL and macros (600 LOC)
- [ ] State management system (800 LOC)
- [ ] Projection deployment API (400 LOC)
- [ ] Monitoring dashboard (web UI)
- [ ] Migration tools (300 LOC)
- [ ] 40+ unit tests (85% coverage)

**SOLID Compliance**:
- **SRP**: Projection, StateStore, Executor are separate
- **OCP**: New projection types via protocol
- **LSP**: Multiple state store implementations (Postgres, Redis, In-memory)
- **ISP**: Focused protocols (Projection, StateStore, Executor)
- **DIP**: Depend on StateStore protocol, not concrete DB

---

### 🔷 v1.5: Event Processing Pipelines (Q2-Q3 2026)

#### 4. Event Processors & Transformations (8-10 weeks)
**Priority**: HIGH
**Dependencies**: Query DSL, Projection Service

**Features**:
- Composable event transformations
- Event enrichment from external sources
- Event filtering and routing
- Event aggregation windows
- Side-effect handling (notifications, webhooks)
- Dead-letter queue for failed events
- Pipeline observability and tracing
- Backpressure handling

**Architecture** (Clean + Functional):
```clojure
;; Domain: Pipeline operators (pure functions)
(ns allsource.pipeline.domain.operators)

(defn filter-events [predicate]
  (fn [event-stream]
    (filter predicate event-stream)))

(defn transform-events [transform-fn]
  (fn [event-stream]
    (map transform-fn event-stream)))

(defn enrich-with [enrichment-fn]
  (fn [event-stream]
    (map (fn [event]
           (merge event (enrichment-fn event)))
         event-stream)))

(defn window-by [duration field]
  (fn [event-stream]
    (partition-by
      (fn [event]
        (time/truncate (get event field) duration))
      event-stream)))

;; Application: Pipeline execution
(ns allsource.pipeline.application.executor)

(defprotocol PipelineExecutor
  (start-pipeline [this pipeline-def])
  (stop-pipeline [this pipeline-name])
  (get-metrics [this pipeline-name]))

;; Infrastructure: External integrations
(ns allsource.pipeline.infrastructure.enrichment)

(defprotocol EnrichmentSource
  (fetch-data [this key]))

(defrecord HttpEnrichmentSource [base-url auth-token]
  EnrichmentSource
  (fetch-data [this key]
    (http/get (str base-url "/api/" key)
      {:headers {"Authorization" (str "Bearer " auth-token)}})))
```

**Example Pipeline**:
```clojure
(ns allsource.pipelines.order-processing)

(defpipeline order-processing
  "Process and enrich order events"
  {:parallelism 4
   :buffer-size 1000
   :error-handling :retry-with-backoff}

  (-> events
      ;; Filter for order events
      (filter-events [:= :event-type "order.placed"])

      ;; Enrich with user data
      (enrich-with
        (fn [event]
          (let [user (fetch-user (:payload.user-id event))]
            {:user-details user})))

      ;; Calculate tax and shipping
      (transform-events
        (fn [event]
          (let [amount (get-in event [:payload :amount])
                tax (* amount 0.08)
                shipping (calculate-shipping event)]
            (-> event
                (assoc-in [:payload :tax] tax)
                (assoc-in [:payload :shipping] shipping)
                (assoc-in [:payload :total] (+ amount tax shipping))))))

      ;; Aggregate by hour
      (window-by :1-hour :timestamp)
      (aggregate-window
        (fn [events]
          {:hour (first-timestamp events)
           :order-count (count events)
           :total-revenue (sum-by [:payload :total] events)
           :avg-order-value (avg-by [:payload :total] events)}))

      ;; Emit to projection
      (sink-to! :hourly-revenue-projection)

      ;; Send notification for large orders
      (side-effect!
        (fn [aggregation]
          (when (> (:total-revenue aggregation) 10000)
            (send-slack-notification! :sales-channel aggregation))))))

;; Deploy pipeline
(deploy-pipeline! order-processing)
```

**Backpressure Handling**:
```clojure
(ns allsource.pipeline.application.backpressure)

(defn with-backpressure [pipeline buffer-size]
  (let [buffer (async/chan buffer-size)]
    (async/pipeline-blocking
      4  ;; Parallelism
      buffer
      (comp pipeline)
      input-chan)))
```

**Error Handling**:
```clojure
(ns allsource.pipeline.application.error-handling)

(defn with-retry [pipeline max-retries backoff-ms]
  (fn [event]
    (loop [attempts 0]
      (try
        (pipeline event)
        (catch Exception e
          (if (< attempts max-retries)
            (do
              (Thread/sleep (* backoff-ms (Math/pow 2 attempts)))
              (recur (inc attempts)))
            (send-to-dlq! event e)))))))
```

**Technical Requirements**:
- Core.async for concurrency
- Kafka/RabbitMQ integration (optional)
- Transducers for efficiency
- Circuit breakers (resilience4clj)
- Distributed tracing (OpenTelemetry)

**Deliverables**:
- [ ] Pipeline execution engine (1,500 LOC)
- [ ] Transformation library (20+ operators, 800 LOC)
- [ ] Windowing operators (400 LOC)
- [ ] Enrichment framework (500 LOC)
- [ ] Integration connectors (600 LOC)
- [ ] Pipeline deployment API (400 LOC)
- [ ] Error handling framework (400 LOC)
- [ ] 60+ unit tests (88% coverage)

**SOLID Compliance**:
- **SRP**: Each operator has one transformation responsibility
- **OCP**: New operators via higher-order functions
- **LSP**: All operators conform to same signature
- **ISP**: Small operator functions instead of monolithic pipeline
- **DIP**: Operators depend on data, not implementations

---

### 🔷 v1.6: Analytics Engine (Q3 2026)

#### 5. Analytics & Aggregations (6-8 weeks)
**Priority**: MEDIUM
**Dependencies**: Query DSL, Event Processors

**Features**:
- Time-series analytics
- Complex aggregations (nested group-by, pivots)
- Statistical functions (percentiles, stddev, correlation)
- Trend detection and forecasting
- Anomaly detection
- Custom metric definitions
- Real-time dashboards
- Export to analytics stores (ClickHouse, TimescaleDB)

**Architecture** (Clean):
```clojure
;; Domain: Analytics queries and functions
(ns allsource.analytics.domain.functions)

(defprotocol AggregationFunction
  (init-state [this])
  (accumulate [this state value])
  (finalize [this state]))

(defrecord CountAggregation []
  AggregationFunction
  (init-state [_] 0)
  (accumulate [_ state _] (inc state))
  (finalize [_ state] state))

(defrecord PercentileAggregation [p]
  AggregationFunction
  (init-state [_] [])
  (accumulate [_ state value] (conj state value))
  (finalize [_ state]
    (percentile (sort state) p)))

;; Application: Analytics executor
(ns allsource.analytics.application.executor)

(defprotocol AnalyticsExecutor
  (execute-time-series [this query])
  (execute-funnel [this funnel-def])
  (execute-cohort [this cohort-def]))

;; Infrastructure: Export adapters
(ns allsource.analytics.infrastructure.export)

(defprotocol ExportAdapter
  (export-results [this results format]))

(defrecord ClickHouseExporter [connection-pool]
  ExportAdapter
  (export-results [this results format]
    (jdbc/insert-multi! connection-pool :analytics results)))
```

**Example Analytics Queries**:
```clojure
;; Time-series aggregation
(analytics/time-series
  {:events (query-events {:event-type "order.placed"})
   :interval :1-hour
   :metrics {:order-count (count-events)
             :total-revenue (sum-field [:payload :amount])
             :avg-order-value (avg-field [:payload :amount])
             :unique-customers (count-distinct [:payload :user-id])
             :p95-order-value (percentile [:payload :amount] 0.95)}
   :group-by [[:payload :product-category]]
   :time-range (past-days 30)})

;; Funnel analysis
(analytics/funnel
  {:steps ["user.created" "order.placed" "payment.completed"]
   :group-by :entity-id
   :time-window :24-hours
   :start-date (days-ago 7)})
;; => {:step "user.created" :count 10000 :conversion-rate 1.0}
;;    {:step "order.placed" :count 6000 :conversion-rate 0.6}
;;    {:step "payment.completed" :count 5400 :conversion-rate 0.9}

;; Cohort analysis
(analytics/cohort
  {:cohort-field :created-at
   :cohort-interval :week
   :return-events ["order.placed"]
   :metrics {:retention-rate (retention-percentage)
             :avg-orders (avg-count)
             :cumulative-ltv (sum-field [:payload :amount])}
   :time-range (past-weeks 12)})

;; Trend detection
(analytics/detect-trends
  {:metric :total-revenue
   :interval :1-day
   :algorithm :linear-regression
   :confidence 0.95})

;; Anomaly detection
(analytics/detect-anomalies
  {:metric :order-count
   :interval :1-hour
   :algorithm :isolation-forest
   :sensitivity 0.8})
```

**Statistical Functions**:
```clojure
(ns allsource.analytics.domain.stats)

(defn percentile [sorted-values p]
  (let [n (count sorted-values)
        idx (* p (dec n))]
    (if (integer? idx)
      (nth sorted-values idx)
      (let [lower (nth sorted-values (int (Math/floor idx)))
            upper (nth sorted-values (int (Math/ceil idx)))]
        (/ (+ lower upper) 2.0)))))

(defn stddev [values]
  (let [n (count values)
        mean (/ (reduce + values) n)
        variance (/ (reduce + (map #(Math/pow (- % mean) 2) values)) n)]
    (Math/sqrt variance)))

(defn correlation [xs ys]
  (let [n (count xs)
        mean-x (/ (reduce + xs) n)
        mean-y (/ (reduce + ys) n)
        cov (/ (reduce + (map * (map #(- % mean-x) xs) (map #(- % mean-y) ys))) n)
        std-x (stddev xs)
        std-y (stddev ys)]
    (/ cov (* std-x std-y))))
```

**Technical Requirements**:
- Incanter or tech.ml for statistics
- Apache Arrow for efficient data transfer
- Time-series data structures (t-digest)
- Streaming aggregations (HyperLogLog, Count-Min Sketch)
- Materialized view management

**Deliverables**:
- [ ] Analytics query engine (1,000 LOC)
- [ ] Statistical functions library (30+ functions, 800 LOC)
- [ ] Time-series operators (500 LOC)
- [ ] Funnel/cohort analysis (600 LOC)
- [ ] Trend/anomaly detection (400 LOC)
- [ ] Visualization helpers (300 LOC)
- [ ] Export adapters (400 LOC)
- [ ] 45+ unit tests (87% coverage)

**SOLID Compliance**:
- **SRP**: Each statistical function is independent
- **OCP**: New functions via AggregationFunction protocol
- **LSP**: All aggregation functions interchangeable
- **ISP**: Focused protocols (AggregationFunction, ExportAdapter)
- **DIP**: Depend on ExportAdapter, not concrete ClickHouse

---

### 🔷 v1.7: Integration & Tools (Q4 2026)

#### Integration Tools (4-6 weeks)
**Priority**: MEDIUM
**Dependencies**: All previous Clojure features

**Features**:
- Event replay utilities with filtering
- State reconstruction tools
- Event migration scripts
- Data quality validation
- Backup and restore from Clojure
- Schema evolution helpers
- Multi-environment management
- Bulk import/export

**Example Tools**:
```clojure
;; Event replay with transformation
(replay/events
  {:source-store production
   :target-store staging
   :filter [:and
            [:> :timestamp (days-ago 30)]
            [:= :tenant-id "tenant-123"]]
   :transform (fn [event]
                (-> event
                    (anonymize-pii [:payload :email] [:payload :phone])
                    (update-schema-version 2)))
   :batch-size 10000})

;; Data quality validation
(validate/events
  {:store production
   :rules [(required-field? :entity-id)
           (valid-timestamp? :timestamp)
           (schema-valid? :payload)
           (no-duplicate-ids?)]
   :on-error :report
   :output "validation-report.edn"})

;; Schema migration
(migrate/schema
  {:event-type "user.created"
   :from-version 1
   :to-version 2
   :migration (fn [payload]
                (-> payload
                    (rename-keys {:name :full-name})
                    (assoc :email-verified false)
                    (update :created-at #(java.time.Instant/parse %))))
   :dry-run? false})

;; Bulk export to CSV
(export/to-csv
  {:query (q/query {:from :events
                    :where [:= :event-type "order.placed"]})
   :output "orders.csv"
   :columns [:entity-id :timestamp :payload.amount :payload.user-id]})
```

**CLI Tool**:
```bash
# Event replay
allsource replay --from prod --to staging --filter "timestamp > 2025-01-01"

# Validation
allsource validate --store prod --rules validation-rules.edn

# Schema migration
allsource migrate-schema --type user.created --from 1 --to 2

# Bulk export
allsource export --query "event-type = order.placed" --output orders.csv
```

**Deliverables**:
- [ ] Replay utilities (400 LOC)
- [ ] Validation framework (600 LOC)
- [ ] Migration tools (500 LOC)
- [ ] CLI tool for operations (800 LOC)
- [ ] Bulk import/export (400 LOC)
- [ ] Integration test suite (50+ tests)
- [ ] Operational runbooks (50 pages)

---

## 🏢 Phase 3: Enterprise Features (v1.8-2.0) - **FUTURE**

### Timeline: 2027

---

### 🔷 v1.8: Multi-Node & Distributed Coordination (Q1 2027)

**🆕 SIMPLIFIED APPROACH** (Inspired by SierraDB's term-based consensus)

**Timeline**: 5 weeks (vs 8-10 weeks for full Raft)
**Time Saved**: 3-5 weeks by avoiding full Raft complexity

**SierraDB Lesson**:
> "Rather than full Raft elections, we implemented term-based consensus inspired by Raft with deterministic leader selection based on cluster topology, reducing coordination overhead."

**Features**:
- Multi-node clustering (term-based consensus)
- **Deterministic leader selection** (no election overhead)
- Partition replication
- Cluster membership management
- **Manual failover** (v1.8, automatic in v1.9)
- Split-brain prevention

**Architecture** (Rust + SierraDB patterns):

**1. Term-Based Consensus** (Simpler than Raft):
```rust
// Domain: Cluster entity
pub struct ClusterNode {
    id: NodeId,
    address: SocketAddr,
    role: NodeRole,
    term: u64,           // Term number (monotonically increasing)
    health: NodeHealth,
}

pub enum NodeRole {
    Leader,
    Follower,
}

pub struct ClusterTerm {
    term_number: u64,
    leader_id: NodeId,
    followers: Vec<NodeId>,
}

// Application: Simplified cluster management
pub trait ClusterManager {
    async fn join_cluster(&self, node: ClusterNode) -> Result<()>;
    async fn leave_cluster(&self, node_id: NodeId) -> Result<()>;

    /// Deterministic leader selection (no voting)
    /// Leader = node with lowest ID in healthy set
    fn select_leader(&self, healthy_nodes: &[NodeId]) -> NodeId {
        healthy_nodes.iter().min().copied().expect("No healthy nodes")
    }

    /// Increment term on topology change
    async fn new_term(&self, leader_id: NodeId) -> Result<ClusterTerm>;
}
```

**2. Partition Replication** (Built on partition architecture from v1.1):
```rust
// Infrastructure: Replication using existing partitions
pub struct PartitionReplicator {
    partition_assignments: HashMap<PartitionKey, Vec<NodeId>>,
    replication_factor: usize,  // Default: 3
}

impl PartitionReplicator {
    /// Write to partition leader + replicas
    async fn replicate_write(&self, partition: PartitionKey, event: Event) -> Result<()> {
        let nodes = self.partition_assignments.get(&partition)?;
        let leader = nodes.first()?;

        // Write to leader
        self.write_to_node(leader, event.clone()).await?;

        // Async replication to followers
        for follower in &nodes[1..] {
            tokio::spawn(async move {
                self.write_to_node(follower, event.clone()).await
            });
        }

        Ok(())
    }
}
```

**3. Simplified Failover** (Manual in v1.8, automatic in v1.9):
```rust
// src/application/use_cases/manual_failover.rs
pub struct ManualFailoverUseCase {
    cluster_manager: Arc<dyn ClusterManager>,
}

impl ManualFailoverUseCase {
    /// Operator triggers failover (via CLI/API)
    pub async fn execute(&self, new_leader_id: NodeId) -> Result<()> {
        // 1. Verify new leader is healthy
        self.verify_node_health(new_leader_id).await?;

        // 2. Increment term
        let new_term = self.cluster_manager.new_term(new_leader_id).await?;

        // 3. Broadcast new topology
        self.broadcast_term_update(new_term).await?;

        Ok(())
    }
}
```

**Benefits of Simplified Approach**:
- ✅ **5 weeks vs 8-10 weeks** (3-5 weeks saved)
- ✅ **No election storms** (deterministic selection)
- ✅ **Simpler debugging** (fewer distributed edge cases)
- ✅ **Partition-based from v1.1** (foundation already exists)
- ✅ **Good enough for v1.8** (automatic failover in v1.9)

**Trade-offs**:
- ⚠️ Manual failover (vs automatic with Raft)
- ⚠️ Leader selection is deterministic (not load-aware)
- ✅ Acceptable for initial multi-node deployment

**Technical Requirements**:
- gRPC for inter-node communication
- Health check heartbeats (every 5s)
- Membership discovery (etcd or built-in)
- Partition assignment algorithm

**Deliverables**:
- [ ] Term-based consensus (800 LOC vs 2,000 for Raft)
- [ ] Deterministic leader selection (200 LOC)
- [ ] Partition replication (1,200 LOC)
- [ ] Manual failover API (400 LOC)
- [ ] Cluster membership (800 LOC)
- [ ] 25+ integration tests

**Future Enhancement** (v1.9):
- Automatic failover (add health-based triggering)
- Load-aware leader selection
- Automatic partition rebalancing

---

### 🔷 v1.9: Geo-Replication & Multi-Region (Q2 2027)

**Features**:
- Cross-region event replication
- Conflict resolution (CRDTs)
- Geo-aware routing
- Regional failover
- Global event ordering (hybrid logical clocks)

**Architecture**:
```rust
pub struct ReplicationController {
    local_region: RegionId,
    remote_regions: Vec<RemoteRegion>,
    conflict_resolver: Arc<dyn ConflictResolver>,
}

pub trait ConflictResolver {
    fn resolve(&self, local: Event, remote: Event) -> Event;
}

// Use hybrid logical clocks for global ordering
pub struct HybridClock {
    physical: SystemTime,
    logical: u64,
}
```

**Deliverables**:
- [ ] Cross-region replication (1,800 LOC)
- [ ] CRDT-based conflict resolution (1,000 LOC)
- [ ] Hybrid logical clock (300 LOC)
- [ ] Regional routing (600 LOC)
- [ ] 25+ tests

---

### 🔷 v2.0: Advanced Query & Stream Processing (Q3-Q4 2027)

**Features**:
- SQL-like query language (EventQL)
- GraphQL API
- Full-text search (Elasticsearch integration)
- Geospatial queries
- Exactly-once stream processing semantics
- Watermarks and late data handling
- Stateful stream processing

**Example EventQL**:
```sql
SELECT
    entity_id,
    COUNT(*) as order_count,
    SUM(payload->>'amount')::numeric as total_spent,
    DATE_TRUNC('day', timestamp) as day
FROM events
WHERE event_type = 'order.placed'
    AND timestamp >= NOW() - INTERVAL '30 days'
GROUP BY entity_id, DATE_TRUNC('day', timestamp)
HAVING COUNT(*) > 5
ORDER BY total_spent DESC
LIMIT 100;
```

**GraphQL API**:
```graphql
query {
  events(
    filter: {
      eventType: "order.placed"
      timestamp: { gte: "2025-01-01" }
    }
    orderBy: TIMESTAMP_DESC
    limit: 100
  ) {
    entityId
    eventType
    timestamp
    payload {
      amount
      userId
    }
  }

  projection(name: "user-statistics", entityId: "user-123") {
    orderCount
    totalSpent
    createdAt
  }
}
```

**Deliverables**:
- [ ] EventQL parser (1,500 LOC)
- [ ] Query planner (1,200 LOC)
- [ ] GraphQL schema and resolver (1,000 LOC)
- [ ] Elasticsearch integration (800 LOC)
- [ ] Stream processing engine (2,000 LOC)
- [ ] Watermark management (600 LOC)
- [ ] 50+ tests

---

## 📊 Success Metrics

### Performance Targets

| Metric | v1.0 (Current) | v1.2 (Target) | v2.0 (Goal) |
|--------|---------------|---------------|-------------|
| **Ingestion Throughput** | 469K events/sec | 1M events/sec | 5M events/sec |
| **Query Latency (p99)** | 11.9μs | <5μs | <1μs |
| **Concurrent Users** | 100+ | 1,000+ | 10,000+ |
| **Event Retention** | 5 years | 10 years | Unlimited |
| **Storage Efficiency** | 70% | 80% | 90% |
| **Projection Lag (p99)** | N/A | <100ms | <10ms |

### Quality Targets

| Metric | v1.0 (Current) | v1.5 (Target) | v2.0 (Goal) |
|--------|---------------|---------------|-------------|
| **Test Coverage** | 98.9% | 95%+ | 95%+ |
| **Uptime SLA** | 99.9% | 99.95% | 99.99% |
| **Zero Data Loss** | ✅ | ✅ | ✅ |
| **Mean Time to Recovery** | <5 min | <2 min | <30 sec |
| **Security Compliance** | JWT/RBAC | SOC 2 Type II | ISO 27001 |

### Adoption Targets

| Metric | 2026 Q1 | 2026 Q4 | 2027 Q4 |
|--------|---------|---------|---------|
| **GitHub Stars** | 100+ | 500+ | 2,000+ |
| **Production Deployments** | 10+ | 50+ | 200+ |
| **Community Contributors** | 5+ | 20+ | 100+ |
| **Active Integrations** | 5+ | 15+ | 50+ |
| **Documentation Pages** | 100+ | 300+ | 500+ |

---

## 🛠️ Technical Stack Summary

| Layer | Technology | Purpose | Clean Architecture |
|-------|-----------|---------|-------------------|
| **Core** | Rust | High-perf ingestion & storage | Domain → App → Infra |
| **Control Plane** | Go | Auth, ops, management | Domain → Use Cases → HTTP |
| **Processing** | Clojure | Queries, projections, analytics | Protocols → Services → Adapters |
| **Storage** | Parquet + WAL | Event persistence | Repository pattern |
| **🤖 AI Interface** | MCP Server (TypeScript) | Agent-native event store interface | MCP Protocol |
| **🤖 Vector Search** | Rust (fastembed + HNSW) | Semantic event search | Search adapter |
| **🤖 Keyword Search** | Rust (tantivy) | BM25 full-text search | Search adapter |
| **Caching** | Redis (optional) | Projection state, hot queries | Cache adapter |
| **Database** | PostgreSQL (optional) | Metadata, projections | Repository interface |
| **Metrics** | Prometheus | System monitoring | Metrics port |
| **Tracing** | Jaeger | Distributed tracing | Tracing middleware |
| **Messaging** | Kafka (future) | External integrations | Message adapter |

**Key Changes (Agentic Postgres inspiration)**:
- ✅ **Native search** (no Elasticsearch dependency)
- ✅ **MCP-first design** (AI agents are first-class citizens)
- ✅ **Unified Rust stack** (core + search integrated)

---

## 📅 Development Roadmap Timeline

**🆕 UPDATED** (with SierraDB + Agentic Postgres lessons integrated)

```
2025 Q4: v1.0 Complete ✅ (DONE)
2025 Q4: v1.3-v1.7 Complete ✅ (DONE - AHEAD OF SCHEDULE!)

2026 Q1-Q2: Phase 1.5 - AI-Native Foundation + Production Readiness (12-15 weeks)

            IMMEDIATE (Can run in parallel):
            - 🤖 MCP Server AI-Native Enhancements (2-3 weeks)
              • Embedded expertise in tool descriptions
              • Agent advisory tool (get_query_advice)
              • Multi-turn conversational context
              • Quick exploration tools (sampling, fast stats)

            v1.1 - Production-Ready Foundation (6-7 weeks):
            - Rust Core refactoring (4-6 weeks)
            - 🆕 Partition architecture (SierraDB - included in refactoring)
            - 🆕 Gapless version guarantees (SierraDB - included in refactoring)
            - 🤖 Copy-on-write event store forks (Agentic Postgres - included)
            - 🆕 Production stress tests (2-3 weeks)
            - Go Control Plane refactoring (3-4 weeks, parallel)

            v1.2 - Performance + Search + Protocol (7-9 weeks):
            - 🤖 Native search capabilities (Agentic Postgres - 3-4 weeks)
              • Vector search (semantic queries)
              • BM25 keyword search
              • Hybrid search orchestrator
            - Performance optimizations (4-5 weeks)
            - 🆕 OPTIONAL: Redis protocol (SierraDB - 2-3 weeks)

            Total: 12-15 weeks (includes AI-native + production features)

2026 Q2-Q4: Clojure Services + Documentation + Polish
            - Query DSL with agent-optimized syntax
            - Projection management with hot-reload
            - Event processing pipelines
            - Analytics engine

2027 Q1: v1.8 (Multi-Node - SIMPLIFIED with SierraDB patterns)
        - 🆕 5 weeks (vs 8-10 weeks) - SAVED 3-5 WEEKS
        - Term-based consensus (not full Raft)
        - Deterministic leader selection
        - Manual failover (automatic in v1.9)
        - Partition replication (leverages v1.1 partitions)

2027 Q2: v1.9 (Geo-Replication + Autonomous Operations)
        - Cross-region replication
        - Automatic failover (evolution from v1.8)
        - 🤖 Self-tuning queries (Agentic Postgres principles)
        - 🤖 Auto-scaling partitions

2027 Q3-Q4: v2.0 (Advanced Features)
        - EventQL (SQL-like query language)
        - GraphQL API
        - 🤖 Autonomous schema evolution
        - Advanced stream processing
```

**Time & Value Summary**:
- **Immediate MCP Enhancements**: +2-3 weeks → HIGH VALUE (agents autonomous)
- **v1.1 Fork Support**: +0 weeks (included in refactoring) → HIGH VALUE (agent experimentation)
- **v1.2 Native Search**: +3-4 weeks → HIGH VALUE (eliminates external dependencies)
- **v1.8 Simplified Consensus**: -3 to -5 weeks (vs full Raft) → TIME SAVED
- **Net: +2-5 weeks total, but with AI-native interface + production readiness**

**Key Advantages from Agentic Postgres Integration**:
✅ Agents can operate autonomously (embedded expertise)
✅ Instant experimentation (copy-on-write forks)
✅ Natural language search (vector + keyword)
✅ No external dependencies (unified Rust stack)
✅ Machine-speed operations (optimized for agent workflows)

---

## 🎓 SOLID Principles Application Summary

### Rust Core

**Single Responsibility**:
- `Event` struct: Only represents an event
- `EventRepository`: Only handles persistence
- `EventService`: Only coordinates use cases

**Open/Closed**:
- `trait EventRepository` allows new storage implementations
- `trait Middleware` for extensible request processing

**Liskov Substitution**:
- `ParquetEventRepository` and `WalEventRepository` interchangeable
- All repositories conform to same trait

**Interface Segregation**:
- Focused traits: `EventRepository`, `SnapshotRepository`, `TenantRepository`
- Not one large `Storage` trait

**Dependency Inversion**:
- `EventService` depends on `EventRepository` trait
- Infrastructure provides concrete implementations

### Go Control Plane

**Single Responsibility**:
- `AuthHandler`: Only auth endpoints
- `TenantHandler`: Only tenant endpoints
- `AuditLogger`: Only audit logging

**Open/Closed**:
- `interface UserRepository` allows new storage
- Middleware chain extensible

**Liskov Substitution**:
- `FileAuditRepository` and `PostgresAuditRepository` interchangeable

**Interface Segregation**:
- `AuthPort`, `AuditPort`, `TenantPort` instead of one large interface

**Dependency Inversion**:
- Use cases depend on port interfaces
- Infrastructure implements ports

### Clojure Services

**Single Responsibility**:
- Each projection has one purpose
- Each pipeline operator does one transformation

**Open/Closed**:
- Protocols allow new implementations
- Higher-order functions for extension

**Liskov Substitution**:
- All `StateStore` implementations interchangeable
- All `Projection` implementations conform to protocol

**Interface Segregation**:
- Small focused protocols (`QueryExecutor`, `StateStore`, `Projection`)

**Dependency Inversion**:
- Depend on protocols, not concrete records
- Use Component for dependency injection

---

## 🤝 Contributing

We welcome contributions! Areas organized by skill level:

### Beginner-Friendly
- Documentation improvements
- Example applications
- Bug reports with reproduction steps
- Test coverage improvements

### Intermediate
- Performance optimizations
- New query operators
- Projection templates
- Integration connectors
- Additional language clients

### Advanced
- Clean architecture refactoring
- Distributed systems features
- Query optimizer improvements
- Stream processing enhancements

---

## 📚 Resources

### Documentation
- [Clean Architecture Guide](./docs/CLEAN_ARCHITECTURE.md) (to be created)
- [SOLID Principles in Practice](./docs/SOLID_PRINCIPLES.md) (to be created)
- [Performance Optimization Guide](./docs/PERFORMANCE.md) (to be created)
- [API Reference](./docs/API.md)
- [Test Coverage Report](./UPDATED_TEST_COVERAGE_REPORT.md)

### Community
- GitHub Discussions (Q&A)
- Discord Server (coming soon)
- Monthly community calls (planned)
- Office hours (planned)

---

## 📄 License

MIT License - see LICENSE file for details

---

**Maintained by**: AllSource Core Team
**Status**: Active Development
**Next Milestone**: v1.1 - Clean Architecture Refactoring (Q1 2026)

---

*This comprehensive roadmap combines performance optimization, clean architecture principles, and feature development to create a world-class event store.*
