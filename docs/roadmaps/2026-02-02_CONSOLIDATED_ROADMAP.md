---
title: "AllSource Event Store - Consolidated Roadmap"
status: CURRENT
last_updated: 2026-02-10
category: roadmap
supersedes:
  - "2025-10-22_COMPREHENSIVE_ROADMAP.md"
  - "2025-10-24_ROADMAP_STATUS_ASSESSMENT.md"
  - "query-service-roadmap.md"
  - "mcp-v2-enhancements.md"
---

# AllSource Event Store - Consolidated Roadmap

**Date**: 2026-02-03
**Version**: 4.0
**Vision**: A high-performance, AI-native event store combining Rust, Go, and Elixir

---

## Executive Summary

This document consolidates all roadmaps and their current completion status as of February 2026.

### Overall Progress by Phase

| Phase | Status | Completion | Notes |
|-------|--------|------------|-------|
| Phase 1 (v1.0) Foundation | ✅ COMPLETE | 100% | Production Ready |
| Phase 1.5 (v1.1) Clean Architecture | ✅ COMPLETE | 100% | 12/12 user stories done |
| Phase 1.5 (v1.2) Performance | ✅ COMPLETE | 100% | 8/8 user stories done |
| Phase 2 (v1.3-v1.7) Clojure Layer | ✅ COMPLETE | 100% | Ahead of schedule |
| Query Service Phase 1 | ✅ COMPLETE | 100% | 281 tests passing |
| Query Service Phase 2 | ✅ COMPLETE | 100% | Broadway + WebSocket |
| MCP Server v2.0 Phase 1 | ✅ COMPLETE | 100% | 13 tools |
| MCP Server AI-Native | ✅ COMPLETE | 100% | 4 AI-native enhancements |
| Native Search | ✅ COMPLETE | 100% | Vector + BM25 + Hybrid |
| Query Service Phase 3 | ⏳ PLANNED | 0% | Q2-Q3 2026 |
| MCP Server v2.0 Phase 2 | 🔄 IN PROGRESS | 19% | Event Management DONE (8/42 tools) |
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

## RECENTLY COMPLETED (February 2026)

### v1.1: Rust Core Clean Architecture Refactoring - ✅ 95% COMPLETE

**Status**: COMPLETE (8/12 user stories)
**Completion Date**: 2026-02-03

##### Domain Layer ✅ COMPLETE
- [x] Extract domain layer (entities, value objects, aggregates)
- [x] Create `PartitionKey` value object (32 partitions)
- [x] Create `StreamVersion` value object (gapless tracking)
- [x] Create `EventStream` aggregate with watermarks
- [x] Create `EventStoreFork` entity for agent experimentation (Agentic Postgres pattern)

##### Application Layer ✅ COMPLETE
- [x] Create use cases: ingest_event, query_events, create_snapshot, replay_events
- [x] Create use cases: create_fork, query_fork, cleanup_forks
- [x] Create application services: EventService, ProjectionService, ForkService
- [x] Create DTOs for all use cases

##### Infrastructure Layer ✅ COMPLETE
- [x] Refactor persistence layer (ParquetEventRepository, WALEventRepository)
- [x] Implement web handlers and middleware
- [x] Dependency injection container (partial - needs completion)
- [x] Implement repository traits

##### Production Readiness (SierraDB patterns) - REMAINING
- [ ] Set up dependency injection container (US-009)
- [ ] Storage integrity checks (US-010)
- [ ] Partition monitoring (US-011)
- [ ] 7-day continuous stress tests (US-012)

---

### v1.2: Performance Optimization - ✅ 75% COMPLETE

**Status**: MOSTLY COMPLETE (6/8 user stories)
**Performance Achieved**: 726K events/sec (up from 469K)

##### Rust Performance ✅ COMPLETE
- [x] Zero-copy deserialization (simd-json) - US-021
- [x] Lock-free data structures (crossbeam, dashmap) - US-022
- [x] Batch processing (10K batch size) - US-023
- [x] Memory pool for allocations (bumpalo) - US-024
- [ ] SIMD event filtering - US-025 (REMAINING)

**Achieved**:
- Ingestion: 726K events/sec (target: 1M+)
- SIMD JSON parsing: 824K events/sec
- Lock-free queue: 41M push/sec

##### Go Control Plane Performance ✅ COMPLETE
- [x] Connection pooling - US-026
- [ ] Response caching - US-027 (REMAINING)
- [ ] Async audit logging - US-028 (REMAINING)

---

### MCP AI-Native Enhancements - ✅ 100% COMPLETE

**Status**: COMPLETE (4/4 user stories)
**Completion Date**: 2026-02-03

#### Embedded Expertise ✅ COMPLETE (US-013)
- [x] Enhanced tool descriptions with agent guidance
- [x] Best practices and common patterns in descriptions
- [x] Performance tips for each tool
- [x] Decision trees for tool selection

#### Agent Advisory Tool ✅ COMPLETE (US-014)
- [x] `get_query_advice` tool implementation
- [x] Use-case specific recommendations
- [x] Query pattern suggestions
- [x] Performance optimization tips

#### Multi-Turn Conversational Context ✅ COMPLETE (US-015)
- [x] ConversationContext manager
- [x] Session-based query refinement
- [x] Iterative query composition
- [x] Context preservation

#### Quick Exploration Tools ✅ COMPLETE (US-016)
- [x] `sample_events` for fast exploration
- [x] `quick_stats` for rapid statistics
- [x] Stratified sampling options

---

### Native Search Capabilities - ✅ 100% COMPLETE

**Status**: COMPLETE (4/4 user stories)
**Completion Date**: 2026-02-03

#### Vector Search ✅ COMPLETE (US-029)
- [x] Vector search engine using fastembed
- [x] HNSW index implementation
- [x] Event embedding generation
- [x] Semantic similarity search

#### BM25 Keyword Search ✅ COMPLETE (US-030)
- [x] Keyword search engine using tantivy
- [x] Event payload indexing
- [x] Full-text search implementation

#### Hybrid Search ✅ COMPLETE (US-031)
- [x] Hybrid search orchestrator
- [x] Score combination and re-ranking
- [x] Metadata filter integration

#### MCP Search Tools ✅ COMPLETE (US-032)
- [x] `semantic_search_events` tool
- [x] `hybrid_search` tool
- [x] MCP integration

---

### Query Service Phase 2: Core Integration - ✅ 100% COMPLETE

**Status**: COMPLETE (4/4 user stories)
**Completion Date**: 2026-02-03

#### WebSocket Integration ✅ COMPLETE (US-017)
- [x] CoreWebSocketClient module (WebSockex)
- [x] PubSub integration
- [x] Auto-reconnect with backoff
- [x] 18+ tests passing

#### Projection State API ✅ COMPLETE (US-018)
- [x] Core API endpoints for projection state (Rust side)
- [x] DashMap-backed storage with 11.9μs access latency

#### Projection State Sync ✅ COMPLETE (US-019)
- [x] ProjectionSync GenServer (Elixir)
- [x] ETS cache for local reads
- [x] Restore from Core on restart
- [x] 20+ tests passing

#### Broadway Pipeline ✅ COMPLETE (US-020)
- [x] Production CoreProducer
- [x] EventPipeline Broadway
- [x] Cursor tracking & persistence
- [x] 56 tests passing

---

## RECENTLY COMPLETED - All Roadmap Items Done

### MCP Server v0.3.0 Event Management Tools - ✅ COMPLETE (2026-02-10)

**Status**: COMPLETE (8/8 tools implemented, 82 tests)

#### Lifecycle Operations (8 tools)
- [x] `delete_events` - Soft delete with GDPR/CCPA compliance, audit trail, dry-run preview
- [x] `archive_events` - Cold storage archival with retention policies
- [x] `restore_events` - Restore deleted or archived events with audit trail
- [x] `export_events` - Export to JSON/JSONL/CSV/Parquet with optional compression
- [x] `import_events` - Bulk import with validation, deduplication, entity ID mapping
- [x] `clone_entity` - Deep copy entity with events, optional PII sanitization
- [x] `merge_entities` - Combine event streams with chronological ordering
- [x] `split_entity` - Partition by event type, time range, or custom criteria

**Technical Details**:
- 82 comprehensive tests added
- All 179 protocol tests passing
- Rich tool descriptions with AI agent guidance
- Dry-run preview mode for all destructive operations
- Complete audit trail for compliance

---

### Production Readiness - ✅ ALL COMPLETE

| Bead ID | User Story | Priority | Status |
|---------|------------|----------|--------|
| chronos-monorepo-2q1.9 | US-009: Set up dependency injection container | P2 | ✅ Closed |
| chronos-monorepo-2q1.10 | US-010: Add storage integrity checks | P2 | ✅ Closed |
| chronos-monorepo-2q1.11 | US-011: Add partition monitoring | P3 | ✅ Closed |
| chronos-monorepo-2q1.12 | US-012: Add 7-day stress test suite | P3 | ✅ Closed |

### Performance Optimizations - ✅ ALL COMPLETE

| Bead ID | User Story | Priority | Status |
|---------|------------|----------|--------|
| chronos-monorepo-198.5 | US-025: Implement SIMD event filtering | P3 | ✅ Closed |
| chronos-monorepo-198.7 | US-027: Go Control Plane response caching | P3 | ✅ Closed |
| chronos-monorepo-198.8 | US-028: Go Control Plane async audit logging | P3 | ✅ Closed |

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

#### 3.1 Phoenix Channels WebSocket (1-2 weeks)
Expose WebSocket endpoint for external clients (currently only internal WS client to Core exists).

- [ ] Phoenix.Socket and Channel implementation
- [ ] `/ws` endpoint on port 3902
- [ ] EventChannel for `events:all`, `events:{entity_id}`, `events:type:{type}`
- [ ] ProjectionChannel for real-time projection state updates
- [ ] JWT authentication for WebSocket connections
- [ ] Presence tracking for connected clients
- [ ] Update API_REFERENCE.md to reflect actual implementation

**Why**: API docs currently describe `/ws` endpoint that doesn't exist. Clients need real-time streaming without polling.

#### 3.2 Distributed Mode (2-3 weeks)
- [ ] libcluster for multi-node
- [ ] Distributed registry
- [ ] Consistent hashing

#### 3.3 Advanced Analytics (2-3 weeks)
- [ ] Leverage Core's `/api/v1/analytics/*` endpoints
- [ ] Time-window aggregations
- [ ] Statistical functions

#### 3.4 Message Queue Integration (2-3 weeks)
- [ ] Kafka integration
- [ ] RabbitMQ integration

#### 3.5 Monitoring & Observability (1-2 weeks)
- [ ] Prometheus exporter
- [ ] Grafana dashboards
- [ ] APM integration

#### 3.6 API Documentation (1 week)
- [ ] OpenAPI spec
- [ ] Swagger UI

---

### FUTURE - MCP Server v2.0 Phase 2 (Q2-Q3 2026)

Expand from 13 tools to 55+ tools for comprehensive AI-native event store interaction.

#### Tool Categories to Add

**Event Management (8 tools)** ✅ COMPLETE (2026-02-10)
- [x] `delete_events` - Soft delete with audit trail (GDPR/CCPA compliant)
- [x] `archive_events` - Move to cold storage with retention policies
- [x] `restore_events` - Restore deleted or archived events
- [x] `export_events` - Export to JSON/JSONL/CSV/Parquet with compression
- [x] `import_events` - Bulk import with validation and deduplication
- [x] `clone_entity` - Deep copy entity with events
- [x] `merge_entities` - Combine event streams
- [x] `split_entity` - Partition event stream by criteria

**Schema & Validation (6 tools)**
- [ ] `register_schema` - Register event type schema
- [ ] `validate_schema` - Check schema compatibility
- [ ] `migrate_schema` - Version migration
- [ ] `list_schemas` - List all registered schemas
- [ ] `infer_schema` - Auto-generate from events
- [ ] `schema_diff` - Compare schema versions

**Advanced Analytics (8 tools)**
- [ ] `cohort_analysis` - User cohort retention
- [ ] `correlation_analysis` - Event correlations
- [ ] `forecast_events` - Predictive analytics
- [ ] `segment_analysis` - Customer segmentation
- [ ] `path_analysis` - User journey mapping
- [ ] `attribution_analysis` - Multi-touch attribution
- [ ] `churn_prediction` - Churn risk scoring
- [ ] `ltv_calculation` - Lifetime value estimation

**Operational Tools (10 tools)**
- [ ] `compact_storage` - Trigger compaction
- [ ] `storage_stats` - Disk usage analytics
- [ ] `partition_info` - Partition health
- [ ] `wal_status` - WAL statistics
- [ ] `backup_create` - Create backup
- [ ] `backup_restore` - Restore from backup
- [ ] `backup_list` - List available backups
- [ ] `health_deep` - Deep health check
- [ ] `performance_report` - Performance metrics
- [ ] `audit_log` - Query audit trail

**Multi-Tenancy (6 tools)**
- [ ] `tenant_create` - Provision tenant
- [ ] `tenant_update` - Update settings
- [ ] `tenant_usage` - Usage statistics
- [ ] `tenant_quotas` - Quota management
- [ ] `tenant_suspend` - Suspend tenant
- [ ] `tenant_export` - Export tenant data

**Developer Experience (4 tools)**
- [ ] `generate_client` - SDK code generation
- [ ] `mock_events` - Generate test data
- [ ] `debug_query` - Query explain plan
- [ ] `benchmark_query` - Query performance test

**Total**: 27 existing (13 + 6 AI-Native + 8 Event Management) + 34 remaining = 61 tools

---

## Completion Summary

### Completed in February 2026 Release

| Epic | User Stories | Completion |
|------|--------------|------------|
| v1.1: Rust Core Clean Architecture | 12/12 | 100% |
| v1.2: Performance Optimizations | 8/8 | 100% |
| MCP AI-Native Enhancements | 4/4 | 100% |
| Native Search Capabilities | 4/4 | 100% |
| Query Service Phase 2 | 4/4 | 100% |
| GitHub Actions Optimization | 10/10 | 100% |
| Docker Image Publishing | 7/7 | 100% |
| Production Readiness Review | 15/15 | 100% |
| SaaS MVP | 15/15 | 100% |
| **MCP Event Management Tools** | **8/8** | **100%** |

**Total**: 87/87 user stories completed (100%)

### Remaining Work

All planned Q1 2026 roadmap items are complete. Future work includes:

| Epic | Timeline | Priority | Key Features |
|------|----------|----------|--------------|
| Query Service Phase 3 | Q2-Q3 2026 | MEDIUM | Phoenix Channels `/ws`, Distributed Mode |
| MCP Server v2.0 Phase 2 | Q2-Q3 2026 | MEDIUM | 27→61 tools (Event Management ✅, 34 remaining) |
| Redis Protocol (Optional) | Q2 2026 | LOW | RESP3 compatibility |

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
- Beads Issue Tracker: `.beads/issues.jsonl`

---

**Document Status**: CURRENT
**Last Updated**: 2026-02-10
**Next Review**: Q2 2026 (Query Service Phase 3)
