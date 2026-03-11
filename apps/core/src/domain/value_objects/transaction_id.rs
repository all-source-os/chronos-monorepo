use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Value Object: TransactionId
///
/// Represents a unique identifier for a payment transaction in the paywall system.
///
/// Domain Rules:
/// - Must be a valid UUID
/// - Immutable once created
/// - Globally unique within the system
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(Uuid);

impl TransactionId {
    /// Create a new TransactionId with a random UUID
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::TransactionId;
    ///
    /// let transaction_id = TransactionId::new();
    /// assert!(!transaction_id.is_nil());
    /// ```
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a TransactionId from an existing UUID
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::TransactionId;
    /// use uuid::Uuid;
    ///
    /// let uuid = Uuid::new_v4();
    /// let transaction_id = TransactionId::from_uuid(uuid);
    /// assert_eq!(transaction_id.as_uuid(), uuid);
    /// ```
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse a TransactionId from a string
    ///
    /// # Errors
    /// Returns error if the string is not a valid UUID
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::TransactionId;
    ///
    /// let transaction_id = TransactionId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    /// assert_eq!(transaction_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    /// ```
    pub fn parse(value: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(value).map_err(|e| {
            crate::error::AllSourceError::InvalidInput(format!(
                "Invalid transaction ID '{value}': {e}"
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

    /// Create a nil TransactionId (for testing or placeholders)
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for TransactionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TransactionId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<TransactionId> for Uuid {
    fn from(transaction_id: TransactionId) -> Self {
        transaction_id.0
    }
}

impl TryFrom<&str> for TransactionId {
    type Error = crate::error::AllSourceError;

    fn try_from(value: &str) -> Result<Self> {
        TransactionId::parse(value)
    }
}

impl TryFrom<String> for TransactionId {
    type Error = crate::error::AllSourceError;

    fn try_from(value: String) -> Result<Self> {
        TransactionId::parse(&value)
    }
}

impl AsRef<Uuid> for TransactionId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_transaction_id() {
        let transaction_id = TransactionId::new();
        assert!(!transaction_id.is_nil());
    }

    #[test]
    fn test_from_uuid() {
        let uuid = Uuid::new_v4();
        let transaction_id = TransactionId::from_uuid(uuid);
        assert_eq!(transaction_id.as_uuid(), uuid);
    }

    #[test]
    fn test_parse_valid_uuid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let transaction_id = TransactionId::parse(uuid_str);
        assert!(transaction_id.is_ok());
        assert_eq!(transaction_id.unwrap().to_string(), uuid_str);
    }

    #[test]
    fn test_parse_invalid_uuid() {
        let invalid = "not-a-uuid";
        let result = TransactionId::parse(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_nil_transaction_id() {
        let nil_id = TransactionId::nil();
        assert!(nil_id.is_nil());
        assert_eq!(nil_id.to_string(), "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn test_default_creates_new_uuid() {
        let id1 = TransactionId::default();
        let id2 = TransactionId::default();
        assert_ne!(id1, id2);
        assert!(!id1.is_nil());
    }

    #[test]
    fn test_display_trait() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let transaction_id = TransactionId::from_uuid(uuid);
        assert_eq!(
            format!("{transaction_id}"),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_from_uuid_trait() {
        let uuid = Uuid::new_v4();
        let transaction_id: TransactionId = uuid.into();
        assert_eq!(transaction_id.as_uuid(), uuid);
    }

    #[test]
    fn test_into_uuid_trait() {
        let transaction_id = TransactionId::new();
        let uuid: Uuid = transaction_id.into();
        assert_eq!(uuid, transaction_id.as_uuid());
    }

    #[test]
    fn test_try_from_str() {
        let transaction_id: Result<TransactionId> =
            "550e8400-e29b-41d4-a716-446655440000".try_into();
        assert!(transaction_id.is_ok());

        let invalid: Result<TransactionId> = "invalid".try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let transaction_id: Result<TransactionId> = "550e8400-e29b-41d4-a716-446655440000"
            .to_string()
            .try_into();
        assert!(transaction_id.is_ok());

        let invalid: Result<TransactionId> = String::new().try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_equality() {
        let uuid = Uuid::new_v4();
        let id1 = TransactionId::from_uuid(uuid);
        let id2 = TransactionId::from_uuid(uuid);
        let id3 = TransactionId::new();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_cloning() {
        let id1 = TransactionId::new();
        let id2 = id1; // Copy
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let uuid = Uuid::new_v4();
        let id1 = TransactionId::from_uuid(uuid);
        let id2 = TransactionId::from_uuid(uuid);

        let mut set = HashSet::new();
        set.insert(id1);

        assert!(set.contains(&id2));
    }

    #[test]
    fn test_serde_serialization() {
        let transaction_id = TransactionId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();

        // Serialize
        let json = serde_json::to_string(&transaction_id).unwrap();
        assert_eq!(json, "\"550e8400-e29b-41d4-a716-446655440000\"");

        // Deserialize
        let deserialized: TransactionId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, transaction_id);
    }

    #[test]
    fn test_as_ref() {
        let transaction_id = TransactionId::new();
        let uuid_ref: &Uuid = transaction_id.as_ref();
        assert_eq!(*uuid_ref, transaction_id.as_uuid());
    }
}
