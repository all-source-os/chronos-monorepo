# Rust Core Clean Architecture Refactoring Plan

**Date**: 2025-10-24
**Status**: 🚧 In Progress
**Estimated Effort**: 4-6 weeks
**Priority**: HIGH (blocks performance optimizations)

---

## 🎯 Objectives

1. **Complete Clean Architecture Implementation**: Finish refactoring all legacy modules into proper domain/application/infrastructure layers
2. **Improve Testability**: Achieve >90% test coverage with true unit tests
3. **Dependency Inversion**: All dependencies flow inward (infrastructure → application → domain)
4. **Maintainability**: Clear separation of concerns, SOLID principles throughout
5. **Performance**: Maintain or improve current performance benchmarks

---

## 📊 Current State Analysis

### Existing Clean Architecture (Partial)

✅ **Domain Layer** (Good foundation)
```
src/domain/
├── entities/
│   ├── event.rs              ✅ Well-designed with validation
│   └── mod.rs
├── repositories/
│   ├── event_repository.rs   ✅ Good trait design (ISP)
│   └── mod.rs
├── value_objects/            ⚠️ Empty (needs population)
└── mod.rs
```

✅ **Application Layer** (Good foundation)
```
src/application/
├── dto/
│   ├── event_dto.rs          ✅ Basic DTOs exist
│   └── mod.rs
├── use_cases/
│   ├── ingest_event.rs       ✅ Good use case design
│   ├── query_events.rs       ✅ Good use case design
│   └── mod.rs
├── services/                 ⚠️ Empty (needs population)
└── mod.rs
```

⚠️ **Infrastructure Layer** (Needs work)
```
src/infrastructure/
├── persistence/              ⚠️ Empty
├── web/                      ⚠️ Empty
├── messaging/                ⚠️ Empty
└── mod.rs                    ✅ Exists
```

### Legacy Modules (To Be Refactored)

❌ **Root-level files** (26 modules marked as "legacy" in lib.rs):
```
src/
├── analytics.rs         → application/services/
├── api_v1.rs           → infrastructure/web/
├── api.rs              → infrastructure/web/
├── auth_api.rs         → infrastructure/web/
├── auth.rs             → infrastructure/security/ (new)
├── backup.rs           → infrastructure/persistence/
├── compaction.rs       → infrastructure/persistence/
├── event.rs            → application/dto/ (merge with existing)
├── index.rs            → infrastructure/persistence/
├── metrics.rs          → infrastructure/observability/ (new)
├── middleware.rs       → infrastructure/web/
├── pipeline.rs         → application/services/
├── projection.rs       → domain/entities/ + application/services/
├── rate_limit.rs       → infrastructure/middleware/ (new)
├── replay.rs           → application/services/
├── schema.rs           → domain/entities/
├── snapshot.rs         → infrastructure/persistence/
├── storage.rs          → infrastructure/persistence/
├── store.rs            → infrastructure/persistence/ (implement EventRepository)
├── tenant_api.rs       → infrastructure/web/
├── tenant.rs           → domain/entities/
├── wal.rs              → infrastructure/persistence/
├── websocket.rs        → infrastructure/web/
├── config.rs           ✅ Keep at root (shared configuration)
├── error.rs            ✅ Keep at root (shared error types)
└── main.rs             ✅ Keep at root (entry point)
```

---

## 🏗️ Target Architecture

### Layer 1: Domain (Enterprise Business Rules)

```
src/domain/
├── entities/
│   ├── event.rs                    ✅ EXISTS (enhance)
│   ├── tenant.rs                   🆕 NEW (from src/tenant.rs)
│   ├── schema.rs                   🆕 NEW (from src/schema.rs)
│   ├── projection.rs               🆕 NEW (from src/projection.rs)
│   ├── pipeline.rs                 🆕 NEW (pipeline domain logic)
│   └── mod.rs
├── value_objects/
│   ├── tenant_id.rs                🆕 NEW
│   ├── event_type.rs               🆕 NEW
│   ├── entity_id.rs                🆕 NEW
│   ├── timestamp.rs                🆕 NEW
│   └── mod.rs
├── aggregates/
│   ├── event_stream.rs             🆕 NEW
│   └── mod.rs
├── repositories/
│   ├── event_repository.rs         ✅ EXISTS (enhance)
│   ├── tenant_repository.rs        🆕 NEW
│   ├── schema_repository.rs        🆕 NEW
│   ├── projection_repository.rs    🆕 NEW
│   └── mod.rs
└── mod.rs
```

**Domain Rules**:
- Zero external dependencies (except std, serde, chrono, uuid)
- Only pure business logic
- No I/O operations
- No framework dependencies

---

### Layer 2: Application (Application Business Rules)

```
src/application/
├── dto/
│   ├── event_dto.rs                ✅ EXISTS (merge src/event.rs)
│   ├── tenant_dto.rs               🆕 NEW
│   ├── schema_dto.rs               🆕 NEW
│   ├── projection_dto.rs           🆕 NEW
│   ├── analytics_dto.rs            🆕 NEW
│   └── mod.rs
├── use_cases/
│   ├── ingest_event.rs             ✅ EXISTS
│   ├── query_events.rs             ✅ EXISTS
│   ├── ingest_batch.rs             🆕 NEW
│   ├── create_tenant.rs            🆕 NEW
│   ├── register_schema.rs          🆕 NEW
│   ├── create_projection.rs        🆕 NEW
│   ├── replay_events.rs            🆕 NEW (from src/replay.rs)
│   ├── backup_events.rs            🆕 NEW (from src/backup.rs)
│   └── mod.rs
├── services/
│   ├── analytics_service.rs        🆕 NEW (from src/analytics.rs)
│   ├── pipeline_service.rs         🆕 NEW (from src/pipeline.rs)
│   ├── projection_service.rs       🆕 NEW (projection management)
│   ├── query_service.rs            🆕 NEW
│   └── mod.rs
└── mod.rs
```

**Application Rules**:
- Depends only on domain layer
- Orchestrates domain entities
- Implements business use cases
- No framework-specific code

---

### Layer 3: Infrastructure (Interface Adapters)

```
src/infrastructure/
├── persistence/
│   ├── event_store_impl.rs         🆕 NEW (from src/store.rs, implements EventRepository)
│   ├── parquet_storage.rs          🆕 NEW (from src/storage.rs)
│   ├── write_ahead_log.rs          🆕 NEW (from src/wal.rs)
│   ├── event_index.rs              🆕 NEW (from src/index.rs)
│   ├── snapshot_manager.rs         🆕 NEW (from src/snapshot.rs)
│   ├── backup_manager.rs           🆕 NEW (from src/backup.rs)
│   ├── compaction_manager.rs       🆕 NEW (from src/compaction.rs)
│   ├── tenant_repository_impl.rs   🆕 NEW
│   ├── schema_repository_impl.rs   🆕 NEW
│   └── mod.rs
├── web/
│   ├── routes/
│   │   ├── event_routes.rs         🆕 NEW (from src/api_v1.rs)
│   │   ├── auth_routes.rs          🆕 NEW (from src/auth_api.rs)
│   │   ├── tenant_routes.rs        🆕 NEW (from src/tenant_api.rs)
│   │   └── mod.rs
│   ├── middleware/
│   │   ├── auth_middleware.rs      🆕 NEW (from src/middleware.rs)
│   │   ├── rate_limit_middleware.rs 🆕 NEW (from src/rate_limit.rs)
│   │   └── mod.rs
│   ├── websocket_handler.rs        🆕 NEW (from src/websocket.rs)
│   └── mod.rs
├── security/
│   ├── jwt_authenticator.rs        🆕 NEW (from src/auth.rs)
│   ├── password_hasher.rs          🆕 NEW
│   └── mod.rs
├── observability/
│   ├── metrics_collector.rs        🆕 NEW (from src/metrics.rs)
│   ├── tracing_config.rs           🆕 NEW
│   └── mod.rs
├── messaging/
│   └── mod.rs                      (future: event bus, message queue)
└── mod.rs
```

**Infrastructure Rules**:
- Implements domain repositories
- Framework-specific code (Axum, Arrow, Parquet)
- I/O operations
- Third-party integrations

---

## 🔄 Migration Strategy

### Phase 1: Foundation (Week 1-2)

**Goal**: Complete domain layer and core value objects

1. **Create Value Objects** (TDD - RED first)
   - [x] Write tests for TenantId value object
   - [ ] Implement TenantId value object (make tests GREEN)
   - [ ] Write tests for EventType value object
   - [ ] Implement EventType value object
   - [ ] Write tests for EntityId value object
   - [ ] Implement EntityId value object
   - [ ] Write tests for Timestamp value object
   - [ ] Implement Timestamp value object

2. **Create Domain Entities** (TDD - RED first)
   - [ ] Write tests for Tenant entity
   - [ ] Implement Tenant entity
   - [ ] Write tests for Schema entity
   - [ ] Implement Schema entity
   - [ ] Write tests for Projection entity
   - [ ] Implement Projection entity
   - [ ] Write tests for Pipeline entity (domain logic only)
   - [ ] Implement Pipeline entity

3. **Enhance Event Entity**
   - [ ] Write tests for Event aggregate behavior
   - [ ] Add aggregate methods (apply_to_stream, validate_sequence, etc.)
   - [ ] Refactor to use value objects instead of raw strings

4. **Create Repository Traits**
   - [ ] Write tests for TenantRepository trait (using mocks)
   - [ ] Define TenantRepository trait
   - [ ] Write tests for SchemaRepository trait
   - [ ] Define SchemaRepository trait
   - [ ] Write tests for ProjectionRepository trait
   - [ ] Define ProjectionRepository trait

### Phase 2: Application Layer (Week 2-3)

**Goal**: Complete application services and use cases

1. **Move DTOs**
   - [ ] Merge src/event.rs DTOs into application/dto/event_dto.rs
   - [ ] Create tenant_dto.rs
   - [ ] Create schema_dto.rs
   - [ ] Create projection_dto.rs
   - [ ] Create analytics_dto.rs

2. **Create Application Services** (TDD)
   - [ ] Write tests for AnalyticsService
   - [ ] Implement AnalyticsService (from src/analytics.rs)
   - [ ] Write tests for PipelineService
   - [ ] Implement PipelineService (from src/pipeline.rs)
   - [ ] Write tests for ProjectionService
   - [ ] Implement ProjectionService (from projection management logic)
   - [ ] Write tests for QueryService
   - [ ] Implement QueryService

3. **Create New Use Cases** (TDD)
   - [ ] Write tests for CreateTenantUseCase
   - [ ] Implement CreateTenantUseCase
   - [ ] Write tests for RegisterSchemaUseCase
   - [ ] Implement RegisterSchemaUseCase
   - [ ] Write tests for CreateProjectionUseCase
   - [ ] Implement CreateProjectionUseCase
   - [ ] Write tests for ReplayEventsUseCase
   - [ ] Implement ReplayEventsUseCase (from src/replay.rs)
   - [ ] Write tests for BackupEventsUseCase
   - [ ] Implement BackupEventsUseCase (from src/backup.rs)

### Phase 3: Infrastructure Layer (Week 3-4)

**Goal**: Move all infrastructure concerns to proper locations

1. **Persistence Layer** (TDD)
   - [ ] Write tests for EventStoreImpl (implementing EventRepository)
   - [ ] Refactor src/store.rs → infrastructure/persistence/event_store_impl.rs
   - [ ] Write tests for ParquetStorage adapter
   - [ ] Move src/storage.rs → infrastructure/persistence/parquet_storage.rs
   - [ ] Write tests for WriteAheadLog adapter
   - [ ] Move src/wal.rs → infrastructure/persistence/write_ahead_log.rs
   - [ ] Write tests for EventIndex adapter
   - [ ] Move src/index.rs → infrastructure/persistence/event_index.rs
   - [ ] Write tests for SnapshotManager
   - [ ] Move src/snapshot.rs → infrastructure/persistence/snapshot_manager.rs
   - [ ] Write tests for BackupManager
   - [ ] Move src/backup.rs → infrastructure/persistence/backup_manager.rs
   - [ ] Write tests for CompactionManager
   - [ ] Move src/compaction.rs → infrastructure/persistence/compaction_manager.rs

2. **Web Layer**
   - [ ] Write tests for event routes
   - [ ] Move src/api_v1.rs → infrastructure/web/routes/event_routes.rs
   - [ ] Write tests for auth routes
   - [ ] Move src/auth_api.rs → infrastructure/web/routes/auth_routes.rs
   - [ ] Write tests for tenant routes
   - [ ] Move src/tenant_api.rs → infrastructure/web/routes/tenant_routes.rs
   - [ ] Write tests for auth middleware
   - [ ] Move src/middleware.rs → infrastructure/web/middleware/auth_middleware.rs
   - [ ] Write tests for rate limit middleware
   - [ ] Move src/rate_limit.rs → infrastructure/web/middleware/rate_limit_middleware.rs
   - [ ] Write tests for websocket handler
   - [ ] Move src/websocket.rs → infrastructure/web/websocket_handler.rs

3. **Security Layer**
   - [ ] Write tests for JWT authenticator
   - [ ] Move src/auth.rs → infrastructure/security/jwt_authenticator.rs
   - [ ] Create password_hasher.rs with tests

4. **Observability Layer**
   - [ ] Write tests for metrics collector
   - [ ] Move src/metrics.rs → infrastructure/observability/metrics_collector.rs
   - [ ] Create tracing_config.rs

### Phase 4: Dependency Injection (Week 4-5)

**Goal**: Implement DI container for loose coupling

1. **Choose DI Approach**
   - [ ] Evaluate options: shaku, dill, or manual Arc-based DI
   - [ ] Decision: Use manual Arc-based DI (zero dependencies, Rust-native)

2. **Create DI Container** (TDD)
   - [ ] Write tests for Container struct
   - [ ] Implement Container with Arc-based dependency registration
   - [ ] Write tests for service resolution
   - [ ] Implement lazy initialization
   - [ ] Write tests for circular dependency detection
   - [ ] Implement cycle detection

3. **Wire Everything Together**
   - [ ] Create factory functions for domain services
   - [ ] Create factory functions for application use cases
   - [ ] Create factory functions for infrastructure adapters
   - [ ] Update main.rs to use container

### Phase 5: Integration & Validation (Week 5-6)

**Goal**: Ensure everything works together

1. **Update Tests**
   - [ ] Run existing tests and fix broken imports
   - [ ] Update integration tests to use new architecture
   - [ ] Add new integration tests for DI container
   - [ ] Achieve >90% test coverage

2. **Update Build**
   - [ ] Update lib.rs exports
   - [ ] Remove legacy module declarations
   - [ ] Update documentation comments
   - [ ] Run cargo check and fix all warnings

3. **Performance Validation**
   - [ ] Run benchmark suite (ensure no regression)
   - [ ] Profile hot paths with flamegraph
   - [ ] Optimize critical paths if needed
   - [ ] Document performance characteristics

4. **Documentation**
   - [ ] Create architecture diagrams
   - [ ] Document each layer's responsibilities
   - [ ] Create migration guide
   - [ ] Update README with new structure

---

## 📏 Success Criteria

- ✅ All legacy modules moved to appropriate layers
- ✅ Zero circular dependencies between layers
- ✅ >90% test coverage
- ✅ All tests passing (unit + integration)
- ✅ Benchmark performance maintained or improved
- ✅ Clean separation: domain has zero infrastructure dependencies
- ✅ Dependency injection container operational
- ✅ Comprehensive documentation

---

## 🛡️ Risk Mitigation

**Risk**: Breaking existing functionality
**Mitigation**:
- Follow TDD strictly (RED → GREEN → REFACTOR)
- Run tests after each file move
- Keep integration tests running throughout

**Risk**: Performance regression
**Mitigation**:
- Run benchmarks after each phase
- Profile before and after refactoring
- Use zero-cost abstractions (traits with monomorphization)

**Risk**: Scope creep
**Mitigation**:
- Stick to refactoring only (no new features)
- Use todo list to track progress
- Complete one phase before starting next

---

## 📦 Dependencies to Add

```toml
# No new dependencies required!
# Use existing dependencies:
# - std::sync::Arc for DI
# - async_trait for async repository traits
# - parking_lot for concurrent data structures
```

---

## 🎓 Principles Applied

1. **Dependency Inversion Principle (DIP)**: All dependencies point inward
2. **Single Responsibility Principle (SRP)**: Each module has one reason to change
3. **Open-Closed Principle (OCP)**: Open for extension, closed for modification
4. **Liskov Substitution Principle (LSP)**: Repository implementations are substitutable
5. **Interface Segregation Principle (ISP)**: Separate EventReader/EventWriter traits
6. **Don't Repeat Yourself (DRY)**: Value objects eliminate duplication
7. **Keep It Simple (KISS)**: Manual DI instead of complex framework
8. **You Aren't Gonna Need It (YAGNI)**: Only refactor what exists

---

## 📊 Progress Tracking

**Week 1**: Domain layer foundation
**Week 2**: Application layer completion
**Week 3**: Infrastructure persistence layer
**Week 4**: Infrastructure web/security/observability
**Week 5**: Dependency injection
**Week 6**: Integration, testing, documentation

---

## 🚀 Next Steps

1. ✅ Create this refactoring plan
2. 🚧 Start Phase 1: Create value objects (TDD)
3. ⏳ Continue through all phases systematically
4. ⏳ Document lessons learned
5. ⏳ Share results with team

**Status**: Ready to begin Phase 1 - Value Objects
**Current Task**: Write tests for TenantId value object (RED phase)
