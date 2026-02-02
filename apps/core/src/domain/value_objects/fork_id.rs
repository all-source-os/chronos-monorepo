use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Value Object: ForkId
///
/// Represents a unique identifier for an event store fork.
/// Forks enable isolated experimentation for AI agents (Agentic Postgres pattern).
///
/// Domain Rules:
/// - Based on UUID v4 for uniqueness
/// - Immutable once created
/// - Can be parsed from string
///
/// # Agentic Postgres Pattern
/// Forks allow AI agents to:
/// - Create isolated copies of event streams for experimentation
/// - Try different approaches without affecting production data
/// - Compare outcomes before committing changes
/// - Safely rollback failed experiments
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ForkId(Uuid);

impl ForkId {
    /// Create a new random ForkId
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::ForkId;
    ///
    /// let fork_id = ForkId::new();
    /// assert!(!fork_id.as_str().is_empty());
    /// ```
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a ForkId from an existing UUID
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::ForkId;
    /// use uuid::Uuid;
    ///
    /// let uuid = Uuid::new_v4();
    /// let fork_id = ForkId::from_uuid(uuid);
    /// ```
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse a ForkId from a string
    ///
    /// # Errors
    /// Returns error if the string is not a valid UUID.
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::ForkId;
    ///
    /// let fork_id = ForkId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    /// ```
    pub fn parse(s: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(s).map_err(|e| {
            crate::error::AllSourceError::InvalidInput(format!("Invalid ForkId: {}", e))
        })?;
        Ok(Self(uuid))
    }

    /// Get the string representation
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Get the inner UUID (consumes self)
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for ForkId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ForkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for ForkId {
    type Error = crate::error::AllSourceError;

    fn try_from(value: &str) -> Result<Self> {
        ForkId::parse(value)
    }
}

impl TryFrom<String> for ForkId {
    type Error = crate::error::AllSourceError;

    fn try_from(value: String) -> Result<Self> {
        ForkId::parse(&value)
    }
}

impl From<Uuid> for ForkId {
    fn from(uuid: Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_fork_id() {
        let fork_id = ForkId::new();
        assert!(!fork_id.as_str().is_empty());
    }

    #[test]
    fn test_fork_id_uniqueness() {
        let id1 = ForkId::new();
        let id2 = ForkId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_parse_valid_uuid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let fork_id = ForkId::parse(uuid_str);
        assert!(fork_id.is_ok());
        assert_eq!(fork_id.unwrap().as_str(), uuid_str);
    }

    #[test]
    fn test_parse_invalid_uuid() {
        let result = ForkId::parse("not-a-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_uuid() {
        let uuid = Uuid::new_v4();
        let fork_id = ForkId::from_uuid(uuid);
        assert_eq!(fork_id.as_uuid(), &uuid);
    }

    #[test]
    fn test_display() {
        let fork_id = ForkId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(format!("{}", fork_id), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_equality() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id1 = ForkId::parse(uuid_str).unwrap();
        let id2 = ForkId::parse(uuid_str).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_try_from_str() {
        let fork_id: Result<ForkId> = "550e8400-e29b-41d4-a716-446655440000".try_into();
        assert!(fork_id.is_ok());
    }

    #[test]
    fn test_try_from_string() {
        let fork_id: Result<ForkId> = "550e8400-e29b-41d4-a716-446655440000".to_string().try_into();
        assert!(fork_id.is_ok());
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id1 = ForkId::parse(uuid_str).unwrap();
        let id2 = ForkId::parse(uuid_str).unwrap();

        let mut set = HashSet::new();
        set.insert(id1);
        assert!(set.contains(&id2));
    }

    #[test]
    fn test_serde_serialization() {
        let fork_id = ForkId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();

        // Serialize
        let json = serde_json::to_string(&fork_id).unwrap();
        assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));

        // Deserialize
        let deserialized: ForkId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, fork_id);
    }

    #[test]
    fn test_into_uuid() {
        let uuid = Uuid::new_v4();
        let fork_id = ForkId::from_uuid(uuid);
        let extracted = fork_id.into_uuid();
        assert_eq!(extracted, uuid);
    }

    #[test]
    fn test_default() {
        let fork_id = ForkId::default();
        assert!(!fork_id.as_str().is_empty());
    }
}
