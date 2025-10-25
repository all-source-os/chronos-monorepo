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
/// Examples: TenantId, EventType, EntityId, Money, Email, etc.

pub mod tenant_id;
pub mod event_type;
pub mod entity_id;

pub use tenant_id::TenantId;
pub use event_type::EventType;
pub use entity_id::EntityId;
