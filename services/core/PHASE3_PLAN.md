# Phase 3: Infrastructure Layer Refactoring Plan

## 🎯 Objective

Complete the Clean Architecture refactoring by organizing the Infrastructure layer with proper separation of concerns, dependency injection, and the repository pattern.

## 📊 Current Status

**✅ Completed:**
- Phase 1: Domain Layer (162 tests) - Value Objects, Entities
- Phase 2: Application Layer (20 tests) - DTOs, Use Cases

**🚧 Phase 3: Infrastructure Layer**
- Current: 37 infrastructure tests scattered across root files
- Target: Organized infrastructure with clear boundaries

## 🏗️ Proposed Directory Structure

```
src/
├── domain/              # ✅ PHASE 1 COMPLETE
│   ├── entities/
│   └── value_objects/
│
├── application/         # ✅ PHASE 2 COMPLETE
│   ├── dto/
│   └── use_cases/
│
└── infrastructure/      # 🚧 PHASE 3 - TO BE CREATED
    ├── repositories/    # Repository implementations
    │   ├── mod.rs
    │   ├── event_repository.rs
    │   ├── tenant_repository.rs
    │   ├── schema_repository.rs
    │   └── projection_repository.rs
    │
    ├── persistence/     # Storage implementations
    │   ├── mod.rs
    │   ├── in_memory_store.rs
    │   ├── parquet_store.rs
    │   ├── wal.rs
    │   └── snapshot.rs
    │
    ├── services/        # Technical services
    │   ├── mod.rs
    │   ├── backup_service.rs
    │   ├── compaction_service.rs
    │   ├── replay_service.rs
    │   ├── analytics_service.rs
    │   ├── websocket_service.rs
    │   └── pipeline_service.rs
    │
    ├── api/             # HTTP API layer
    │   ├── mod.rs
    │   ├── v1/
    │   │   ├── mod.rs
    │   │   ├── event_handlers.rs
    │   │   ├── tenant_handlers.rs
    │   │   ├── schema_handlers.rs
    │   │   └── projection_handlers.rs
    │   ├── middleware.rs
    │   └── routes.rs
    │
    ├── security/        # Auth & security
    │   ├── mod.rs
    │   ├── auth.rs
    │   └── rate_limit.rs
    │
    ├── config/          # Configuration
    │   ├── mod.rs
    │   └── settings.rs
    │
    └── metrics/         # Observability
        ├── mod.rs
        └── prometheus.rs
```

## 📋 Implementation Phases

### Phase 3.1: Repository Pattern (Week 1)

#### Step 1: Define Repository Traits

Create `src/infrastructure/repositories/traits.rs`:

```rust
use crate::domain::entities::{Event, Tenant, Schema, Projection};
use crate::domain::value_objects::{TenantId, EventType, EntityId};
use crate::error::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Repository for Event entities
pub trait EventRepository: Send + Sync {
    /// Store a new event
    fn save(&self, event: Event) -> Result<()>;

    /// Save multiple events in a batch
    fn save_batch(&self, events: Vec<Event>) -> Result<()>;

    /// Find events by entity ID
    fn find_by_entity(&self, entity_id: &EntityId) -> Result<Vec<Event>>;

    /// Find events by event type
    fn find_by_type(&self, event_type: &EventType) -> Result<Vec<Event>>;

    /// Find events by tenant
    fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Event>>;

    /// Find events in time range
    fn find_by_time_range(
        &self,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    ) -> Result<Vec<Event>>;

    /// Count events
    fn count(&self) -> usize;
}

/// Repository for Tenant entities
pub trait TenantRepository: Send + Sync {
    /// Create a new tenant
    fn create(&self, tenant: Tenant) -> Result<()>;

    /// Find tenant by ID
    fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>>;

    /// Update tenant
    fn update(&self, tenant: Tenant) -> Result<()>;

    /// Delete tenant
    fn delete(&self, id: &TenantId) -> Result<()>;

    /// List all tenants
    fn list(&self) -> Result<Vec<Tenant>>;
}

/// Repository for Schema entities
pub trait SchemaRepository: Send + Sync {
    /// Register a new schema
    fn register(&self, schema: Schema) -> Result<()>;

    /// Find schema by subject and version
    fn find_by_subject(&self, subject: &str, version: Option<u32>) -> Result<Option<Schema>>;

    /// List all versions of a subject
    fn list_versions(&self, subject: &str) -> Result<Vec<Schema>>;

    /// List all subjects
    fn list_subjects(&self) -> Result<Vec<String>>;

    /// Update schema metadata
    fn update(&self, schema: Schema) -> Result<()>;
}

/// Repository for Projection entities
pub trait ProjectionRepository: Send + Sync {
    /// Create a new projection
    fn create(&self, projection: Projection) -> Result<()>;

    /// Find projection by ID
    fn find_by_id(&self, id: &Uuid) -> Result<Option<Projection>>;

    /// Find projections by tenant
    fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Projection>>;

    /// Update projection
    fn update(&self, projection: Projection) -> Result<()>;

    /// Delete projection
    fn delete(&self, id: &Uuid) -> Result<()>;

    /// List all projections
    fn list(&self) -> Result<Vec<Projection>>;
}
```

#### Step 2: Implement In-Memory Repositories

Create `src/infrastructure/repositories/in_memory_event_repository.rs`:

```rust
use super::traits::EventRepository;
use crate::domain::entities::Event;
use crate::domain::value_objects::{TenantId, EventType, EntityId};
use crate::error::Result;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct InMemoryEventRepository {
    events: Arc<RwLock<Vec<Event>>>,
}

impl InMemoryEventRepository {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl EventRepository for InMemoryEventRepository {
    fn save(&self, event: Event) -> Result<()> {
        self.events.write().push(event);
        Ok(())
    }

    fn save_batch(&self, events: Vec<Event>) -> Result<()> {
        self.events.write().extend(events);
        Ok(())
    }

    fn find_by_entity(&self, entity_id: &EntityId) -> Result<Vec<Event>> {
        Ok(self.events.read()
            .iter()
            .filter(|e| e.relates_to_entity(entity_id))
            .cloned()
            .collect())
    }

    fn find_by_type(&self, event_type: &EventType) -> Result<Vec<Event>> {
        Ok(self.events.read()
            .iter()
            .filter(|e| e.is_type(event_type))
            .cloned()
            .collect())
    }

    fn find_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Event>> {
        Ok(self.events.read()
            .iter()
            .filter(|e| e.belongs_to_tenant(tenant_id))
            .cloned()
            .collect())
    }

    fn find_by_time_range(
        &self,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    ) -> Result<Vec<Event>> {
        Ok(self.events.read()
            .iter()
            .filter(|e| {
                let timestamp = e.timestamp();
                let after_since = since.map_or(true, |s| timestamp >= s);
                let before_until = until.map_or(true, |u| timestamp <= u);
                after_since && before_until
            })
            .cloned()
            .collect())
    }

    fn count(&self) -> usize {
        self.events.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Event;

    #[test]
    fn test_save_and_find_by_entity() {
        let repo = InMemoryEventRepository::new();
        let event = Event::from_strings(
            "tenant-1",
            "test.event",
            "entity-1",
            serde_json::json!({"test": true}),
        ).unwrap();

        repo.save(event.clone()).unwrap();

        let entity_id = EntityId::new("entity-1".to_string()).unwrap();
        let found = repo.find_by_entity(&entity_id).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].entity_id(), event.entity_id());
    }
}
```

**Similar implementations needed for:**
- `in_memory_tenant_repository.rs`
- `in_memory_schema_repository.rs`
- `in_memory_projection_repository.rs`

**Estimated Work:** ~600 lines of code, 20 tests

### Phase 3.2: Refactor Use Cases to Use Repositories (Week 2)

#### Update Use Cases

Modify use cases to accept repository dependencies:

```rust
// Before
impl IngestEventUseCase {
    pub fn execute(store: &EventStore, request: IngestEventRequest) -> Result<IngestEventResponse> {
        // ...
    }
}

// After
impl IngestEventUseCase {
    pub fn execute<R: EventRepository>(
        repository: &R,
        request: IngestEventRequest,
    ) -> Result<IngestEventResponse> {
        let event = Event::from_strings(
            &request.tenant_id.unwrap_or_else(|| TenantId::default().to_string()),
            &request.event_type,
            &request.entity_id,
            request.payload,
        )?;

        repository.save(event.clone())?;

        Ok(IngestEventResponse {
            event: EventDto::from(&event),
        })
    }
}
```

**Files to Update:**
- `src/application/use_cases/ingest_event.rs`
- `src/application/use_cases/query_events.rs`
- `src/application/use_cases/manage_tenant.rs`
- `src/application/use_cases/manage_schema.rs`
- `src/application/use_cases/manage_projection.rs`

**Estimated Work:** ~300 lines changed, 0 new tests (existing tests updated)

### Phase 3.3: API Layer Refactoring (Week 3)

#### Create API Handler Structure

`src/infrastructure/api/v1/event_handlers.rs`:

```rust
use axum::{
    extract::{Query, State},
    Json,
};
use crate::application::dto::{IngestEventRequest, IngestEventResponse, QueryEventsRequest};
use crate::application::use_cases::{IngestEventUseCase, QueryEventsUseCase};
use crate::infrastructure::repositories::traits::EventRepository;
use crate::error::Result;
use std::sync::Arc;

pub struct EventHandlers<R: EventRepository> {
    repository: Arc<R>,
}

impl<R: EventRepository> EventHandlers<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub async fn ingest_event(
        &self,
        Json(request): Json<IngestEventRequest>,
    ) -> Result<Json<IngestEventResponse>> {
        let response = IngestEventUseCase::execute(&*self.repository, request)?;
        Ok(Json(response))
    }

    pub async fn query_events(
        &self,
        Query(request): Query<QueryEventsRequest>,
    ) -> Result<Json<QueryEventsResponse>> {
        let response = QueryEventsUseCase::execute(&*self.repository, request)?;
        Ok(Json(response))
    }
}
```

**Similar handlers needed for:**
- `tenant_handlers.rs`
- `schema_handlers.rs`
- `projection_handlers.rs`

**Estimated Work:** ~400 lines of code, 16 tests

### Phase 3.4: Dependency Injection Container (Week 4)

#### Create DI Container

`src/infrastructure/container.rs`:

```rust
use std::sync::Arc;
use crate::infrastructure::repositories::{
    InMemoryEventRepository,
    InMemoryTenantRepository,
    InMemorySchemaRepository,
    InMemoryProjectionRepository,
};
use crate::infrastructure::api::v1::{
    EventHandlers,
    TenantHandlers,
    SchemaHandlers,
    ProjectionHandlers,
};

pub struct Container {
    // Repositories
    pub event_repository: Arc<InMemoryEventRepository>,
    pub tenant_repository: Arc<InMemoryTenantRepository>,
    pub schema_repository: Arc<InMemorySchemaRepository>,
    pub projection_repository: Arc<InMemoryProjectionRepository>,

    // API Handlers
    pub event_handlers: EventHandlers<InMemoryEventRepository>,
    pub tenant_handlers: TenantHandlers<InMemoryTenantRepository>,
    pub schema_handlers: SchemaHandlers<InMemorySchemaRepository>,
    pub projection_handlers: ProjectionHandlers<InMemoryProjectionRepository>,
}

impl Container {
    pub fn new() -> Self {
        // Create repositories
        let event_repository = Arc::new(InMemoryEventRepository::new());
        let tenant_repository = Arc::new(InMemoryTenantRepository::new());
        let schema_repository = Arc::new(InMemorySchemaRepository::new());
        let projection_repository = Arc::new(InMemoryProjectionRepository::new());

        // Create handlers
        let event_handlers = EventHandlers::new(event_repository.clone());
        let tenant_handlers = TenantHandlers::new(tenant_repository.clone());
        let schema_handlers = SchemaHandlers::new(schema_repository.clone());
        let projection_handlers = ProjectionHandlers::new(projection_repository.clone());

        Self {
            event_repository,
            tenant_repository,
            schema_repository,
            projection_repository,
            event_handlers,
            tenant_handlers,
            schema_handlers,
            projection_handlers,
        }
    }
}
```

**Estimated Work:** ~200 lines of code, 4 tests

## 📊 Migration Strategy

### Option 1: Big Bang (Not Recommended)
- Move all files at once
- High risk of breaking changes
- Difficult to test incrementally

### Option 2: Incremental Migration (Recommended)

**Week 1:** Repository Pattern
- ✅ Create repository traits
- ✅ Implement in-memory repositories
- ✅ Add repository tests
- ⚠️ Keep existing code working

**Week 2:** Use Case Migration
- ✅ Update use cases to use repositories
- ✅ Update use case tests
- ⚠️ Maintain backward compatibility

**Week 3:** API Layer
- ✅ Create new API handlers
- ✅ Gradually migrate endpoints
- ⚠️ Keep old endpoints working

**Week 4:** Container & Cleanup
- ✅ Implement DI container
- ✅ Wire up all dependencies
- ✅ Remove old infrastructure code
- ✅ Update main.rs

## 🧪 Testing Strategy

### Repository Tests
- Unit tests for each repository implementation
- Test CRUD operations
- Test query filters
- Test edge cases (empty results, duplicates)
- Test concurrency (using Arc/RwLock)

### Integration Tests
- Test use cases with repositories
- Test API handlers with repositories
- Test full request/response cycle
- Test error handling

### Migration Tests
- Keep existing tests passing during migration
- Add new tests for new patterns
- Ensure no regression

## 📈 Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Test Coverage | 219 tests | 260+ tests |
| Infrastructure Tests | 37 | 80+ |
| Repository Pattern | 0% | 100% |
| API Decoupling | 0% | 100% |
| Code Organization | Mixed | Clean Architecture |

## 🚀 Quick Start

```bash
# Create directory structure
mkdir -p src/infrastructure/{repositories,persistence,services,api/v1,security,config,metrics}

# Create repository trait file
touch src/infrastructure/repositories/traits.rs
touch src/infrastructure/repositories/mod.rs

# Create in-memory implementations
touch src/infrastructure/repositories/in_memory_event_repository.rs
touch src/infrastructure/repositories/in_memory_tenant_repository.rs
touch src/infrastructure/repositories/in_memory_schema_repository.rs
touch src/infrastructure/repositories/in_memory_projection_repository.rs

# Create API handlers
touch src/infrastructure/api/v1/event_handlers.rs
touch src/infrastructure/api/v1/tenant_handlers.rs
touch src/infrastructure/api/v1/schema_handlers.rs
touch src/infrastructure/api/v1/projection_handlers.rs
touch src/infrastructure/api/v1/mod.rs
touch src/infrastructure/api/mod.rs

# Create container
touch src/infrastructure/container.rs
touch src/infrastructure/mod.rs
```

## 📋 Checklist

### Phase 3.1: Repository Pattern
- [ ] Create `infrastructure/repositories/` directory
- [ ] Define repository traits
- [ ] Implement `InMemoryEventRepository`
- [ ] Implement `InMemoryTenantRepository`
- [ ] Implement `InMemorySchemaRepository`
- [ ] Implement `InMemoryProjectionRepository`
- [ ] Write repository tests (20 tests)
- [ ] All tests passing

### Phase 3.2: Use Case Migration
- [ ] Update `IngestEventUseCase`
- [ ] Update `QueryEventsUseCase`
- [ ] Update `manage_tenant` use cases
- [ ] Update `manage_schema` use cases
- [ ] Update `manage_projection` use cases
- [ ] Update all use case tests
- [ ] All tests still passing (219+)

### Phase 3.3: API Layer
- [ ] Create API handler structure
- [ ] Implement `EventHandlers`
- [ ] Implement `TenantHandlers`
- [ ] Implement `SchemaHandlers`
- [ ] Implement `ProjectionHandlers`
- [ ] Write API integration tests (16 tests)
- [ ] All tests passing

### Phase 3.4: Container & Cleanup
- [ ] Implement DI container
- [ ] Wire up dependencies in main.rs
- [ ] Migrate remaining infrastructure files
- [ ] Remove deprecated code
- [ ] Update documentation
- [ ] Final test run (260+ tests)

## 🎯 Expected Outcome

After Phase 3 completion:

```
✅ Domain Layer: Pure business logic (162 tests)
✅ Application Layer: Use cases & DTOs (20 tests)
✅ Infrastructure Layer: Clean implementation (80+ tests)
   - Repository pattern for data access
   - Dependency injection
   - Testable API handlers
   - Separated concerns

Total: 260+ tests, 100% passing
Clean Architecture: Fully implemented
```

## 📚 Resources

- [Clean Architecture by Robert C. Martin](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [Repository Pattern](https://martinfowler.com/eaaCatalog/repository.html)
- [Dependency Injection in Rust](https://github.com/Nercury/di-rs)
