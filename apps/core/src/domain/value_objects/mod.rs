/// Value Objects Module
///
/// Value objects are immutable objects defined by their value, not their identity.
/// They encapsulate domain concepts and enforce business rules through validation.
///
/// Characteristics of value objects:
/// - Immutable
/// - Defined by value equality (not identity)
/// - Self-validating
/// - No lifecycle
/// - Can be freely copied/cloned
///
/// Examples: TenantId, EventType, EntityId, PartitionKey, Money, Email, etc.

// Core event sourcing value objects
pub mod embedding_vector;
pub mod entity_id;
pub mod event_type;
pub mod partition_key;
pub mod tenant_id;

// Core re-exports
pub use embedding_vector::{DistanceMetric, EmbeddingVector, SimilarityScore};
pub use entity_id::EntityId;
pub use event_type::EventType;
pub use partition_key::PartitionKey;
pub use tenant_id::TenantId;
