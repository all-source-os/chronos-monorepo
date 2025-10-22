use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::Result;

/// Domain Entity: Event
///
/// Core event structure representing a domain event in the event store.
/// This is an immutable, timestamped record of something that happened.
///
/// Domain Rules:
/// - Events are immutable once created
/// - Event type must follow naming convention (lowercase, dot-separated)
/// - Entity ID cannot be empty
/// - Tenant ID cannot be empty
/// - Timestamp must not be in the future
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub entity_id: String,
    #[serde(default = "default_tenant_id")]
    pub tenant_id: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
    pub version: i64,
}

fn default_tenant_id() -> String {
    "default".to_string()
}

impl Event {
    /// Create a new Event with full validation
    pub fn new_validated(
        event_type: String,
        entity_id: String,
        tenant_id: String,
        payload: serde_json::Value,
        metadata: Option<serde_json::Value>,
    ) -> Result<Self> {
        // Domain validation
        Self::validate_event_type(&event_type)?;
        Self::validate_entity_id(&entity_id)?;
        Self::validate_tenant_id(&tenant_id)?;

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

    /// Create a new Event without validation (for legacy compatibility)
    /// Use new_validated() for new code
    pub fn new(
        event_type: String,
        entity_id: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            entity_id,
            tenant_id: "default".to_string(),
            payload,
            timestamp: Utc::now(),
            metadata: None,
            version: 1,
        }
    }

    /// Create event with explicit tenant
    pub fn new_with_tenant(
        event_type: String,
        entity_id: String,
        payload: serde_json::Value,
        tenant_id: String,
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

    /// Reconstruct an Event from storage (bypasses validation for stored events)
    pub fn reconstruct(
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
            event_type,
            entity_id,
            tenant_id,
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

    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
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

    // Domain behavior methods

    /// Check if this event belongs to a specific tenant
    pub fn belongs_to_tenant(&self, tenant_id: &str) -> bool {
        self.tenant_id == tenant_id
    }

    /// Check if this event relates to a specific entity
    pub fn relates_to_entity(&self, entity_id: &str) -> bool {
        self.entity_id == entity_id
    }

    /// Check if this event is of a specific type
    pub fn is_type(&self, event_type: &str) -> bool {
        self.event_type == event_type
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

    // Private validation methods

    fn validate_event_type(event_type: &str) -> Result<()> {
        if event_type.is_empty() {
            return Err(crate::error::Error::InvalidInput(
                "Event type cannot be empty".to_string(),
            ));
        }

        // Event types should follow convention: lowercase with dots
        if !event_type.chars().all(|c| c.is_lowercase() || c == '.' || c == '_') {
            return Err(crate::error::Error::InvalidInput(
                format!("Event type '{}' must be lowercase with dots/underscores", event_type),
            ));
        }

        Ok(())
    }

    fn validate_entity_id(entity_id: &str) -> Result<()> {
        if entity_id.is_empty() {
            return Err(crate::error::Error::InvalidInput(
                "Entity ID cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_tenant_id(tenant_id: &str) -> Result<()> {
        if tenant_id.is_empty() {
            return Err(crate::error::Error::InvalidInput(
                "Tenant ID cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_creation_with_validation() {
        let event = Event::new_validated(
            "user.created".to_string(),
            "user-123".to_string(),
            "tenant-1".to_string(),
            json!({"name": "Alice"}),
            None,
        );
        assert!(event.is_ok());

        let event = event.unwrap();
        assert_eq!(event.event_type, "user.created");
        assert_eq!(event.entity_id, "user-123");
        assert_eq!(event.tenant_id, "tenant-1");
    }

    #[test]
    fn test_event_type_validation() {
        // Valid event types
        assert!(Event::new_validated(
            "user.created".to_string(),
            "e1".to_string(),
            "t1".to_string(),
            json!({}),
            None,
        ).is_ok());

        // Invalid: uppercase
        assert!(Event::new_validated(
            "User.Created".to_string(),
            "e1".to_string(),
            "t1".to_string(),
            json!({}),
            None,
        ).is_err());

        // Invalid: empty
        assert!(Event::new_validated(
            "".to_string(),
            "e1".to_string(),
            "t1".to_string(),
            json!({}),
            None,
        ).is_err());
    }

    #[test]
    fn test_entity_id_validation() {
        // Invalid: empty entity_id
        assert!(Event::new_validated(
            "user.created".to_string(),
            "".to_string(),
            "t1".to_string(),
            json!({}),
            None,
        ).is_err());
    }

    #[test]
    fn test_tenant_id_validation() {
        // Invalid: empty tenant_id
        assert!(Event::new_validated(
            "user.created".to_string(),
            "e1".to_string(),
            "".to_string(),
            json!({}),
            None,
        ).is_err());
    }

    #[test]
    fn test_domain_behavior_methods() {
        let event = Event::new_validated(
            "order.placed".to_string(),
            "order-456".to_string(),
            "tenant-1".to_string(),
            json!({"amount": 100}),
            None,
        ).unwrap();

        assert!(event.belongs_to_tenant("tenant-1"));
        assert!(!event.belongs_to_tenant("tenant-2"));

        assert!(event.relates_to_entity("order-456"));
        assert!(!event.relates_to_entity("order-789"));

        assert!(event.is_type("order.placed"));
        assert!(!event.is_type("order.cancelled"));
    }

    #[test]
    fn test_time_range_queries() {
        let event = Event::new_validated(
            "test.event".to_string(),
            "e1".to_string(),
            "t1".to_string(),
            json!({}),
            None,
        ).unwrap();

        let past = Utc::now() - chrono::Duration::hours(1);
        let future = Utc::now() + chrono::Duration::hours(1);

        assert!(event.occurred_after(past));
        assert!(event.occurred_before(future));
        assert!(event.occurred_between(past, future));
    }
}
