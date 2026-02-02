use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Value Object: ProjectionName
///
/// Represents the name of a projection in the event store.
/// Projections are materialized views derived from event streams.
///
/// Domain Rules:
/// - Cannot be empty
/// - Must be between 1 and 100 characters
/// - Must be alphanumeric with hyphens/underscores only
/// - Case-sensitive
/// - Immutable once created
/// - Must be unique within a tenant
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectionName(String);

impl ProjectionName {
    /// Create a new ProjectionName with validation
    ///
    /// # Errors
    /// Returns error if:
    /// - Name is empty
    /// - Name is longer than 100 characters
    /// - Name contains invalid characters
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::ProjectionName;
    ///
    /// let name = ProjectionName::new("user_snapshot".to_string()).unwrap();
    /// assert_eq!(name.as_str(), "user_snapshot");
    /// ```
    pub fn new(value: String) -> Result<Self> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    /// Create ProjectionName without validation (for internal use, e.g., from trusted storage)
    ///
    /// # Safety
    /// This bypasses validation. Only use when loading from trusted sources
    /// where validation has already occurred.
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the inner String (consumes self)
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Check if this name starts with a prefix
    ///
    /// Useful for grouping projections by naming convention.
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::ProjectionName;
    ///
    /// let name = ProjectionName::new("user_snapshot".to_string()).unwrap();
    /// assert!(name.starts_with("user"));
    /// assert!(!name.starts_with("order"));
    /// ```
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }

    /// Check if this name ends with a suffix
    ///
    /// Useful for identifying projection types by naming convention.
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::ProjectionName;
    ///
    /// let name = ProjectionName::new("user_snapshot".to_string()).unwrap();
    /// assert!(name.ends_with("snapshot"));
    /// assert!(!name.ends_with("counter"));
    /// ```
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.0.ends_with(suffix)
    }

    /// Check if this name contains a substring
    pub fn contains(&self, pattern: &str) -> bool {
        self.0.contains(pattern)
    }

    /// Validate a projection name string
    fn validate(value: &str) -> Result<()> {
        // Rule: Cannot be empty
        if value.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Projection name cannot be empty".to_string(),
            ));
        }

        // Rule: Maximum length 100 characters
        if value.len() > 100 {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Projection name cannot exceed 100 characters, got {}",
                value.len()
            )));
        }

        // Rule: Only alphanumeric, hyphens, and underscores
        if !value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Projection name '{}' must be alphanumeric with hyphens or underscores",
                value
            )));
        }

        // Rule: Cannot start or end with hyphen/underscore
        if value.starts_with('-')
            || value.starts_with('_')
            || value.ends_with('-')
            || value.ends_with('_')
        {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Projection name '{}' cannot start or end with hyphen or underscore",
                value
            )));
        }

        Ok(())
    }
}

impl fmt::Display for ProjectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for ProjectionName {
    type Error = crate::error::AllSourceError;

    fn try_from(value: &str) -> Result<Self> {
        ProjectionName::new(value.to_string())
    }
}

impl TryFrom<String> for ProjectionName {
    type Error = crate::error::AllSourceError;

    fn try_from(value: String) -> Result<Self> {
        ProjectionName::new(value)
    }
}

impl AsRef<str> for ProjectionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_names() {
        // Simple name
        let name = ProjectionName::new("user_snapshot".to_string());
        assert!(name.is_ok());
        assert_eq!(name.unwrap().as_str(), "user_snapshot");

        // With hyphen
        let name = ProjectionName::new("event-counter".to_string());
        assert!(name.is_ok());

        // Just alphanumeric
        let name = ProjectionName::new("projection123".to_string());
        assert!(name.is_ok());

        // Mixed
        let name = ProjectionName::new("user-account_snapshot123".to_string());
        assert!(name.is_ok());
    }

    #[test]
    fn test_reject_empty_name() {
        let result = ProjectionName::new("".to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn test_reject_too_long_name() {
        let long_name = "a".repeat(101);
        let result = ProjectionName::new(long_name);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("cannot exceed 100 characters"));
        }
    }

    #[test]
    fn test_accept_max_length_name() {
        let max_name = "a".repeat(100);
        let result = ProjectionName::new(max_name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_invalid_characters() {
        // Space
        let result = ProjectionName::new("user snapshot".to_string());
        assert!(result.is_err());

        // Dot
        let result = ProjectionName::new("user.snapshot".to_string());
        assert!(result.is_err());

        // Colon
        let result = ProjectionName::new("user:snapshot".to_string());
        assert!(result.is_err());

        // Special characters
        let result = ProjectionName::new("user@snapshot".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_starting_with_special_char() {
        let result = ProjectionName::new("-projection".to_string());
        assert!(result.is_err());

        let result = ProjectionName::new("_projection".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_ending_with_special_char() {
        let result = ProjectionName::new("projection-".to_string());
        assert!(result.is_err());

        let result = ProjectionName::new("projection_".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_starts_with() {
        let name = ProjectionName::new("user_snapshot".to_string()).unwrap();
        assert!(name.starts_with("user"));
        assert!(name.starts_with("user_"));
        assert!(!name.starts_with("order"));
    }

    #[test]
    fn test_ends_with() {
        let name = ProjectionName::new("user_snapshot".to_string()).unwrap();
        assert!(name.ends_with("snapshot"));
        assert!(name.ends_with("_snapshot"));
        assert!(!name.ends_with("counter"));
    }

    #[test]
    fn test_contains() {
        let name = ProjectionName::new("user_snapshot".to_string()).unwrap();
        assert!(name.contains("user"));
        assert!(name.contains("_"));
        assert!(name.contains("snap"));
        assert!(!name.contains("order"));
    }

    #[test]
    fn test_display_trait() {
        let name = ProjectionName::new("user_snapshot".to_string()).unwrap();
        assert_eq!(format!("{}", name), "user_snapshot");
    }

    #[test]
    fn test_try_from_str() {
        let name: Result<ProjectionName> = "user_snapshot".try_into();
        assert!(name.is_ok());
        assert_eq!(name.unwrap().as_str(), "user_snapshot");

        let invalid: Result<ProjectionName> = "".try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let name: Result<ProjectionName> = "event_counter".to_string().try_into();
        assert!(name.is_ok());

        let invalid: Result<ProjectionName> = String::new().try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_into_inner() {
        let name = ProjectionName::new("test_projection".to_string()).unwrap();
        let inner = name.into_inner();
        assert_eq!(inner, "test_projection");
    }

    #[test]
    fn test_equality() {
        let name1 = ProjectionName::new("user_snapshot".to_string()).unwrap();
        let name2 = ProjectionName::new("user_snapshot".to_string()).unwrap();
        let name3 = ProjectionName::new("order_snapshot".to_string()).unwrap();

        assert_eq!(name1, name2);
        assert_ne!(name1, name3);
    }

    #[test]
    fn test_cloning() {
        let name1 = ProjectionName::new("test_projection".to_string()).unwrap();
        let name2 = name1.clone();
        assert_eq!(name1, name2);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let name1 = ProjectionName::new("user_snapshot".to_string()).unwrap();
        let name2 = ProjectionName::new("user_snapshot".to_string()).unwrap();

        let mut set = HashSet::new();
        set.insert(name1);

        assert!(set.contains(&name2));
    }

    #[test]
    fn test_serde_serialization() {
        let name = ProjectionName::new("user_snapshot".to_string()).unwrap();

        // Serialize
        let json = serde_json::to_string(&name).unwrap();
        assert_eq!(json, "\"user_snapshot\"");

        // Deserialize
        let deserialized: ProjectionName = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, name);
    }

    #[test]
    fn test_as_ref() {
        let name = ProjectionName::new("test_projection".to_string()).unwrap();
        let str_ref: &str = name.as_ref();
        assert_eq!(str_ref, "test_projection");
    }

    #[test]
    fn test_new_unchecked() {
        // Should create without validation (for internal use)
        let name = ProjectionName::new_unchecked("invalid name!".to_string());
        assert_eq!(name.as_str(), "invalid name!");
    }
}
