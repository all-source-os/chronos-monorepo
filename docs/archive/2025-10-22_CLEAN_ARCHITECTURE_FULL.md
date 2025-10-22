# Clean Architecture Guide for AllSource Event Store

**Version**: 1.0
**Last Updated**: 2025-10-21
**Author**: AllSource Core Team

---

## 📋 Table of Contents

1. [Introduction](#introduction)
2. [The Four Layers](#the-four-layers)
3. [The Dependency Rule](#the-dependency-rule)
4. [Rust Implementation](#rust-implementation)
5. [Go Implementation](#go-implementation)
6. [Clojure Implementation](#clojure-implementation)
7. [Best Practices](#best-practices)
8. [Common Pitfalls](#common-pitfalls)
9. [Migration Strategies](#migration-strategies)
10. [Testing Strategies](#testing-strategies)

---

## Introduction

### What is Clean Architecture?

Clean Architecture is a software design philosophy created by Robert C. Martin (Uncle Bob) that emphasizes:

1. **Independence from Frameworks**: Business logic doesn't depend on Gin, Axum, or Ring
2. **Testability**: Business logic can be tested without UI, database, or external services
3. **Independence from UI**: Swap CLI for Web UI without changing business logic
4. **Independence from Database**: Swap Postgres for MongoDB without changing business logic
5. **Independence from External Agencies**: Business logic doesn't know about external systems

### Why Clean Architecture for Event Stores?

Event stores have complex requirements:
- **High Performance**: Critical path must be optimized
- **Data Integrity**: Zero data loss requirement
- **Flexibility**: Need to swap storage engines, query engines, etc.
- **Testability**: Must test without spinning up databases
- **Maintainability**: Code must be understandable by new developers

Clean Architecture helps us achieve all of these goals.

---

## The Four Layers

```
┌─────────────────────────────────────────────────────────┐
│         🔧 Frameworks & Drivers (Outermost)             │
│  Web Framework • Database • File System • External APIs │
└───────────────────────┬─────────────────────────────────┘
                        │ (Depends on ↓)
┌───────────────────────┴─────────────────────────────────┐
│           🔌 Interface Adapters                         │
│     Controllers • Presenters • Gateways • Repositories  │
└───────────────────────┬─────────────────────────────────┘
                        │ (Depends on ↓)
┌───────────────────────┴─────────────────────────────────┐
│          ⚙️  Application Business Rules                 │
│         Use Cases • Application Services • DTOs         │
└───────────────────────┬─────────────────────────────────┘
                        │ (Depends on ↓)
┌───────────────────────┴─────────────────────────────────┐
│         💎 Enterprise Business Rules (Innermost)        │
│         Entities • Value Objects • Domain Logic         │
└─────────────────────────────────────────────────────────┘
```

### Layer 1: Enterprise Business Rules (Domain)

**What belongs here?**
- Core business entities (Event, Tenant, User)
- Value objects (EventId, Timestamp, TenantId)
- Domain services (policy evaluation, validation)
- Repository interfaces (abstractions only)

**What doesn't belong here?**
- Database code
- HTTP handlers
- External API calls
- Framework-specific code

**Characteristics**:
- ✅ No dependencies on outer layers
- ✅ Pure business logic
- ✅ Framework-agnostic
- ✅ Highly testable

### Layer 2: Application Business Rules (Use Cases)

**What belongs here?**
- Use cases (IngestEvent, QueryEvents, CreateSnapshot)
- Application services (orchestrate multiple entities)
- Input/output ports (interfaces)
- DTOs (Data Transfer Objects)

**What doesn't belong here?**
- HTTP request/response handling
- Database queries
- External service calls
- UI logic

**Characteristics**:
- ✅ Depends only on Domain layer
- ✅ Orchestrates business logic
- ✅ Defines interfaces for infrastructure
- ✅ No framework dependencies

### Layer 3: Interface Adapters

**What belongs here?**
- Controllers/Handlers (HTTP, CLI)
- Presenters (format output)
- Gateways (implement ports)
- Repository implementations
- External service adapters

**What doesn't belong here?**
- Business logic
- Framework setup
- Database configuration

**Characteristics**:
- ✅ Adapts external formats to use cases
- ✅ Implements interfaces from Application layer
- ✅ Converts DTOs to/from external formats

### Layer 4: Frameworks & Drivers

**What belongs here?**
- Web framework setup (Axum, Gin, Ring)
- Database connections
- File system operations
- External API clients
- Configuration

**What doesn't belong here?**
- Business logic
- Use case orchestration

**Characteristics**:
- ✅ Glue code
- ✅ Framework-specific
- ✅ Easiest to replace
- ✅ Minimal logic

---

## The Dependency Rule

> **Source code dependencies must point only inward, toward higher-level policies.**

### What This Means:

```
❌ WRONG: Domain depends on Infrastructure
┌─────────────┐
│   Domain    │──────┐
│   (Event)   │      │ depends on
└─────────────┘      ↓
                ┌─────────────┐
                │ Parquet DB  │
                └─────────────┘

✅ CORRECT: Infrastructure depends on Domain
┌─────────────┐
│   Domain    │
│   (Event)   │
└─────────────┘
       ↑
       │ implements
┌─────────────┐
│ Parquet DB  │
└─────────────┘
```

### Inversion of Control

The inner layer defines an **interface**, the outer layer **implements** it:

```rust
// Domain layer (inner) - defines interface
pub trait EventRepository {
    async fn save(&self, event: Event) -> Result<()>;
    async fn find_by_id(&self, id: EventId) -> Result<Option<Event>>;
}

// Infrastructure layer (outer) - implements interface
pub struct ParquetEventRepository { /* ... */ }

impl EventRepository for ParquetEventRepository {
    async fn save(&self, event: Event) -> Result<()> {
        // Parquet-specific implementation
    }
}
```

---

## Rust Implementation

### Directory Structure

```
src/
├── domain/                    # Layer 1: Enterprise Business Rules
│   ├── entities/
│   │   ├── event.rs          # Core Event entity
│   │   ├── tenant.rs         # Tenant entity
│   │   ├── user.rs           # User entity
│   │   └── snapshot.rs       # Snapshot entity
│   ├── value_objects/
│   │   ├── event_id.rs       # Strongly-typed ID
│   │   ├── tenant_id.rs
│   │   ├── timestamp.rs
│   │   └── event_type.rs
│   ├── aggregates/
│   │   ├── event_stream.rs   # Aggregate root
│   │   └── tenant_config.rs
│   └── repositories/         # Repository traits (abstractions)
│       ├── event_repository.rs
│       ├── tenant_repository.rs
│       └── snapshot_repository.rs
│
├── application/               # Layer 2: Application Business Rules
│   ├── use_cases/
│   │   ├── ingest_event.rs   # One use case per file
│   │   ├── query_events.rs
│   │   ├── create_snapshot.rs
│   │   ├── replay_events.rs
│   │   └── manage_tenant.rs
│   ├── services/
│   │   ├── event_service.rs  # Application service
│   │   ├── projection_service.rs
│   │   └── analytics_service.rs
│   ├── dto/
│   │   ├── event_dto.rs
│   │   ├── query_dto.rs
│   │   └── response_dto.rs
│   └── ports/                # Interfaces for infrastructure
│       ├── event_store_port.rs
│       └── notification_port.rs
│
├── infrastructure/            # Layer 3: Interface Adapters
│   ├── persistence/
│   │   ├── parquet/
│   │   │   └── parquet_event_repository.rs
│   │   ├── wal/
│   │   │   └── wal_event_repository.rs
│   │   └── postgres/
│   │       └── postgres_tenant_repository.rs
│   ├── web/
│   │   ├── handlers/
│   │   │   ├── event_handler.rs
│   │   │   ├── query_handler.rs
│   │   │   └── tenant_handler.rs
│   │   ├── middleware/
│   │   │   ├── auth.rs
│   │   │   └── tracing.rs
│   │   └── routes.rs
│   ├── messaging/
│   │   ├── websocket_publisher.rs
│   │   └── kafka_publisher.rs
│   └── cache/
│       └── redis_cache.rs
│
├── config/                    # Layer 4: Frameworks & Drivers
│   ├── app_config.rs
│   └── dependency_injection.rs
│
└── lib.rs                     # Application entry point
```

### Example: Event Entity (Domain Layer)

```rust
// src/domain/entities/event.rs

use crate::domain::value_objects::{EventId, EventType, Timestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Core Event entity - represents an immutable event
///
/// This is pure domain logic with no infrastructure dependencies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: EventId,
    pub event_type: EventType,
    pub entity_id: String,
    pub tenant_id: String,
    pub payload: Value,
    pub timestamp: Timestamp,
    pub metadata: Option<Value>,
    pub version: u32,
}

impl Event {
    /// Create a new event with validation
    pub fn new(
        event_type: EventType,
        entity_id: String,
        tenant_id: String,
        payload: Value,
    ) -> Result<Self, DomainError> {
        // Domain validation
        if entity_id.is_empty() {
            return Err(DomainError::InvalidEntityId);
        }

        if tenant_id.is_empty() {
            return Err(DomainError::InvalidTenantId);
        }

        Ok(Self {
            id: EventId::generate(),
            event_type,
            entity_id,
            tenant_id,
            payload,
            timestamp: Timestamp::now(),
            metadata: None,
            version: 1,
        })
    }

    /// Check if event belongs to tenant
    pub fn belongs_to_tenant(&self, tenant_id: &str) -> bool {
        self.tenant_id == tenant_id
    }

    /// Check if event is older than duration
    pub fn is_older_than(&self, duration: std::time::Duration) -> bool {
        self.timestamp.elapsed() > duration
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid entity ID")]
    InvalidEntityId,

    #[error("Invalid tenant ID")]
    InvalidTenantId,
}
```

### Example: Repository Trait (Domain Layer)

```rust
// src/domain/repositories/event_repository.rs

use crate::domain::entities::Event;
use crate::domain::value_objects::{EventId, EventType};
use async_trait::async_trait;
use std::sync::Arc;

/// EventRepository defines the contract for event persistence
///
/// This is an abstraction - the domain defines WHAT operations it needs,
/// not HOW they are implemented.
#[async_trait]
pub trait EventRepository: Send + Sync {
    /// Save a single event
    async fn save(&self, event: Event) -> Result<(), RepositoryError>;

    /// Save multiple events in a batch
    async fn save_batch(&self, events: Vec<Event>) -> Result<(), RepositoryError>;

    /// Find event by ID
    async fn find_by_id(&self, id: &EventId) -> Result<Option<Event>, RepositoryError>;

    /// Find all events for an entity
    async fn find_by_entity(&self, entity_id: &str) -> Result<Vec<Event>, RepositoryError>;

    /// Find events by type
    async fn find_by_type(&self, event_type: &EventType) -> Result<Vec<Event>, RepositoryError>;

    /// Find events in time range
    async fn find_in_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Event>, RepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Event not found")]
    NotFound,

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

// Type alias for convenience
pub type DynEventRepository = Arc<dyn EventRepository>;
```

### Example: Use Case (Application Layer)

```rust
// src/application/use_cases/ingest_event.rs

use crate::domain::entities::Event;
use crate::domain::repositories::EventRepository;
use crate::application::dto::{IngestEventRequest, IngestEventResponse};
use std::sync::Arc;

/// IngestEvent use case orchestrates event ingestion
///
/// This is application logic that coordinates domain entities
/// and infrastructure without depending on specific implementations
pub struct IngestEventUseCase {
    event_repository: Arc<dyn EventRepository>,
    // Could add other dependencies like notification service
}

impl IngestEventUseCase {
    pub fn new(event_repository: Arc<dyn EventRepository>) -> Self {
        Self { event_repository }
    }

    /// Execute the use case
    pub async fn execute(
        &self,
        request: IngestEventRequest,
    ) -> Result<IngestEventResponse, IngestEventError> {
        // 1. Validate input (application-level validation)
        if request.payload.is_null() {
            return Err(IngestEventError::InvalidPayload);
        }

        // 2. Create domain entity
        let event = Event::new(
            request.event_type,
            request.entity_id,
            request.tenant_id,
            request.payload,
        )
        .map_err(|e| IngestEventError::DomainError(e.to_string()))?;

        // 3. Persist using repository abstraction
        self.event_repository
            .save(event.clone())
            .await
            .map_err(|e| IngestEventError::RepositoryError(e.to_string()))?;

        // 4. Return response
        Ok(IngestEventResponse {
            event_id: event.id.to_string(),
            timestamp: event.timestamp,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IngestEventError {
    #[error("Invalid payload")]
    InvalidPayload,

    #[error("Domain error: {0}")]
    DomainError(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),
}
```

### Example: Repository Implementation (Infrastructure Layer)

```rust
// src/infrastructure/persistence/parquet/parquet_event_repository.rs

use crate::domain::entities::Event;
use crate::domain::repositories::{EventRepository, RepositoryError};
use crate::domain::value_objects::{EventId, EventType};
use async_trait::async_trait;
use parquet::file::writer::SerializedFileWriter;
use std::path::PathBuf;

/// Parquet implementation of EventRepository
///
/// This is infrastructure code that implements the domain contract
pub struct ParquetEventRepository {
    data_dir: PathBuf,
    writer: SerializedFileWriter<std::fs::File>,
}

impl ParquetEventRepository {
    pub fn new(data_dir: PathBuf) -> Result<Self, std::io::Error> {
        // Infrastructure-specific initialization
        std::fs::create_dir_all(&data_dir)?;

        let writer = Self::create_writer(&data_dir)?;

        Ok(Self { data_dir, writer })
    }

    fn create_writer(data_dir: &PathBuf) -> Result<SerializedFileWriter<std::fs::File>, std::io::Error> {
        // Parquet-specific writer creation
        todo!("Create Parquet writer")
    }
}

#[async_trait]
impl EventRepository for ParquetEventRepository {
    async fn save(&self, event: Event) -> Result<(), RepositoryError> {
        // Parquet-specific save implementation
        // Convert domain Event to Parquet row
        // Write to file
        // Handle Parquet-specific errors

        Ok(())
    }

    async fn save_batch(&self, events: Vec<Event>) -> Result<(), RepositoryError> {
        // Batch write optimization for Parquet
        Ok(())
    }

    async fn find_by_id(&self, id: &EventId) -> Result<Option<Event>, RepositoryError> {
        // Read from Parquet file
        // Convert Parquet row to domain Event
        Ok(None)
    }

    async fn find_by_entity(&self, entity_id: &str) -> Result<Vec<Event>, RepositoryError> {
        // Use Parquet predicate pushdown for efficiency
        Ok(vec![])
    }

    async fn find_by_type(&self, event_type: &EventType) -> Result<Vec<Event>, RepositoryError> {
        Ok(vec![])
    }

    async fn find_in_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Event>, RepositoryError> {
        Ok(vec![])
    }
}
```

### Example: HTTP Handler (Interface Adapters Layer)

```rust
// src/infrastructure/web/handlers/event_handler.rs

use crate::application::use_cases::IngestEventUseCase;
use crate::application::dto::{IngestEventRequest, IngestEventResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

/// HTTP Handler for event ingestion
///
/// This adapts HTTP requests to use case inputs
pub struct EventHandler {
    ingest_use_case: Arc<IngestEventUseCase>,
}

impl EventHandler {
    pub fn new(ingest_use_case: Arc<IngestEventUseCase>) -> Self {
        Self { ingest_use_case }
    }

    /// Handle POST /events
    pub async fn ingest_event(
        State(handler): State<Arc<EventHandler>>,
        Json(request): Json<IngestEventRequest>,
    ) -> Result<impl IntoResponse, AppError> {
        // 1. Call use case (no business logic here!)
        let response = handler
            .ingest_use_case
            .execute(request)
            .await
            .map_err(AppError::from)?;

        // 2. Convert response to HTTP format
        Ok((StatusCode::CREATED, Json(response)))
    }
}

// Error handling for HTTP layer
#[derive(Debug)]
pub struct AppError(String);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}
```

### Example: Dependency Injection (Frameworks & Drivers)

```rust
// src/config/dependency_injection.rs

use crate::domain::repositories::EventRepository;
use crate::infrastructure::persistence::parquet::ParquetEventRepository;
use crate::application::use_cases::IngestEventUseCase;
use crate::infrastructure::web::handlers::EventHandler;
use std::sync::Arc;

/// Application container holds all dependencies
pub struct AppContainer {
    pub event_repository: Arc<dyn EventRepository>,
    pub ingest_use_case: Arc<IngestEventUseCase>,
    pub event_handler: Arc<EventHandler>,
}

impl AppContainer {
    /// Wire up all dependencies
    pub fn new(config: AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Create infrastructure (outermost layer)
        let event_repository: Arc<dyn EventRepository> = Arc::new(
            ParquetEventRepository::new(config.data_dir)?
        );

        // 2. Create use cases (application layer)
        let ingest_use_case = Arc::new(
            IngestEventUseCase::new(event_repository.clone())
        );

        // 3. Create handlers (adapter layer)
        let event_handler = Arc::new(
            EventHandler::new(ingest_use_case.clone())
        );

        Ok(Self {
            event_repository,
            ingest_use_case,
            event_handler,
        })
    }
}

pub struct AppConfig {
    pub data_dir: std::path::PathBuf,
}
```

### Example: Main Entry Point

```rust
// src/lib.rs

mod domain;
mod application;
mod infrastructure;
mod config;

use config::{AppContainer, AppConfig};
use axum::{Router, routing::post};
use std::sync::Arc;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load configuration
    let config = AppConfig {
        data_dir: std::path::PathBuf::from("./data"),
    };

    // 2. Build dependency container
    let container = Arc::new(AppContainer::new(config)?);

    // 3. Setup routes
    let app = Router::new()
        .route("/events", post(infrastructure::web::handlers::event_handler::EventHandler::ingest_event))
        .with_state(container.event_handler.clone());

    // 4. Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## Go Implementation

### Directory Structure

```
cmd/
└── control-plane/
    └── main.go               # Entry point

internal/
├── domain/                   # Layer 1: Domain
│   ├── entities/
│   │   ├── user.go
│   │   ├── tenant.go
│   │   └── audit_event.go
│   ├── valueobjects/
│   │   ├── user_id.go
│   │   └── role.go
│   └── repositories/        # Interfaces
│       ├── user_repository.go
│       ├── tenant_repository.go
│       └── audit_repository.go
│
├── application/              # Layer 2: Application
│   ├── usecases/
│   │   ├── authenticate_user.go
│   │   ├── authorize_request.go
│   │   └── manage_tenant.go
│   ├── dto/
│   │   ├── auth_dto.go
│   │   └── tenant_dto.go
│   └── ports/               # Interfaces for infrastructure
│       ├── auth_port.go
│       └── notification_port.go
│
├── infrastructure/           # Layer 3: Adapters
│   ├── persistence/
│   │   ├── file_audit_repository.go
│   │   └── postgres_user_repository.go
│   ├── web/
│   │   ├── handlers/
│   │   │   ├── auth_handler.go
│   │   │   └── tenant_handler.go
│   │   ├── middleware/
│   │   │   ├── auth_middleware.go
│   │   │   └── logging_middleware.go
│   │   └── router.go
│   ├── clients/
│   │   └── rust_core_client.go
│   └── config/
│       └── config.go
│
└── pkg/                      # Shared utilities
    └── errors/
        └── errors.go
```

### Example: User Entity (Domain)

```go
// internal/domain/entities/user.go

package entities

import (
    "github.com/allsource/control-plane/internal/domain/valueobjects"
    "time"
)

// User is a domain entity representing a system user
// Pure domain logic with no infrastructure dependencies
type User struct {
    ID           valueobjects.UserID
    Username     string
    Email        string
    PasswordHash string
    Role         valueobjects.Role
    TenantID     string
    CreatedAt    time.Time
    UpdatedAt    time.Time
}

// NewUser creates a new user with validation
func NewUser(
    username string,
    email string,
    passwordHash string,
    role valueobjects.Role,
    tenantID string,
) (*User, error) {
    // Domain validation
    if username == "" {
        return nil, ErrInvalidUsername
    }

    if email == "" {
        return nil, ErrInvalidEmail
    }

    if tenantID == "" {
        return nil, ErrInvalidTenantID
    }

    return &User{
        ID:           valueobjects.GenerateUserID(),
        Username:     username,
        Email:        email,
        PasswordHash: passwordHash,
        Role:         role,
        TenantID:     tenantID,
        CreatedAt:    time.Now().UTC(),
        UpdatedAt:    time.Now().UTC(),
    }, nil
}

// BelongsToTenant checks if user belongs to tenant
func (u *User) BelongsToTenant(tenantID string) bool {
    return u.TenantID == tenantID
}

// HasRole checks if user has specific role
func (u *User) HasRole(role valueobjects.Role) bool {
    return u.Role == role
}

// Domain errors
var (
    ErrInvalidUsername  = errors.New("invalid username")
    ErrInvalidEmail     = errors.New("invalid email")
    ErrInvalidTenantID  = errors.New("invalid tenant ID")
)
```

### Example: Repository Interface (Domain)

```go
// internal/domain/repositories/user_repository.go

package repositories

import (
    "context"
    "github.com/allsource/control-plane/internal/domain/entities"
    "github.com/allsource/control-plane/internal/domain/valueobjects"
)

// UserRepository defines contract for user persistence
// This is an abstraction - domain defines WHAT, not HOW
type UserRepository interface {
    Save(ctx context.Context, user *entities.User) error
    FindByID(ctx context.Context, id valueobjects.UserID) (*entities.User, error)
    FindByUsername(ctx context.Context, username string) (*entities.User, error)
    FindByEmail(ctx context.Context, email string) (*entities.User, error)
    FindAll(ctx context.Context) ([]*entities.User, error)
    Delete(ctx context.Context, id valueobjects.UserID) error
}
```

### Example: Use Case (Application)

```go
// internal/application/usecases/authenticate_user.go

package usecases

import (
    "context"
    "github.com/allsource/control-plane/internal/domain/entities"
    "github.com/allsource/control-plane/internal/domain/repositories"
    "github.com/allsource/control-plane/internal/application/dto"
    "golang.org/x/crypto/bcrypt"
)

// AuthenticateUser is a use case for user authentication
// Application logic that orchestrates domain and infrastructure
type AuthenticateUser struct {
    userRepo repositories.UserRepository
    jwtSigner JWTSigner // Port (interface)
}

// JWTSigner is a port (interface) for infrastructure
type JWTSigner interface {
    Sign(user *entities.User) (string, error)
}

func NewAuthenticateUser(
    userRepo repositories.UserRepository,
    jwtSigner JWTSigner,
) *AuthenticateUser {
    return &AuthenticateUser{
        userRepo:  userRepo,
        jwtSigner: jwtSigner,
    }
}

// Execute runs the authentication use case
func (uc *AuthenticateUser) Execute(
    ctx context.Context,
    req dto.AuthenticateRequest,
) (*dto.AuthenticateResponse, error) {
    // 1. Find user by username
    user, err := uc.userRepo.FindByUsername(ctx, req.Username)
    if err != nil {
        return nil, ErrInvalidCredentials
    }

    // 2. Verify password (domain logic)
    if err := bcrypt.CompareHashAndPassword(
        []byte(user.PasswordHash),
        []byte(req.Password),
    ); err != nil {
        return nil, ErrInvalidCredentials
    }

    // 3. Generate JWT token (via port)
    token, err := uc.jwtSigner.Sign(user)
    if err != nil {
        return nil, err
    }

    // 4. Return response
    return &dto.AuthenticateResponse{
        Token:    token,
        UserID:   user.ID.String(),
        Username: user.Username,
        Role:     user.Role.String(),
    }, nil
}

var ErrInvalidCredentials = errors.New("invalid credentials")
```

### Example: Repository Implementation (Infrastructure)

```go
// internal/infrastructure/persistence/postgres_user_repository.go

package persistence

import (
    "context"
    "database/sql"
    "github.com/allsource/control-plane/internal/domain/entities"
    "github.com/allsource/control-plane/internal/domain/repositories"
    "github.com/allsource/control-plane/internal/domain/valueobjects"
)

// PostgresUserRepository implements UserRepository using PostgreSQL
// Infrastructure code that implements domain contract
type PostgresUserRepository struct {
    db *sql.DB
}

func NewPostgresUserRepository(db *sql.DB) *PostgresUserRepository {
    return &PostgresUserRepository{db: db}
}

// Implement interface
var _ repositories.UserRepository = (*PostgresUserRepository)(nil)

func (r *PostgresUserRepository) Save(
    ctx context.Context,
    user *entities.User,
) error {
    query := `
        INSERT INTO users (id, username, email, password_hash, role, tenant_id, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (id) DO UPDATE
        SET username = $2, email = $3, password_hash = $4, role = $5, updated_at = $8
    `

    _, err := r.db.ExecContext(
        ctx,
        query,
        user.ID.String(),
        user.Username,
        user.Email,
        user.PasswordHash,
        user.Role.String(),
        user.TenantID,
        user.CreatedAt,
        user.UpdatedAt,
    )

    return err
}

func (r *PostgresUserRepository) FindByID(
    ctx context.Context,
    id valueobjects.UserID,
) (*entities.User, error) {
    query := `
        SELECT id, username, email, password_hash, role, tenant_id, created_at, updated_at
        FROM users
        WHERE id = $1
    `

    var user entities.User
    var roleStr string

    err := r.db.QueryRowContext(ctx, query, id.String()).Scan(
        &user.ID,
        &user.Username,
        &user.Email,
        &user.PasswordHash,
        &roleStr,
        &user.TenantID,
        &user.CreatedAt,
        &user.UpdatedAt,
    )

    if err == sql.ErrNoRows {
        return nil, repositories.ErrNotFound
    }

    if err != nil {
        return nil, err
    }

    user.Role, err = valueobjects.ParseRole(roleStr)
    if err != nil {
        return nil, err
    }

    return &user, nil
}

func (r *PostgresUserRepository) FindByUsername(
    ctx context.Context,
    username string,
) (*entities.User, error) {
    // Similar to FindByID
    return nil, nil
}

func (r *PostgresUserRepository) FindByEmail(
    ctx context.Context,
    email string,
) (*entities.User, error) {
    return nil, nil
}

func (r *PostgresUserRepository) FindAll(
    ctx context.Context,
) ([]*entities.User, error) {
    return nil, nil
}

func (r *PostgresUserRepository) Delete(
    ctx context.Context,
    id valueobjects.UserID,
) error {
    return nil
}
```

### Example: HTTP Handler (Interface Adapters)

```go
// internal/infrastructure/web/handlers/auth_handler.go

package handlers

import (
    "encoding/json"
    "net/http"
    "github.com/allsource/control-plane/internal/application/usecases"
    "github.com/allsource/control-plane/internal/application/dto"
)

// AuthHandler adapts HTTP to use cases
type AuthHandler struct {
    authenticateUC *usecases.AuthenticateUser
}

func NewAuthHandler(authenticateUC *usecases.AuthenticateUser) *AuthHandler {
    return &AuthHandler{authenticateUC: authenticateUC}
}

// Login handles POST /api/v1/auth/login
func (h *AuthHandler) Login(w http.ResponseWriter, r *http.Request) {
    // 1. Parse HTTP request
    var req dto.AuthenticateRequest
    if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
        http.Error(w, "invalid request", http.StatusBadRequest)
        return
    }

    // 2. Call use case (no business logic here!)
    resp, err := h.authenticateUC.Execute(r.Context(), req)
    if err != nil {
        http.Error(w, err.Error(), http.StatusUnauthorized)
        return
    }

    // 3. Convert response to HTTP
    w.Header().Set("Content-Type", "application/json")
    w.WriteStatus(http.StatusOK)
    json.NewEncoder(w).Encode(resp)
}
```

### Example: Dependency Injection with Wire

```go
// cmd/control-plane/wire.go
// +build wireinject

package main

import (
    "github.com/google/wire"
    "github.com/allsource/control-plane/internal/domain/repositories"
    "github.com/allsource/control-plane/internal/application/usecases"
    "github.com/allsource/control-plane/internal/infrastructure/persistence"
    "github.com/allsource/control-plane/internal/infrastructure/web/handlers"
)

// Wire providers
var infrastructureSet = wire.NewSet(
    persistence.NewPostgresUserRepository,
    wire.Bind(new(repositories.UserRepository), new(*persistence.PostgresUserRepository)),
)

var applicationSet = wire.NewSet(
    usecases.NewAuthenticateUser,
)

var handlerSet = wire.NewSet(
    handlers.NewAuthHandler,
)

// InitializeApp wires everything together
func InitializeApp(config *Config) (*App, error) {
    wire.Build(
        infrastructureSet,
        applicationSet,
        handlerSet,
        NewApp,
    )
    return &App{}, nil
}
```

---

## Clojure Implementation

### Directory Structure

```
src/allsource/
├── domain/                   # Layer 1: Domain
│   ├── entities/
│   │   ├── event.clj
│   │   └── tenant.clj
│   ├── protocols/           # Like interfaces in OOP
│   │   ├── event_repository.clj
│   │   └── query_engine.clj
│   └── services/
│       └── query_service.clj
│
├── application/              # Layer 2: Application
│   ├── use_cases/
│   │   ├── execute_query.clj
│   │   └── build_projection.clj
│   └── handlers/            # Ring handlers
│       ├── query_handler.clj
│       └── projection_handler.clj
│
├── infrastructure/           # Layer 3: Adapters
│   ├── adapters/
│   │   ├── http_client.clj
│   │   └── postgres_repo.clj
│   ├── web/
│   │   ├── routes.clj
│   │   └── middleware.clj
│   └── config/
│       └── system.clj        # Component system
│
└── utils/
    └── logging.clj
```

### Example: Event Entity (Domain)

```clojure
;; src/allsource/domain/entities/event.clj

(ns allsource.domain.entities.event
  (:require [clojure.spec.alpha :as s]
            [java-time :as time]))

;; Specs define the shape of our domain entities (validation)
(s/def ::id uuid?)
(s/def ::event-type string?)
(s/def ::entity-id string?)
(s/def ::tenant-id string?)
(s/def ::payload map?)
(s/def ::timestamp inst?)
(s/def ::version pos-int?)

(s/def ::event
  (s/keys :req-un [::id ::event-type ::entity-id ::tenant-id ::payload ::timestamp ::version]
          :opt-un [::metadata]))

;; Event entity constructor with validation
(defn new-event
  "Create a new event with domain validation"
  [{:keys [event-type entity-id tenant-id payload]}]
  (let [event {:id (java.util.UUID/randomUUID)
               :event-type event-type
               :entity-id entity-id
               :tenant-id tenant-id
               :payload payload
               :timestamp (java.util.Date.)
               :version 1}]
    ;; Validate against spec
    (if (s/valid? ::event event)
      event
      (throw (ex-info "Invalid event"
                      {:errors (s/explain-data ::event event)})))))

;; Domain behaviors (pure functions)
(defn belongs-to-tenant?
  "Check if event belongs to tenant"
  [event tenant-id]
  (= (:tenant-id event) tenant-id))

(defn older-than?
  "Check if event is older than duration"
  [event duration-ms]
  (> (- (System/currentTimeMillis)
        (.getTime (:timestamp event)))
     duration-ms))

(defn with-metadata
  "Add metadata to event"
  [event metadata]
  (assoc event :metadata metadata))
```

### Example: Repository Protocol (Domain)

```clojure
;; src/allsource/domain/protocols/event_repository.clj

(ns allsource.domain.protocols.event-repository)

;; Protocol defines the contract (like interface in OOP)
;; Domain defines WHAT operations it needs, not HOW
(defprotocol EventRepository
  "Contract for event persistence"

  (save-event [this event]
    "Save a single event, returns event with id")

  (save-events [this events]
    "Save multiple events in batch, returns events")

  (find-by-id [this event-id]
    "Find event by ID, returns event or nil")

  (find-by-entity [this entity-id]
    "Find all events for entity, returns vector of events")

  (find-by-type [this event-type]
    "Find events by type, returns vector of events")

  (find-in-range [this start-time end-time]
    "Find events in time range, returns vector of events"))
```

### Example: Use Case (Application)

```clojure
;; src/allsource/application/use_cases/execute_query.clj

(ns allsource.application.use-cases.execute-query
  (:require [allsource.domain.protocols.event-repository :as repo]
            [allsource.domain.entities.event :as event]
            [clojure.spec.alpha :as s]))

;; Input/output specs for use case
(s/def ::query-request
  (s/keys :req-un [::event-type]
          :opt-un [::start-time ::end-time ::limit]))

(s/def ::query-response
  (s/keys :req-un [::events ::count]))

;; Use case orchestrates domain logic
(defn execute
  "Execute a query use case"
  [event-repository request]
  {:pre [(s/valid? ::query-request request)]}

  ;; 1. Validate input (application-level)
  (when (and (:start-time request)
             (:end-time request)
             (> (:start-time request) (:end-time request)))
    (throw (ex-info "Invalid time range"
                    {:start (:start-time request)
                     :end (:end-time request)})))

  ;; 2. Query via repository abstraction
  (let [events (if (and (:start-time request) (:end-time request))
                 (repo/find-in-range event-repository
                                     (:start-time request)
                                     (:end-time request))
                 (repo/find-by-type event-repository
                                    (:event-type request)))

        ;; 3. Filter and limit (application logic)
        filtered (filter #(event/belongs-to-tenant? % (:tenant-id request))
                         events)

        limited (if-let [limit (:limit request)]
                  (take limit filtered)
                  filtered)]

    ;; 4. Return response
    {:events (vec limited)
     :count (count limited)}))
```

### Example: Repository Implementation (Infrastructure)

```clojure
;; src/allsource/infrastructure/adapters/postgres_repo.clj

(ns allsource.infrastructure.adapters.postgres-repo
  (:require [allsource.domain.protocols.event-repository :as repo]
            [next.jdbc :as jdbc]
            [next.jdbc.sql :as sql]
            [clojure.data.json :as json]))

;; Record implements the protocol
(defrecord PostgresEventRepository [datasource]
  repo/EventRepository

  (save-event [this event]
    ;; PostgreSQL-specific save
    (let [row {:id (str (:id event))
               :event_type (:event-type event)
               :entity_id (:entity-id event)
               :tenant_id (:tenant-id event)
               :payload (json/write-str (:payload event))
               :timestamp (:timestamp event)
               :version (:version event)}]
      (sql/insert! datasource :events row))
    event)

  (save-events [this events]
    ;; Batch insert for PostgreSQL
    (jdbc/with-transaction [tx datasource]
      (doseq [event events]
        (repo/save-event (->PostgresEventRepository tx) event)))
    events)

  (find-by-id [this event-id]
    ;; Query PostgreSQL
    (when-let [row (sql/get-by-id datasource :events (str event-id))]
      {:id (java.util.UUID/fromString (:id row))
       :event-type (:event_type row)
       :entity-id (:entity_id row)
       :tenant-id (:tenant_id row)
       :payload (json/read-str (:payload row) :key-fn keyword)
       :timestamp (:timestamp row)
       :version (:version row)}))

  (find-by-entity [this entity-id]
    ;; Query with WHERE clause
    (let [rows (jdbc/execute! datasource
                              ["SELECT * FROM events WHERE entity_id = ?" entity-id])]
      (mapv row->event rows)))

  (find-by-type [this event-type]
    (let [rows (jdbc/execute! datasource
                              ["SELECT * FROM events WHERE event_type = ?" event-type])]
      (mapv row->event rows)))

  (find-in-range [this start-time end-time]
    (let [rows (jdbc/execute! datasource
                              ["SELECT * FROM events WHERE timestamp BETWEEN ? AND ?"
                               start-time end-time])]
      (mapv row->event rows))))

;; Helper to convert DB row to domain entity
(defn- row->event [row]
  {:id (java.util.UUID/fromString (:id row))
   :event-type (:event_type row)
   :entity-id (:entity_id row)
   :tenant-id (:tenant_id row)
   :payload (json/read-str (:payload row) :key-fn keyword)
   :timestamp (:timestamp row)
   :version (:version row)})

;; Constructor
(defn new-postgres-repository [datasource]
  (->PostgresEventRepository datasource))
```

### Example: HTTP Handler (Interface Adapters)

```clojure
;; src/allsource/application/handlers/query_handler.clj

(ns allsource.application.handlers.query-handler
  (:require [allsource.application.use-cases.execute-query :as query-uc]
            [ring.util.response :as response]))

;; Handler adapts HTTP to use case
(defn query-events
  "Handle POST /api/v1/query"
  [event-repository]
  (fn [request]
    (try
      ;; 1. Parse HTTP body
      (let [body (:body-params request)

            ;; 2. Call use case (no business logic here!)
            result (query-uc/execute event-repository body)]

        ;; 3. Convert to HTTP response
        (response/response result))

      (catch clojure.lang.ExceptionInfo e
        ;; Handle validation errors
        (response/bad-request {:error (.getMessage e)
                               :details (ex-data e)})))))
```

### Example: Component System (Frameworks & Drivers)

```clojure
;; src/allsource/infrastructure/config/system.clj

(ns allsource.infrastructure.config.system
  (:require [com.stuartsierra.component :as component]
            [next.jdbc :as jdbc]
            [allsource.infrastructure.adapters.postgres-repo :as postgres]
            [allsource.infrastructure.web.routes :as routes]))

;; Component for database connection
(defrecord Database [config datasource]
  component/Lifecycle

  (start [this]
    (println "Starting database connection")
    (let [ds (jdbc/get-datasource (:db config))]
      (assoc this :datasource ds)))

  (stop [this]
    (println "Stopping database connection")
    ;; Close connections
    (dissoc this :datasource)))

;; Component for event repository
(defrecord EventRepositoryComponent [database repository]
  component/Lifecycle

  (start [this]
    (println "Starting event repository")
    (let [repo (postgres/new-postgres-repository (:datasource database))]
      (assoc this :repository repo)))

  (stop [this]
    (println "Stopping event repository")
    (dissoc this :repository)))

;; Component for web server
(defrecord WebServer [config event-repository server]
  component/Lifecycle

  (start [this]
    (println "Starting web server")
    (let [app (routes/app (:repository event-repository))
          server (start-server app (:port config))]
      (assoc this :server server)))

  (stop [this]
    (println "Stopping web server")
    (when server
      (stop-server server))
    (dissoc this :server)))

;; System composition - wire dependencies
(defn system [config]
  (component/system-map
    :config config

    :database (Database. config nil)

    :event-repository (component/using
                        (EventRepositoryComponent. nil nil)
                        [:database])

    :web-server (component/using
                  (WebServer. config nil nil)
                  [:event-repository])))

;; Start the system
(defn start! [config]
  (component/start (system config)))

;; Stop the system
(defn stop! [system]
  (component/stop system))
```

---

## Best Practices

### 1. Keep Domain Pure

**✅ DO**:
```rust
// Domain entity with pure logic
impl Event {
    pub fn is_valid(&self) -> bool {
        !self.entity_id.is_empty() && !self.tenant_id.is_empty()
    }
}
```

**❌ DON'T**:
```rust
// Domain entity with infrastructure dependency
impl Event {
    pub async fn save_to_database(&self, db: &Database) -> Result<()> {
        db.execute("INSERT INTO events...").await
    }
}
```

### 2. Use Dependency Injection

**✅ DO**:
```rust
pub struct EventService {
    repository: Arc<dyn EventRepository>,  // Inject abstraction
}
```

**❌ DON'T**:
```rust
pub struct EventService {
    repository: ParquetRepository,  // Hard-coded concrete type
}
```

### 3. Define Interfaces in Inner Layers

**✅ DO**:
```rust
// domain/repositories/event_repository.rs
pub trait EventRepository {
    async fn save(&self, event: Event) -> Result<()>;
}

// infrastructure/persistence/parquet_repository.rs
impl EventRepository for ParquetRepository {
    async fn save(&self, event: Event) -> Result<()> { /* ... */ }
}
```

**❌ DON'T**:
```rust
// infrastructure/persistence/repository_interface.rs (in wrong layer!)
pub trait EventRepository {
    async fn save(&self, event: Event) -> Result<()>;
}
```

### 4. Keep Use Cases Focused

**✅ DO**:
```rust
// One responsibility
pub struct IngestEventUseCase {
    event_repository: Arc<dyn EventRepository>,
}
```

**❌ DON'T**:
```rust
// Too many responsibilities
pub struct EventUseCase {
    event_repository: Arc<dyn EventRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
    notification_service: Arc<dyn NotificationService>,
    analytics_service: Arc<dyn AnalyticsService>,
    // ... too many!
}
```

### 5. Use DTOs for Boundaries

**✅ DO**:
```rust
// Application layer defines DTO
pub struct IngestEventRequest {
    pub event_type: String,
    pub entity_id: String,
    pub tenant_id: String,
    pub payload: serde_json::Value,
}

// Handler converts DTO to domain entity
let event = Event::new(request.event_type, ...)?;
```

**❌ DON'T**:
```rust
// Handler directly exposes domain entity
pub async fn ingest_event(Json(event): Json<Event>) { /* ... */ }
```

---

## Common Pitfalls

### Pitfall 1: Leaking Infrastructure into Domain

**Problem**:
```rust
// ❌ Domain entity depends on database
pub struct Event {
    #[sqlx(rename = "event_id")]  // Database-specific annotation
    pub id: Uuid,
}
```

**Solution**:
```rust
// ✅ Pure domain entity
pub struct Event {
    pub id: Uuid,  // No database annotations
}

// Database mapping in infrastructure layer
impl From<Event> for EventRow {
    fn from(event: Event) -> Self {
        EventRow {
            event_id: event.id,  // Map here
        }
    }
}
```

### Pitfall 2: Business Logic in Handlers

**Problem**:
```rust
// ❌ Handler contains business logic
pub async fn ingest_event(Json(request): Json<IngestRequest>) -> Result<Response> {
    // Validation logic in handler
    if request.entity_id.is_empty() {
        return Err("Invalid entity ID");
    }

    // Business logic in handler
    let event = Event { /* ... */ };

    // Direct database call in handler
    sqlx::query("INSERT INTO events...").execute(&pool).await?;
}
```

**Solution**:
```rust
// ✅ Handler delegates to use case
pub async fn ingest_event(
    State(use_case): State<Arc<IngestEventUseCase>>,
    Json(request): Json<IngestRequest>,
) -> Result<Response> {
    let response = use_case.execute(request).await?;
    Ok(Json(response))
}
```

### Pitfall 3: Circular Dependencies

**Problem**:
```
Domain → Application → Infrastructure → Domain  // ❌ Circular!
```

**Solution**:
```
Domain ← Application ← Infrastructure  // ✅ One direction only
```

---

## Migration Strategies

### Strategy 1: Strangler Fig Pattern

Gradually refactor existing code:

```rust
// Step 1: Create new clean architecture in parallel
// src/domain/ (new structure)
// src/legacy/ (old code)

// Step 2: New features use new structure
pub async fn ingest_event_v2(/* uses clean arch */) { }

// Step 3: Gradually migrate old code
// Legacy handler calls new use case
pub async fn ingest_event_legacy(request) {
    let new_request = convert_to_new_format(request);
    ingest_event_use_case.execute(new_request).await
}

// Step 4: Remove legacy code when migration complete
```

### Strategy 2: Feature-by-Feature Migration

```
Week 1: Migrate event ingestion to clean architecture
Week 2: Migrate query endpoints
Week 3: Migrate tenant management
...
```

---

## Testing Strategies

### Unit Testing (Domain Layer)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new(
            EventType::from("user.created"),
            "user-123".to_string(),
            "tenant-456".to_string(),
            serde_json::json!({"name": "John"}),
        ).unwrap();

        assert_eq!(event.entity_id, "user-123");
        assert!(event.belongs_to_tenant("tenant-456"));
    }

    #[test]
    fn test_invalid_event() {
        let result = Event::new(
            EventType::from("user.created"),
            "".to_string(),  // Invalid: empty
            "tenant-456".to_string(),
            serde_json::json!({}),
        );

        assert!(result.is_err());
    }
}
```

### Use Case Testing (Application Layer)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;

    // Mock repository
    mock! {
        EventRepository {}

        #[async_trait]
        impl EventRepository for EventRepository {
            async fn save(&self, event: Event) -> Result<(), RepositoryError>;
        }
    }

    #[tokio::test]
    async fn test_ingest_event_use_case() {
        // Arrange
        let mut mock_repo = MockEventRepository::new();
        mock_repo
            .expect_save()
            .times(1)
            .returning(|_| Ok(()));

        let use_case = IngestEventUseCase::new(Arc::new(mock_repo));
        let request = IngestEventRequest {
            event_type: "user.created".to_string(),
            entity_id: "user-123".to_string(),
            tenant_id: "tenant-456".to_string(),
            payload: serde_json::json!({}),
        };

        // Act
        let result = use_case.execute(request).await;

        // Assert
        assert!(result.is_ok());
    }
}
```

### Integration Testing (Infrastructure Layer)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parquet_repository_save_and_load() {
        // Real infrastructure test
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = ParquetEventRepository::new(temp_dir.path().to_path_buf()).unwrap();

        let event = Event::new(/* ... */).unwrap();

        // Save
        repo.save(event.clone()).await.unwrap();

        // Load
        let loaded = repo.find_by_id(&event.id).await.unwrap();

        assert_eq!(loaded.unwrap().id, event.id);
    }
}
```

---

## Summary

Clean Architecture provides:

✅ **Testability**: Mock infrastructure, test business logic in isolation
✅ **Flexibility**: Swap databases, frameworks without touching business logic
✅ **Maintainability**: Clear boundaries, single responsibility
✅ **Performance**: Optimizations don't affect architecture
✅ **Team Collaboration**: Multiple teams can work on different layers

**Key Principles**:
1. Dependencies point inward
2. Inner layers define interfaces
3. Outer layers implement interfaces
4. Business logic is framework-independent
5. Use dependency injection

---

**Further Reading**:
- Clean Architecture by Robert C. Martin
- Domain-Driven Design by Eric Evans
- Hexagonal Architecture by Alistair Cockburn

---

*This guide is part of the AllSource Event Store documentation. For questions or contributions, see [CONTRIBUTING.md](../CONTRIBUTING.md).*
