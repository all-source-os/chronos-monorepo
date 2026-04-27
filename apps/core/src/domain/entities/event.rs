use crate::{
    domain::value_objects::{EntityId, EventType, TenantId},
    error::Result,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Domain Entity: Event
///
/// Core event structure representing a domain event in the event store.
/// This is an immutable, timestamped record of something that happened.
///
/// Domain Rules:
/// - Events are immutable once created
/// - Event type must follow naming convention (enforced by EventType value object)
/// - Entity ID cannot be empty (enforced by EntityId value object)
/// - Tenant ID cannot be empty (enforced by TenantId value object)
/// - Timestamp must not be in the future
/// - Version starts at 1
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: Uuid,
    pub event_type: EventType,
    pub entity_id: EntityId,
    #[serde(default = "default_tenant_id")]
    pub tenant_id: TenantId,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
    pub version: i64,
}

fn default_tenant_id() -> TenantId {
    TenantId::default_tenant()
}

impl Event {
    /// Create a new Event with value objects (recommended)
    pub fn new(
        event_type: EventType,
        entity_id: EntityId,
        tenant_id: TenantId,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            entity_id,
            tenant_id,
            payload,
            timestamp: Utc::now(),
            metadata: None,
            version: 1,
        }
    }

    /// Create event with optional metadata
    pub fn with_metadata(
        event_type: EventType,
        entity_id: EntityId,
        tenant_id: TenantId,
        payload: serde_json::Value,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            entity_id,
            tenant_id,
            payload,
            timestamp: Utc::now(),
            metadata: Some(metadata),
            version: 1,
        }
    }

    /// Create event with default tenant (for single-tenant use)
    pub fn with_default_tenant(
        event_type: EventType,
        entity_id: EntityId,
        payload: serde_json::Value,
    ) -> Self {
        Self::new(event_type, entity_id, TenantId::default_tenant(), payload)
    }

    /// Create event from strings (for backward compatibility)
    ///
    /// This validates the strings and creates value objects.
    /// Use the value object constructor for new code.
    pub fn from_strings(
        event_type: String,
        entity_id: String,
        tenant_id: String,
        payload: serde_json::Value,
        metadata: Option<serde_json::Value>,
    ) -> Result<Self> {
        let event_type = EventType::new(event_type)?;
        let entity_id = EntityId::new(entity_id)?;
        let tenant_id = TenantId::new(tenant_id)?;

        Ok(Self {
            id: Uuid::new_v4(),
            event_type,
            entity_id,
            tenant_id,
            payload,
            timestamp: Utc::now(),
            metadata,
            version: 1,
        })
    }

    /// Reconstruct an Event from storage (bypasses validation for stored events)
    pub fn reconstruct(
        id: Uuid,
        event_type: EventType,
        entity_id: EntityId,
        tenant_id: TenantId,
        payload: serde_json::Value,
        timestamp: DateTime<Utc>,
        metadata: Option<serde_json::Value>,
        version: i64,
    ) -> Self {
        Self {
            id,
            event_type,
            entity_id,
            tenant_id,
            payload,
            timestamp,
            metadata,
            version,
        }
    }

    /// Reconstruct from raw strings (for loading from old storage)
    pub fn reconstruct_from_strings(
        id: Uuid,
        event_type: String,
        entity_id: String,
        tenant_id: String,
        payload: serde_json::Value,
        timestamp: DateTime<Utc>,
        metadata: Option<serde_json::Value>,
        version: i64,
    ) -> Self {
        Self {
            id,
            event_type: EventType::new_unchecked(event_type),
            entity_id: EntityId::new_unchecked(entity_id),
            tenant_id: TenantId::new_unchecked(tenant_id),
            payload,
            timestamp,
            metadata,
            version,
        }
    }

    // Getters (Events are immutable)

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn event_type(&self) -> &EventType {
        &self.event_type
    }

    pub fn event_type_str(&self) -> &str {
        self.event_type.as_str()
    }

    pub fn entity_id(&self) -> &EntityId {
        &self.entity_id
    }

    pub fn entity_id_str(&self) -> &str {
        self.entity_id.as_str()
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn tenant_id_str(&self) -> &str {
        self.tenant_id.as_str()
    }

    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    pub fn metadata(&self) -> Option<&serde_json::Value> {
        self.metadata.as_ref()
    }

    pub fn version(&self) -> i64 {
        self.version
    }

    /// Approximate resident size of this event in memory, in bytes.
    /// Used by the per-tenant memory cache (Step 3 of the
    /// sustainable data strategy) to make eviction decisions
    /// against a configurable byte budget.
    ///
    /// The estimator uses serialized-JSON payload length plus a
    /// fixed 256-byte overhead for the rest of the Event struct
    /// (UUID, event_type, entity_id, tenant_id, timestamp,
    /// version, plus DashMap and Vec slot overhead). It's
    /// intentionally cheap and order-of-magnitude correct rather
    /// than bytes-precise — the budget logic compares against a
    /// 2 GiB target, so an estimator that's off by 30% per event
    /// still produces a sensible cap.
    ///
    /// Cost: O(payload size) — one serde serialization per
    /// event. Called once per event on the cache append path,
    /// not on the query path.
    pub fn estimated_size_bytes(&self) -> u64 {
        const FIXED_OVERHEAD: u64 = 256;
        let payload_bytes = serde_json::to_vec(&self.payload)
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        let metadata_bytes = self
            .metadata
            .as_ref()
            .and_then(|m| serde_json::to_vec(m).ok())
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        FIXED_OVERHEAD + payload_bytes + metadata_bytes
    }

    // Domain behavior methods

    /// Check if this event belongs to a specific tenant
    pub fn belongs_to_tenant(&self, tenant_id: &TenantId) -> bool {
        &self.tenant_id == tenant_id
    }

    /// Check if this event belongs to a tenant (by string)
    pub fn belongs_to_tenant_str(&self, tenant_id: &str) -> bool {
        self.tenant_id.as_str() == tenant_id
    }

    /// Check if this event relates to a specific entity
    pub fn relates_to_entity(&self, entity_id: &EntityId) -> bool {
        &self.entity_id == entity_id
    }

    /// Check if this event relates to an entity (by string)
    pub fn relates_to_entity_str(&self, entity_id: &str) -> bool {
        self.entity_id.as_str() == entity_id
    }

    /// Check if this event is of a specific type
    pub fn is_type(&self, event_type: &EventType) -> bool {
        &self.event_type == event_type
    }

    /// Check if this event is of a type (by string)
    pub fn is_type_str(&self, event_type: &str) -> bool {
        self.event_type.as_str() == event_type
    }

    /// Check if this event is in a specific namespace
    pub fn is_in_namespace(&self, namespace: &str) -> bool {
        self.event_type.is_in_namespace(namespace)
    }

    /// Check if this event occurred within a time range
    pub fn occurred_between(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> bool {
        self.timestamp >= start && self.timestamp <= end
    }

    /// Check if event occurred before a specific time
    pub fn occurred_before(&self, time: DateTime<Utc>) -> bool {
        self.timestamp < time
    }

    /// Check if event occurred after a specific time
    pub fn occurred_after(&self, time: DateTime<Utc>) -> bool {
        self.timestamp > time
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_event_type() -> EventType {
        EventType::new("user.created".to_string()).unwrap()
    }

    fn test_entity_id() -> EntityId {
        EntityId::new("user-123".to_string()).unwrap()
    }

    fn test_tenant_id() -> TenantId {
        TenantId::new("tenant-1".to_string()).unwrap()
    }

    #[test]
    fn test_estimated_size_bytes_grows_with_payload() {
        // Two events, identical except payload size — the larger
        // payload must produce a larger estimate. Exact bytes
        // aren't specified; ordering is.
        let small = Event::new(
            test_event_type(),
            test_entity_id(),
            test_tenant_id(),
            json!({"k": "v"}),
        );
        let large_payload = json!({"data": "x".repeat(10_000)});
        let large = Event::new(
            test_event_type(),
            test_entity_id(),
            test_tenant_id(),
            large_payload,
        );

        assert!(small.estimated_size_bytes() < large.estimated_size_bytes());
        // The fixed overhead floors a tiny event at 256+ bytes;
        // the large one with a 10k-char string must clear 10k.
        assert!(small.estimated_size_bytes() >= 256);
        assert!(large.estimated_size_bytes() > 10_000);
    }

    #[test]
    fn test_estimated_size_bytes_includes_metadata() {
        let no_meta = Event::new(
            test_event_type(),
            test_entity_id(),
            test_tenant_id(),
            json!({"k": "v"}),
        );
        let with_meta = Event::with_metadata(
            test_event_type(),
            test_entity_id(),
            test_tenant_id(),
            json!({"k": "v"}),
            json!({"correlation_id": "abc-123-def-456-very-long"}),
        );
        assert!(with_meta.estimated_size_bytes() > no_meta.estimated_size_bytes());
    }

    #[test]
    fn test_event_creation_with_value_objects() {
        let event = Event::new(
            test_event_type(),
            test_entity_id(),
            test_tenant_id(),
            json!({"name": "Alice"}),
        );

        assert_eq!(event.event_type_str(), "user.created");
        assert_eq!(event.entity_id_str(), "user-123");
        assert_eq!(event.tenant_id_str(), "tenant-1");
        assert_eq!(event.version(), 1);
    }

    #[test]
    fn test_event_creation_from_strings() {
        let event = Event::from_strings(
            "user.created".to_string(),
            "user-123".to_string(),
            "tenant-1".to_string(),
            json!({"name": "Alice"}),
            None,
        );

        assert!(event.is_ok());
        let event = event.unwrap();
        assert_eq!(event.event_type_str(), "user.created");
        assert_eq!(event.entity_id_str(), "user-123");
        assert_eq!(event.tenant_id_str(), "tenant-1");
    }

    #[test]
    fn test_event_with_metadata() {
        let event = Event::with_metadata(
            test_event_type(),
            test_entity_id(),
            test_tenant_id(),
            json!({"name": "Bob"}),
            json!({"source": "api"}),
        );

        assert!(event.metadata().is_some());
        assert_eq!(event.metadata().unwrap(), &json!({"source": "api"}));
    }

    #[test]
    fn test_event_with_default_tenant() {
        let event = Event::with_default_tenant(test_event_type(), test_entity_id(), json!({}));

        assert_eq!(event.tenant_id_str(), "default");
    }

    #[test]
    fn test_from_strings_validates_event_type() {
        // Invalid: uppercase
        let result = Event::from_strings(
            "User.Created".to_string(),
            "e1".to_string(),
            "t1".to_string(),
            json!({}),
            None,
        );
        assert!(result.is_err());

        // Invalid: empty
        let result = Event::from_strings(
            String::new(),
            "e1".to_string(),
            "t1".to_string(),
            json!({}),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_from_strings_validates_entity_id() {
        // Invalid: empty entity_id
        let result = Event::from_strings(
            "user.created".to_string(),
            String::new(),
            "t1".to_string(),
            json!({}),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_from_strings_validates_tenant_id() {
        // Invalid: empty tenant_id
        let result = Event::from_strings(
            "user.created".to_string(),
            "e1".to_string(),
            String::new(),
            json!({}),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_belongs_to_tenant() {
        let tenant1 = TenantId::new("tenant-1".to_string()).unwrap();
        let tenant2 = TenantId::new("tenant-2".to_string()).unwrap();

        let event = Event::new(
            test_event_type(),
            test_entity_id(),
            tenant1.clone(),
            json!({}),
        );

        assert!(event.belongs_to_tenant(&tenant1));
        assert!(!event.belongs_to_tenant(&tenant2));
    }

    #[test]
    fn test_belongs_to_tenant_str() {
        let event = Event::new(
            test_event_type(),
            test_entity_id(),
            test_tenant_id(),
            json!({}),
        );

        assert!(event.belongs_to_tenant_str("tenant-1"));
        assert!(!event.belongs_to_tenant_str("tenant-2"));
    }

    #[test]
    fn test_relates_to_entity() {
        let entity1 = EntityId::new("order-456".to_string()).unwrap();
        let entity2 = EntityId::new("order-789".to_string()).unwrap();

        let event = Event::new(
            EventType::new("order.placed".to_string()).unwrap(),
            entity1.clone(),
            test_tenant_id(),
            json!({}),
        );

        assert!(event.relates_to_entity(&entity1));
        assert!(!event.relates_to_entity(&entity2));
    }

    #[test]
    fn test_relates_to_entity_str() {
        let event = Event::new(
            EventType::new("order.placed".to_string()).unwrap(),
            EntityId::new("order-456".to_string()).unwrap(),
            test_tenant_id(),
            json!({}),
        );

        assert!(event.relates_to_entity_str("order-456"));
        assert!(!event.relates_to_entity_str("order-789"));
    }

    #[test]
    fn test_is_type() {
        let type1 = EventType::new("order.placed".to_string()).unwrap();
        let type2 = EventType::new("order.cancelled".to_string()).unwrap();

        let event = Event::new(type1.clone(), test_entity_id(), test_tenant_id(), json!({}));

        assert!(event.is_type(&type1));
        assert!(!event.is_type(&type2));
    }

    #[test]
    fn test_is_type_str() {
        let event = Event::new(
            EventType::new("order.placed".to_string()).unwrap(),
            test_entity_id(),
            test_tenant_id(),
            json!({}),
        );

        assert!(event.is_type_str("order.placed"));
        assert!(!event.is_type_str("order.cancelled"));
    }

    #[test]
    fn test_is_in_namespace() {
        let event = Event::new(
            EventType::new("order.placed".to_string()).unwrap(),
            test_entity_id(),
            test_tenant_id(),
            json!({}),
        );

        assert!(event.is_in_namespace("order"));
        assert!(!event.is_in_namespace("user"));
    }

    #[test]
    fn test_time_range_queries() {
        let event = Event::new(
            test_event_type(),
            test_entity_id(),
            test_tenant_id(),
            json!({}),
        );

        let past = Utc::now() - chrono::Duration::hours(1);
        let future = Utc::now() + chrono::Duration::hours(1);

        assert!(event.occurred_after(past));
        assert!(event.occurred_before(future));
        assert!(event.occurred_between(past, future));
    }

    #[test]
    fn test_serde_serialization() {
        let event = Event::new(
            test_event_type(),
            test_entity_id(),
            test_tenant_id(),
            json!({"test": "data"}),
        );

        // Should be able to serialize
        let json = serde_json::to_string(&event);
        assert!(json.is_ok());

        // Should be able to deserialize
        let deserialized = serde_json::from_str::<Event>(&json.unwrap());
        assert!(deserialized.is_ok());

        let deserialized = deserialized.unwrap();
        assert_eq!(deserialized.event_type_str(), "user.created");
        assert_eq!(deserialized.entity_id_str(), "user-123");
    }

    #[test]
    fn test_reconstruct_from_strings() {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();

        let event = Event::reconstruct_from_strings(
            id,
            "order.placed".to_string(),
            "order-123".to_string(),
            "tenant-1".to_string(),
            json!({"amount": 100}),
            timestamp,
            Some(json!({"source": "api"})),
            1,
        );

        assert_eq!(event.id(), id);
        assert_eq!(event.event_type_str(), "order.placed");
        assert_eq!(event.entity_id_str(), "order-123");
        assert_eq!(event.tenant_id_str(), "tenant-1");
        assert_eq!(event.version(), 1);
    }
}
