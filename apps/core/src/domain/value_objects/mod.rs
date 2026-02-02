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
/// Examples: TenantId, EventType, EntityId, PartitionKey, EventId, Version, Money, etc.

// Core event sourcing value objects
pub mod embedding_vector;
pub mod entity_id;
pub mod event_id;
pub mod event_type;
pub mod fork_id;
pub mod partition_key;
pub mod projection_name;
pub mod schema_subject;
pub mod stream_name;
pub mod tenant_id;
pub mod version;

// Paywall domain value objects
pub mod article_id;
pub mod creator_id;
pub mod money;
pub mod transaction_id;
pub mod wallet_address;

// Core re-exports
pub use embedding_vector::{DistanceMetric, EmbeddingVector, SimilarityScore};
pub use entity_id::EntityId;
pub use event_id::EventId;
pub use event_type::EventType;
pub use fork_id::ForkId;
pub use partition_key::PartitionKey;
pub use projection_name::ProjectionName;
pub use schema_subject::SchemaSubject;
pub use stream_name::StreamName;
pub use tenant_id::TenantId;
pub use version::Version;

// Paywall re-exports
pub use article_id::ArticleId;
pub use creator_id::CreatorId;
pub use money::{Currency, Money};
pub use transaction_id::TransactionId;
pub use wallet_address::WalletAddress;
