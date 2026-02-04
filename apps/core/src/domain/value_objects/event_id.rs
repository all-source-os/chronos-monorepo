use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Value Object: EventId
///
/// Represents a unique identifier for an event in the event store.
///
/// Domain Rules:
/// - Must be a valid UUID
/// - Immutable once created
/// - Globally unique within the event store
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(Uuid);

impl EventId {
    /// Create a new EventId with a random UUID
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::EventId;
    ///
    /// let event_id = EventId::new();
    /// assert!(!event_id.is_nil());
    /// ```
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an EventId from an existing UUID
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::EventId;
    /// use uuid::Uuid;
    ///
    /// let uuid = Uuid::new_v4();
    /// let event_id = EventId::from_uuid(uuid);
    /// assert_eq!(event_id.as_uuid(), uuid);
    /// ```
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse an EventId from a string
    ///
    /// # Errors
    /// Returns error if the string is not a valid UUID
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::EventId;
    ///
    /// let event_id = EventId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    /// assert_eq!(event_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    /// ```
    pub fn parse(value: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(value).map_err(|e| {
            crate::error::AllSourceError::InvalidInput(format!(
                "Invalid event ID '{}': {}",
                value, e
            ))
        })?;
        Ok(Self(uuid))
    }

    /// Get the UUID value
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Check if this is a nil (all zeros) UUID
    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }

    /// Create a nil EventId (for testing or placeholders)
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for EventId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<EventId> for Uuid {
    fn from(event_id: EventId) -> Self {
        event_id.0
    }
}

impl TryFrom<&str> for EventId {
    type Error = crate::error::AllSourceError;

    fn try_from(value: &str) -> Result<Self> {
        EventId::parse(value)
    }
}

impl TryFrom<String> for EventId {
    type Error = crate::error::AllSourceError;

    fn try_from(value: String) -> Result<Self> {
        EventId::parse(&value)
    }
}

impl AsRef<Uuid> for EventId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_event_id() {
        let event_id = EventId::new();
        assert!(!event_id.is_nil());
    }

    #[test]
    fn test_from_uuid() {
        let uuid = Uuid::new_v4();
        let event_id = EventId::from_uuid(uuid);
        assert_eq!(event_id.as_uuid(), uuid);
    }

    #[test]
    fn test_parse_valid_uuid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let event_id = EventId::parse(uuid_str);
        assert!(event_id.is_ok());
        assert_eq!(event_id.unwrap().to_string(), uuid_str);
    }

    #[test]
    fn test_parse_invalid_uuid() {
        let invalid = "not-a-uuid";
        let result = EventId::parse(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_nil_event_id() {
        let nil_id = EventId::nil();
        assert!(nil_id.is_nil());
        assert_eq!(nil_id.to_string(), "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn test_default_creates_new_uuid() {
        let id1 = EventId::default();
        let id2 = EventId::default();
        assert_ne!(id1, id2);
        assert!(!id1.is_nil());
    }

    #[test]
    fn test_display_trait() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let event_id = EventId::from_uuid(uuid);
        assert_eq!(
            format!("{}", event_id),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_from_uuid_trait() {
        let uuid = Uuid::new_v4();
        let event_id: EventId = uuid.into();
        assert_eq!(event_id.as_uuid(), uuid);
    }

    #[test]
    fn test_into_uuid_trait() {
        let event_id = EventId::new();
        let uuid: Uuid = event_id.into();
        assert_eq!(uuid, event_id.as_uuid());
    }

    #[test]
    fn test_try_from_str() {
        let event_id: Result<EventId> = "550e8400-e29b-41d4-a716-446655440000".try_into();
        assert!(event_id.is_ok());

        let invalid: Result<EventId> = "invalid".try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let event_id: Result<EventId> = "550e8400-e29b-41d4-a716-446655440000"
            .to_string()
            .try_into();
        assert!(event_id.is_ok());

        let invalid: Result<EventId> = String::new().try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_equality() {
        let uuid = Uuid::new_v4();
        let id1 = EventId::from_uuid(uuid);
        let id2 = EventId::from_uuid(uuid);
        let id3 = EventId::new();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_cloning() {
        let id1 = EventId::new();
        let id2 = id1; // Copy
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let uuid = Uuid::new_v4();
        let id1 = EventId::from_uuid(uuid);
        let id2 = EventId::from_uuid(uuid);

        let mut set = HashSet::new();
        set.insert(id1);

        assert!(set.contains(&id2));
    }

    #[test]
    fn test_serde_serialization() {
        let event_id = EventId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();

        // Serialize
        let json = serde_json::to_string(&event_id).unwrap();
        assert_eq!(json, "\"550e8400-e29b-41d4-a716-446655440000\"");

        // Deserialize
        let deserialized: EventId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, event_id);
    }

    #[test]
    fn test_as_ref() {
        let event_id = EventId::new();
        let uuid_ref: &Uuid = event_id.as_ref();
        assert_eq!(*uuid_ref, event_id.as_uuid());
    }
}
