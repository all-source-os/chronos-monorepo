use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Value Object: StreamName
///
/// Represents the name of an event stream in the event store.
/// Streams are logical groupings of events, typically by entity or aggregate.
///
/// Domain Rules:
/// - Cannot be empty
/// - Must be between 1 and 256 characters
/// - Must follow naming convention: alphanumeric with hyphens/underscores/colons
/// - Case-sensitive
/// - Immutable once created
///
/// Common patterns:
/// - `entity-type:entity-id` (e.g., "user:123", "order:abc-456")
/// - `aggregate-type-aggregate-id` (e.g., "user-123", "order-abc-456")
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamName(String);

impl StreamName {
    /// Create a new StreamName with validation
    ///
    /// # Errors
    /// Returns error if:
    /// - Name is empty
    /// - Name is longer than 256 characters
    /// - Name contains invalid characters
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::StreamName;
    ///
    /// let stream = StreamName::new("user:123".to_string()).unwrap();
    /// assert_eq!(stream.as_str(), "user:123");
    /// ```
    pub fn new(value: String) -> Result<Self> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    /// Create StreamName without validation (for internal use, e.g., from trusted storage)
    ///
    /// # Safety
    /// This bypasses validation. Only use when loading from trusted sources
    /// where validation has already occurred.
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    /// Create a StreamName from entity type and entity ID
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::StreamName;
    ///
    /// let stream = StreamName::for_entity("user", "123").unwrap();
    /// assert_eq!(stream.as_str(), "user:123");
    /// ```
    pub fn for_entity(entity_type: &str, entity_id: &str) -> Result<Self> {
        Self::new(format!("{}:{}", entity_type, entity_id))
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the inner String (consumes self)
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Extract entity type if stream follows "type:id" pattern
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::StreamName;
    ///
    /// let stream = StreamName::new("user:123".to_string()).unwrap();
    /// assert_eq!(stream.entity_type(), Some("user"));
    ///
    /// let stream = StreamName::new("simple-stream".to_string()).unwrap();
    /// assert_eq!(stream.entity_type(), None);
    /// ```
    pub fn entity_type(&self) -> Option<&str> {
        self.0.split(':').next().filter(|_| self.0.contains(':'))
    }

    /// Extract entity ID if stream follows "type:id" pattern
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::StreamName;
    ///
    /// let stream = StreamName::new("user:123".to_string()).unwrap();
    /// assert_eq!(stream.entity_id(), Some("123"));
    ///
    /// let stream = StreamName::new("simple-stream".to_string()).unwrap();
    /// assert_eq!(stream.entity_id(), None);
    /// ```
    pub fn entity_id(&self) -> Option<&str> {
        if self.0.contains(':') {
            self.0.splitn(2, ':').nth(1)
        } else {
            None
        }
    }

    /// Check if this stream is for a specific entity type
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::StreamName;
    ///
    /// let stream = StreamName::new("user:123".to_string()).unwrap();
    /// assert!(stream.is_entity_type("user"));
    /// assert!(!stream.is_entity_type("order"));
    /// ```
    pub fn is_entity_type(&self, entity_type: &str) -> bool {
        self.entity_type() == Some(entity_type)
    }

    /// Check if this stream name starts with a prefix
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }

    /// Validate a stream name string
    fn validate(value: &str) -> Result<()> {
        // Rule: Cannot be empty
        if value.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Stream name cannot be empty".to_string(),
            ));
        }

        // Rule: Maximum length 256 characters
        if value.len() > 256 {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Stream name cannot exceed 256 characters, got {}",
                value.len()
            )));
        }

        // Rule: Only alphanumeric, hyphens, underscores, and colons
        if !value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':')
        {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Stream name '{}' must be alphanumeric with hyphens, underscores, or colons",
                value
            )));
        }

        // Rule: Cannot start or end with special characters
        if value.starts_with(':')
            || value.starts_with('-')
            || value.starts_with('_')
            || value.ends_with(':')
            || value.ends_with('-')
            || value.ends_with('_')
        {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Stream name '{}' cannot start or end with special characters",
                value
            )));
        }

        // Rule: Cannot have consecutive special characters
        if value.contains("::")
            || value.contains("--")
            || value.contains("__")
            || value.contains(":-")
            || value.contains("-:")
        {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Stream name '{}' cannot have consecutive special characters",
                value
            )));
        }

        Ok(())
    }
}

impl fmt::Display for StreamName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for StreamName {
    type Error = crate::error::AllSourceError;

    fn try_from(value: &str) -> Result<Self> {
        StreamName::new(value.to_string())
    }
}

impl TryFrom<String> for StreamName {
    type Error = crate::error::AllSourceError;

    fn try_from(value: String) -> Result<Self> {
        StreamName::new(value)
    }
}

impl AsRef<str> for StreamName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_stream_names() {
        // Simple name
        let stream = StreamName::new("my-stream".to_string());
        assert!(stream.is_ok());
        assert_eq!(stream.unwrap().as_str(), "my-stream");

        // With colon (entity pattern)
        let stream = StreamName::new("user:123".to_string());
        assert!(stream.is_ok());

        // With underscore
        let stream = StreamName::new("order_stream".to_string());
        assert!(stream.is_ok());

        // Mixed
        let stream = StreamName::new("user_account:abc-123".to_string());
        assert!(stream.is_ok());

        // Just alphanumeric
        let stream = StreamName::new("stream123".to_string());
        assert!(stream.is_ok());
    }

    #[test]
    fn test_reject_empty_stream_name() {
        let result = StreamName::new("".to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn test_reject_too_long_stream_name() {
        let long_name = "a".repeat(257);
        let result = StreamName::new(long_name);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("cannot exceed 256 characters"));
        }
    }

    #[test]
    fn test_accept_max_length_stream_name() {
        let max_name = "a".repeat(256);
        let result = StreamName::new(max_name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_invalid_characters() {
        // Space
        let result = StreamName::new("stream name".to_string());
        assert!(result.is_err());

        // Dot
        let result = StreamName::new("stream.name".to_string());
        assert!(result.is_err());

        // Special characters
        let result = StreamName::new("stream@name".to_string());
        assert!(result.is_err());

        let result = StreamName::new("stream/name".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_starting_with_special_char() {
        let result = StreamName::new(":stream".to_string());
        assert!(result.is_err());

        let result = StreamName::new("-stream".to_string());
        assert!(result.is_err());

        let result = StreamName::new("_stream".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_ending_with_special_char() {
        let result = StreamName::new("stream:".to_string());
        assert!(result.is_err());

        let result = StreamName::new("stream-".to_string());
        assert!(result.is_err());

        let result = StreamName::new("stream_".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_consecutive_special_chars() {
        let result = StreamName::new("stream::id".to_string());
        assert!(result.is_err());

        let result = StreamName::new("stream--name".to_string());
        assert!(result.is_err());

        let result = StreamName::new("stream__name".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_for_entity() {
        let stream = StreamName::for_entity("user", "123");
        assert!(stream.is_ok());
        assert_eq!(stream.unwrap().as_str(), "user:123");
    }

    #[test]
    fn test_entity_type_extraction() {
        let stream = StreamName::new("user:123".to_string()).unwrap();
        assert_eq!(stream.entity_type(), Some("user"));

        let stream = StreamName::new("order:abc-456".to_string()).unwrap();
        assert_eq!(stream.entity_type(), Some("order"));

        // No colon
        let stream = StreamName::new("simple-stream".to_string()).unwrap();
        assert_eq!(stream.entity_type(), None);
    }

    #[test]
    fn test_entity_id_extraction() {
        let stream = StreamName::new("user:123".to_string()).unwrap();
        assert_eq!(stream.entity_id(), Some("123"));

        let stream = StreamName::new("order:abc-456".to_string()).unwrap();
        assert_eq!(stream.entity_id(), Some("abc-456"));

        // Multiple colons - take everything after first colon
        let stream = StreamName::new("complex:id:with:colons".to_string()).unwrap();
        assert_eq!(stream.entity_id(), Some("id:with:colons"));

        // No colon
        let stream = StreamName::new("simple-stream".to_string()).unwrap();
        assert_eq!(stream.entity_id(), None);
    }

    #[test]
    fn test_is_entity_type() {
        let stream = StreamName::new("user:123".to_string()).unwrap();
        assert!(stream.is_entity_type("user"));
        assert!(!stream.is_entity_type("order"));
    }

    #[test]
    fn test_starts_with() {
        let stream = StreamName::new("user:123".to_string()).unwrap();
        assert!(stream.starts_with("user"));
        assert!(stream.starts_with("user:"));
        assert!(!stream.starts_with("order"));
    }

    #[test]
    fn test_display_trait() {
        let stream = StreamName::new("user:123".to_string()).unwrap();
        assert_eq!(format!("{}", stream), "user:123");
    }

    #[test]
    fn test_try_from_str() {
        let stream: Result<StreamName> = "user:123".try_into();
        assert!(stream.is_ok());
        assert_eq!(stream.unwrap().as_str(), "user:123");

        let invalid: Result<StreamName> = "".try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let stream: Result<StreamName> = "order:456".to_string().try_into();
        assert!(stream.is_ok());

        let invalid: Result<StreamName> = String::new().try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_into_inner() {
        let stream = StreamName::new("test-stream".to_string()).unwrap();
        let inner = stream.into_inner();
        assert_eq!(inner, "test-stream");
    }

    #[test]
    fn test_equality() {
        let stream1 = StreamName::new("user:123".to_string()).unwrap();
        let stream2 = StreamName::new("user:123".to_string()).unwrap();
        let stream3 = StreamName::new("order:456".to_string()).unwrap();

        assert_eq!(stream1, stream2);
        assert_ne!(stream1, stream3);
    }

    #[test]
    fn test_cloning() {
        let stream1 = StreamName::new("test-stream".to_string()).unwrap();
        let stream2 = stream1.clone();
        assert_eq!(stream1, stream2);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let stream1 = StreamName::new("user:123".to_string()).unwrap();
        let stream2 = StreamName::new("user:123".to_string()).unwrap();

        let mut set = HashSet::new();
        set.insert(stream1);

        assert!(set.contains(&stream2));
    }

    #[test]
    fn test_serde_serialization() {
        let stream = StreamName::new("user:123".to_string()).unwrap();

        // Serialize
        let json = serde_json::to_string(&stream).unwrap();
        assert_eq!(json, "\"user:123\"");

        // Deserialize
        let deserialized: StreamName = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, stream);
    }

    #[test]
    fn test_as_ref() {
        let stream = StreamName::new("test-stream".to_string()).unwrap();
        let str_ref: &str = stream.as_ref();
        assert_eq!(str_ref, "test-stream");
    }

    #[test]
    fn test_new_unchecked() {
        // Should create without validation (for internal use)
        let stream = StreamName::new_unchecked("invalid stream!".to_string());
        assert_eq!(stream.as_str(), "invalid stream!");
    }
}
