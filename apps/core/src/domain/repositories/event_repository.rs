use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::domain::entities::Event;
use crate::error::Result;

/// Event Repository Trait (Domain Layer)
///
/// This is the abstract interface defined in the domain layer.
/// Concrete implementations live in the infrastructure layer.
/// This follows the Dependency Inversion Principle - the domain
/// defines what it needs, infrastructure provides it.
#[async_trait]
pub trait EventRepository: Send + Sync {
    /// Save a single event to the repository
    async fn save(&self, event: &Event) -> Result<()>;

    /// Save multiple events in a batch (atomic if possible)
    async fn save_batch(&self, events: &[Event]) -> Result<()>;

    /// Find an event by its unique ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Event>>;

    /// Find all events for a specific entity
    async fn find_by_entity(&self, entity_id: &str, tenant_id: &str) -> Result<Vec<Event>>;

    /// Find all events of a specific type
    async fn find_by_type(&self, event_type: &str, tenant_id: &str) -> Result<Vec<Event>>;

    /// Find events in a time range
    async fn find_by_time_range(
        &self,
        tenant_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Event>>;

    /// Find events for an entity up to a specific timestamp (time-travel)
    async fn find_by_entity_as_of(
        &self,
        entity_id: &str,
        tenant_id: &str,
        as_of: DateTime<Utc>,
    ) -> Result<Vec<Event>>;

    /// Count total events for a tenant
    async fn count(&self, tenant_id: &str) -> Result<usize>;

    /// Check if repository is healthy
    async fn health_check(&self) -> Result<()>;
}

/// Read-only event repository (for query optimization)
///
/// Following Interface Segregation Principle - separate read/write interfaces
#[async_trait]
pub trait EventReader: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Event>>;
    async fn find_by_entity(&self, entity_id: &str, tenant_id: &str) -> Result<Vec<Event>>;
    async fn find_by_type(&self, event_type: &str, tenant_id: &str) -> Result<Vec<Event>>;
    async fn count(&self, tenant_id: &str) -> Result<usize>;
}

/// Write-only event repository (for ingestion optimization)
#[async_trait]
pub trait EventWriter: Send + Sync {
    async fn save(&self, event: &Event) -> Result<()>;
    async fn save_batch(&self, events: &[Event]) -> Result<()>;
}
