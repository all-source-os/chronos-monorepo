// Core event sourcing repositories
pub mod audit_event_repository;
pub mod event_repository;
pub mod event_stream_repository;
pub mod tenant_repository;
pub mod vector_search_repository;

// Core re-exports
pub use audit_event_repository::{AuditEventQuery, AuditEventRepository};
pub use event_repository::{EventReader, EventRepository, EventWriter};
pub use event_stream_repository::{EventStreamReader, EventStreamRepository, EventStreamWriter};
pub use tenant_repository::{TenantQuery, TenantRepository};
pub use vector_search_repository::{
    SearchResult, VectorEntry, VectorSearchQuery, VectorSearchReader, VectorSearchRepository,
    VectorSearchWriter,
};
