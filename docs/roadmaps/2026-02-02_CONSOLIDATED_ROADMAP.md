---
title: "AllSource Event Store - Consolidated Roadmap"
status: CURRENT
last_updated: 2026-02-02
category: roadmap
supersedes:
  - "2025-10-22_COMPREHENSIVE_ROADMAP.md"
  - "2025-10-24_ROADMAP_STATUS_ASSESSMENT.md"
  - "query-service-roadmap.md"
  - "mcp-v2-enhancements.md"
---

# AllSource Event Store - Consolidated Roadmap

**Date**: 2026-02-02
**Version**: 3.0
**Vision**: A high-performance, AI-native event store combining Rust, Go, and Elixir

---

## Executive Summary

This document consolidates all roadmaps and their current completion status as of February 2026.

### Overall Progress by Phase

| Phase | Status | Completion | Notes |
|-------|--------|------------|-------|
| Phase 1 (v1.0) Foundation | ✅ COMPLETE | 100% | Production Ready |
| Phase 1.5 (v1.1) Clean Architecture | 🟡 PARTIAL | 67% | Rust Core pending |
| Phase 1.5 (v1.2) Performance | ❌ NOT STARTED | 0% | Blocked by v1.1 |
| Phase 2 (v1.3-v1.7) Clojure Layer | ✅ COMPLETE | 100% | Ahead of schedule |
| Query Service Phase 1 | ✅ COMPLETE | 100% | 281 tests passing |
| Query Service Phase 2 | ❌ NOT STARTED | 0% | Core Integration |
| MCP Server v2.0 Phase 1 | ✅ COMPLETE | 100% | 13 tools |
| MCP Server AI-Native | ❌ NOT STARTED | 0% | Embedded expertise |
| Phase 3 (v1.8-v2.0) Enterprise | ⏳ PLANNED | 0% | 2027 |

---

## COMPLETED ITEMS

### Phase 1: Foundation (v1.0) - 100% COMPLETE

**Completion Date**: 2025-10-21

#### Rust Core (469K events/sec)
- [x] High-performance event ingestion
- [x] Write-ahead log (WAL) with durability
- [x] Parquet storage for efficient queries
- [x] Multi-tenant isolation with quotas
- [x] Event indexing (entity, type-based)
- [x] Snapshot system for state reconstruction
- [x] Real-time WebSocket streaming
- [x] Compaction for storage optimization
- [x] JWT authentication & RBAC
- [x] Rate limiting (token bucket)
- [x] Backup & restore capabilities

#### Go Control Plane
- [x] JWT authentication client
- [x] Role-based access control (RBAC)
- [x] Policy engine with 5 default policies
- [x] Comprehensive audit logging
- [x] Prometheus metrics integration
- [x] OpenTelemetry tracing (Jaeger)
- [x] Health checks and cluster status
- [x] RESTful management API (12 endpoints)
- [x] Domain layer with entities and repository interfaces
- [x] Application layer with use cases and ports
- [x] Infrastructure layer with concrete implementations
- [x] Dependency injection with Google Wire
- [x] 95%+ test coverage
- [x] SOLID principles compliance

#### Quality & Testing
- [x] 176+ tests passing (98.9% pass rate)
- [x] 17 performance benchmarks
- [x] Comprehensive documentation
- [x] Integration test suite

---

### Phase 2: Clojure Integration Layer (v1.3-v1.7) - 100% COMPLETE

**Completion Date**: 2025-10-24 (AHEAD OF SCHEDULE - Originally Q1-Q4 2026)

#### v1.3: Query DSL + REPL - COMPLETE
- [x] Query DSL library (400 LOC)
- [x] Query compiler and optimizer (200 LOC)
- [x] HTTP client with connection pooling (300 LOC)
- [x] Component-based DI (200 LOC)
- [x] Interactive REPL (200 LOC)
- [x] 50+ unit tests (90% coverage)

#### v1.4: Projection Management - COMPLETE
- [x] Projection domain entities
- [x] Projection executor with lifecycle management
- [x] In-memory state store
- [x] PostgreSQL state store (HikariCP pooling)
- [x] Redis state store (TTL support)
- [x] Snapshot creation/restoration
- [x] State migration between versions
- [x] Hot-reload projections
- [x] Multi-projection support
- [x] 100+ unit tests

#### v1.5: Event Processing Pipelines - COMPLETE
- [x] Pipeline domain entities
- [x] Pipeline executor with metrics
- [x] 10 composable operators (Filter, Map, Flat-Map, Enrich, Window, Batch, Throttle, Deduplicate, Partition, Aggregate)
- [x] Tumbling and sliding windows
- [x] Backpressure handling (3 strategies)
- [x] Async pipeline execution
- [x] Parallel pipeline execution
- [x] Per-operator metrics collection
- [x] 80+ unit tests

#### v1.6: Analytics Engine - COMPLETE
- [x] Analytics domain entities
- [x] Analytics engine implementation
- [x] 11 aggregation functions
- [x] Time-series analytics
- [x] Funnel analysis with conversion tracking
- [x] Cohort analysis with retention
- [x] Trend analysis with forecasting
- [x] Anomaly detection (3 algorithms: Z-score, IQR, MAD)
- [x] Data quality metrics
- [x] 100+ unit tests

#### v1.7: Integration & Tools - COMPLETE
- [x] Integration tools domain
- [x] Event replay (sequential + parallel)
- [x] Event validation with custom rules
- [x] Schema migration with versioning
- [x] Rollback support for migrations
- [x] Data quality metrics calculation
- [x] Common validation rules
- [x] Common migration patterns
- [x] 100+ unit tests

**Phase 2 Summary**: ~10,000 LOC, 240+ tests, TDD methodology

---

### Query Service (Elixir) Phase 1 - 100% COMPLETE

**Status**: Production-Ready (281/281 tests passing)

#### 1.1 Query DSL - COMPLETE (54 tests)
- [x] Fluent query building with Elixir pipes
- [x] Predicates: eq, gt, lt, gte, lte, between, in, not_in
- [x] Time helpers: days_ago, hours_ago, since, until
- [x] Sorting, limiting, field projection

#### 1.2 Projections - COMPLETE (61 tests)
- [x] GenServer-based state management
- [x] OTP supervision for fault tolerance
- [x] Event application to projections
- [x] Snapshot support (in-memory)
- [x] Current/historical state queries

#### 1.3 Event Pipelines - COMPLETE (81 tests)
- [x] 6 operator types: Filter, Transform, Enrich, Validate, Route, Aggregate
- [x] Batch processing
- [x] Statistics tracking
- [x] Error handling

#### 1.4 HTTP Client - COMPLETE (34 tests)
- [x] Tesla-based client to Core
- [x] Connection pooling
- [x] Event CRUD, queries, snapshots
- [x] Error handling

#### 1.5 Phoenix HTTP API - COMPLETE (5 tests, 11 endpoints)
- [x] GET/POST /api/events
- [x] POST /api/query
- [x] GET/POST /api/projections
- [x] GET /api/health, /api/metrics

#### 1.6 Production Readiness - COMPLETE
- [x] Docker with multi-stage build
- [x] Mix releases
- [x] Health checks & metrics
- [x] Environment-based configuration

---

### MCP Server v2.0 Phase 1 - 100% COMPLETE

**Status**: 13 advanced tools implemented (~900 LOC)

- [x] advanced_query - Complex queries with aggregations
- [x] time_series_analysis - Trend analysis over time
- [x] funnel_analysis - Conversion tracking
- [x] detect_anomalies - Real-time anomaly detection
- [x] create_projection - Materialized views
- [x] get_projection_state - Query projections
- [x] list_projections - List all projections
- [x] execute_pipeline - Event processing
- [x] replay_events - Event replay
- [x] validate_events - Data validation
- [x] create_policy - Policy creation
- [x] evaluate_policy - Policy evaluation
- [x] list_policies - List policies

---

## REMAINING ITEMS (TO BE CONVERTED TO BEADS)

### HIGH PRIORITY - Phase 1.5

#### v1.1: Rust Core Clean Architecture Refactoring (4-6 weeks)

**Status**: NOT STARTED
**Priority**: CRITICAL (blocks v1.2 performance optimizations)

##### Domain Layer
- [ ] Extract domain layer (entities, value objects, aggregates)
- [ ] Create `PartitionKey` value object (32 partitions)
- [ ] Create `StreamVersion` value object (gapless tracking)
- [ ] Create `EventStream` aggregate with watermarks
- [ ] Create `EventStoreFork` entity for agent experimentation (Agentic Postgres pattern)

##### Application Layer
- [ ] Create use cases: ingest_event, query_events, create_snapshot, replay_events
- [ ] Create use cases: create_fork, query_fork, cleanup_forks
- [ ] Create application services: EventService, ProjectionService, ForkService
- [ ] Create DTOs for all use cases

##### Infrastructure Layer
- [ ] Refactor persistence layer (ParquetEventRepository, WALEventRepository)
- [ ] Implement web handlers and middleware
- [ ] Set up dependency injection container
- [ ] Implement repository traits

##### Production Readiness (SierraDB patterns)
- [ ] 7-day continuous stress tests
- [ ] Storage integrity checks (checksums)
- [ ] WAL integrity verification
- [ ] Partition monitoring
- [ ] Corruption detection on startup

---

#### v1.2: Performance Optimization (8-10 weeks)

**Status**: NOT STARTED
**Dependencies**: v1.1 Rust Refactoring

##### Rust Performance (4-5 weeks)
- [ ] Zero-copy deserialization (simd-json)
- [ ] Lock-free data structures (crossbeam, dashmap)
- [ ] Batch processing (10K batch size)
- [ ] Memory pool for allocations (bumpalo)
- [ ] SIMD for event processing

**Targets**:
- Ingestion: 1M+ events/sec (current: 469K)
- Query latency: <5μs p99 (current: 11.9μs)
- Memory: <2GB for 100M events (current: ~3GB)

##### Go Control Plane Performance (2-3 weeks)
- [ ] Connection pooling
- [ ] Response caching
- [ ] Async audit logging

**Targets**:
- Latency: <5ms p99
- Throughput: 10K+ req/sec

##### Clojure Services Performance (2-3 weeks)
- [ ] Transducers for efficiency
- [ ] Reducers for parallelism
- [ ] Persistent data structure tuning
- [ ] Transients for large collections

---

### MEDIUM PRIORITY - MCP AI-Native Enhancements

#### Embedded Expertise (1 week)
- [ ] Enhanced tool descriptions with agent guidance
- [ ] Best practices and common patterns in descriptions
- [ ] Performance tips for each tool
- [ ] Decision trees for tool selection

#### Agent Advisory Tool (1 week)
- [ ] `get_query_advice` tool implementation
- [ ] Use-case specific recommendations
- [ ] Query pattern suggestions
- [ ] Performance optimization tips

#### Multi-Turn Conversational Context (1 week)
- [ ] ConversationContext manager
- [ ] Session-based query refinement
- [ ] Iterative query composition
- [ ] Context preservation

#### Quick Exploration Tools (1 week)
- [ ] `sample_events` for fast exploration
- [ ] `quick_stats` for rapid statistics
- [ ] Stratified sampling options

---

### MEDIUM PRIORITY - Native Search Capabilities (v1.2)

#### Vector Search (2 weeks)
- [ ] Vector search engine using fastembed
- [ ] HNSW index implementation
- [ ] Event embedding generation
- [ ] Semantic similarity search

#### BM25 Keyword Search (1 week)
- [ ] Keyword search engine using tantivy
- [ ] Event payload indexing
- [ ] Full-text search implementation

#### Hybrid Search (1 week)
- [ ] Hybrid search orchestrator
- [ ] Score combination and re-ranking
- [ ] Metadata filter integration
- [ ] MCP search tools (semantic_search_events, hybrid_search)

---

### MEDIUM PRIORITY - Query Service Phase 2 (3-4 weeks)

#### 2.1 WebSocket Integration (1 week)
- [ ] CoreWebSocketClient module (WebSockex)
- [ ] PubSub integration
- [ ] Auto-reconnect with backoff
- [ ] 15+ tests
- [ ] Optional Phoenix Channels relay

#### 2.2 Projection State Sync (1 week)
- [ ] Core API endpoints for projection state (Rust side)
- [ ] ProjectionSync GenServer (Elixir)
- [ ] ETS cache for local reads
- [ ] Restore from Core on restart
- [ ] 20+ Elixir tests, 15+ Rust tests

#### 2.3 Broadway Integration (1-2 weeks)
- [ ] Production CoreProducer
- [ ] EventPipeline Broadway
- [ ] Cursor tracking & persistence
- [ ] Performance benchmarks (10K events/sec)
- [ ] 15+ tests

---

### OPTIONAL - Redis Protocol (v1.2)

**Status**: OPTIONAL (2-3 weeks if multi-language support is priority)

- [ ] RESP3 server implementation
- [ ] Redis command mapping (XADD, XRANGE, SUBSCRIBE)
- [ ] Integration tests with redis-cli
- [ ] Documentation for Redis clients

---

### FUTURE - Phase 3 Enterprise Features (2027)

#### v1.8: Multi-Node Clustering (Q1 2027)
- [ ] Term-based consensus (simplified Raft)
- [ ] Deterministic leader selection
- [ ] Partition replication
- [ ] Manual failover
- [ ] Cluster membership management

#### v1.9: Geo-Replication (Q2 2027)
- [ ] Cross-region event replication
- [ ] CRDT-based conflict resolution
- [ ] Hybrid logical clocks
- [ ] Regional failover
- [ ] Automatic failover (evolution from v1.8)

#### v2.0: Advanced Features (Q3-Q4 2027)
- [ ] EventQL (SQL-like query language)
- [ ] GraphQL API
- [ ] Full-text search integration
- [ ] Geospatial queries
- [ ] Exactly-once stream processing
- [ ] Autonomous schema evolution

---

### FUTURE - Query Service Phase 3 (Q2-Q3 2026)

#### 3.1 Distributed Mode (2-3 weeks)
- [ ] libcluster for multi-node
- [ ] Distributed registry
- [ ] Consistent hashing

#### 3.2 Advanced Analytics (2-3 weeks)
- [ ] Leverage Core's `/api/v1/analytics/*` endpoints
- [ ] Time-window aggregations
- [ ] Statistical functions

#### 3.3 Message Queue Integration (2-3 weeks)
- [ ] Kafka integration
- [ ] RabbitMQ integration

#### 3.4 Monitoring & Observability (1-2 weeks)
- [ ] Prometheus exporter
- [ ] Grafana dashboards
- [ ] APM integration

#### 3.5 API Documentation (1 week)
- [ ] OpenAPI spec
- [ ] Swagger UI

---

## Priority Summary for Beads Conversion

### Immediate (Q1 2026)

| Epic | Priority | Estimated Effort |
|------|----------|-----------------|
| Rust Core Clean Architecture | HIGH | 4-6 weeks |
| MCP AI-Native Enhancements | MEDIUM | 2-3 weeks |
| Query Service Phase 2 | MEDIUM | 3-4 weeks |

### Short-term (Q2 2026)

| Epic | Priority | Estimated Effort |
|------|----------|-----------------|
| Performance Optimizations | HIGH | 8-10 weeks |
| Native Search Capabilities | MEDIUM | 3-4 weeks |
| Query Service Phase 3 | LOW | 6-8 weeks |

### Future (2027)

| Epic | Priority | Estimated Effort |
|------|----------|-----------------|
| Multi-Node Clustering (v1.8) | LOW | 5 weeks |
| Geo-Replication (v1.9) | LOW | 6-8 weeks |
| Advanced Features (v2.0) | LOW | 12-16 weeks |

---

## References

- Previous Roadmap: docs/roadmaps/2025-10-22_COMPREHENSIVE_ROADMAP.md
- Status Assessment: docs/roadmaps/2025-10-24_ROADMAP_STATUS_ASSESSMENT.md
- Query Service Roadmap: docs/roadmaps/query-service-roadmap.md
- MCP Enhancements: docs/roadmaps/mcp-v2-enhancements.md
- SierraDB Integration: docs/roadmaps/2025-10-26_SIERRADB_LEARNINGS_INTEGRATION.md

---

**Document Status**: CURRENT
**Last Updated**: 2026-02-02
**Next Review**: After Rust Core Refactoring completion
