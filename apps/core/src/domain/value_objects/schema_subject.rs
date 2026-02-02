use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Value Object: SchemaSubject
///
/// Represents the subject/topic that a schema applies to.
/// Subjects typically correspond to event types (e.g., "user.created", "order.placed").
///
/// Domain Rules:
/// - Cannot be empty
/// - Must be between 1 and 256 characters
/// - Must be lowercase with dots, underscores, or hyphens
/// - Case-sensitive
/// - Immutable once created
/// - Must be unique within a schema registry
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaSubject(String);

impl SchemaSubject {
    /// Create a new SchemaSubject with validation
    ///
    /// # Errors
    /// Returns error if:
    /// - Subject is empty
    /// - Subject is longer than 256 characters
    /// - Subject contains invalid characters
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::SchemaSubject;
    ///
    /// let subject = SchemaSubject::new("user.created".to_string()).unwrap();
    /// assert_eq!(subject.as_str(), "user.created");
    /// ```
    pub fn new(value: String) -> Result<Self> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    /// Create SchemaSubject without validation (for internal use, e.g., from trusted storage)
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

    /// Get the namespace (everything before the first dot)
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::SchemaSubject;
    ///
    /// let subject = SchemaSubject::new("user.created".to_string()).unwrap();
    /// assert_eq!(subject.namespace(), Some("user"));
    ///
    /// let subject = SchemaSubject::new("simple".to_string()).unwrap();
    /// assert_eq!(subject.namespace(), None);
    /// ```
    pub fn namespace(&self) -> Option<&str> {
        self.0.split('.').next().filter(|_| self.0.contains('.'))
    }

    /// Get the action/event name (everything after the last dot)
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::SchemaSubject;
    ///
    /// let subject = SchemaSubject::new("user.created".to_string()).unwrap();
    /// assert_eq!(subject.action(), Some("created"));
    ///
    /// let subject = SchemaSubject::new("user.profile.updated".to_string()).unwrap();
    /// assert_eq!(subject.action(), Some("updated"));
    /// ```
    pub fn action(&self) -> Option<&str> {
        self.0.rsplit('.').next().filter(|_| self.0.contains('.'))
    }

    /// Check if this subject is in a specific namespace
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::SchemaSubject;
    ///
    /// let subject = SchemaSubject::new("user.created".to_string()).unwrap();
    /// assert!(subject.is_in_namespace("user"));
    /// assert!(!subject.is_in_namespace("order"));
    /// ```
    pub fn is_in_namespace(&self, namespace: &str) -> bool {
        self.namespace() == Some(namespace)
    }

    /// Check if this subject starts with a prefix
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }

    /// Check if this subject matches an event type pattern
    ///
    /// Supports wildcards: `*` matches any single segment, `**` matches any number of segments
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::SchemaSubject;
    ///
    /// let subject = SchemaSubject::new("user.created".to_string()).unwrap();
    /// assert!(subject.matches_pattern("user.created"));
    /// assert!(subject.matches_pattern("user.*"));
    /// assert!(subject.matches_pattern("**"));
    /// ```
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        if pattern == "**" {
            return true;
        }

        let subject_parts: Vec<&str> = self.0.split('.').collect();
        let pattern_parts: Vec<&str> = pattern.split('.').collect();

        if pattern.contains("**") {
            // Simple implementation: ** at the end matches any trailing segments
            let prefix: Vec<&str> = pattern_parts
                .iter()
                .take_while(|&&p| p != "**")
                .copied()
                .collect();

            if subject_parts.len() < prefix.len() {
                return false;
            }

            for (s, p) in subject_parts.iter().zip(prefix.iter()) {
                if *p != "*" && *s != *p {
                    return false;
                }
            }
            true
        } else {
            // Exact segment matching with * wildcards
            if subject_parts.len() != pattern_parts.len() {
                return false;
            }

            for (s, p) in subject_parts.iter().zip(pattern_parts.iter()) {
                if *p != "*" && *s != *p {
                    return false;
                }
            }
            true
        }
    }

    /// Validate a schema subject string
    fn validate(value: &str) -> Result<()> {
        // Rule: Cannot be empty
        if value.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Schema subject cannot be empty".to_string(),
            ));
        }

        // Rule: Maximum length 256 characters
        if value.len() > 256 {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Schema subject cannot exceed 256 characters, got {}",
                value.len()
            )));
        }

        // Rule: Must be lowercase with dots, underscores, or hyphens
        if !value
            .chars()
            .all(|c| c.is_lowercase() || c.is_numeric() || c == '.' || c == '_' || c == '-')
        {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Schema subject '{}' must be lowercase with dots, underscores, or hyphens",
                value
            )));
        }

        // Rule: Cannot start or end with special characters
        if value.starts_with('.')
            || value.starts_with('-')
            || value.starts_with('_')
            || value.ends_with('.')
            || value.ends_with('-')
            || value.ends_with('_')
        {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Schema subject '{}' cannot start or end with special characters",
                value
            )));
        }

        // Rule: Cannot have consecutive dots
        if value.contains("..") {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Schema subject '{}' cannot have consecutive dots",
                value
            )));
        }

        Ok(())
    }
}

impl fmt::Display for SchemaSubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for SchemaSubject {
    type Error = crate::error::AllSourceError;

    fn try_from(value: &str) -> Result<Self> {
        SchemaSubject::new(value.to_string())
    }
}

impl TryFrom<String> for SchemaSubject {
    type Error = crate::error::AllSourceError;

    fn try_from(value: String) -> Result<Self> {
        SchemaSubject::new(value)
    }
}

impl AsRef<str> for SchemaSubject {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_subjects() {
        // Simple dot-separated
        let subject = SchemaSubject::new("user.created".to_string());
        assert!(subject.is_ok());
        assert_eq!(subject.unwrap().as_str(), "user.created");

        // Three parts
        let subject = SchemaSubject::new("user.profile.updated".to_string());
        assert!(subject.is_ok());

        // With underscore
        let subject = SchemaSubject::new("order_item.created".to_string());
        assert!(subject.is_ok());

        // With hyphen
        let subject = SchemaSubject::new("payment-processed".to_string());
        assert!(subject.is_ok());

        // With numbers
        let subject = SchemaSubject::new("event.v2.updated".to_string());
        assert!(subject.is_ok());

        // Single word
        let subject = SchemaSubject::new("created".to_string());
        assert!(subject.is_ok());
    }

    #[test]
    fn test_reject_empty_subject() {
        let result = SchemaSubject::new("".to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn test_reject_too_long_subject() {
        let long_subject = "a".repeat(257);
        let result = SchemaSubject::new(long_subject);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("cannot exceed 256 characters"));
        }
    }

    #[test]
    fn test_accept_max_length_subject() {
        let max_subject = "a".repeat(256);
        let result = SchemaSubject::new(max_subject);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_uppercase() {
        let result = SchemaSubject::new("User.Created".to_string());
        assert!(result.is_err());

        let result = SchemaSubject::new("user.CREATED".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_invalid_characters() {
        // Space
        let result = SchemaSubject::new("user created".to_string());
        assert!(result.is_err());

        // Colon
        let result = SchemaSubject::new("user:created".to_string());
        assert!(result.is_err());

        // Special characters
        let result = SchemaSubject::new("user@created".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_starting_with_special_char() {
        let result = SchemaSubject::new(".user.created".to_string());
        assert!(result.is_err());

        let result = SchemaSubject::new("-user.created".to_string());
        assert!(result.is_err());

        let result = SchemaSubject::new("_user.created".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_ending_with_special_char() {
        let result = SchemaSubject::new("user.created.".to_string());
        assert!(result.is_err());

        let result = SchemaSubject::new("user.created-".to_string());
        assert!(result.is_err());

        let result = SchemaSubject::new("user.created_".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_consecutive_dots() {
        let result = SchemaSubject::new("user..created".to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("consecutive dots"));
        }
    }

    #[test]
    fn test_namespace_extraction() {
        let subject = SchemaSubject::new("user.created".to_string()).unwrap();
        assert_eq!(subject.namespace(), Some("user"));

        let subject = SchemaSubject::new("user.profile.updated".to_string()).unwrap();
        assert_eq!(subject.namespace(), Some("user"));

        // No dot
        let subject = SchemaSubject::new("created".to_string()).unwrap();
        assert_eq!(subject.namespace(), None);
    }

    #[test]
    fn test_action_extraction() {
        let subject = SchemaSubject::new("user.created".to_string()).unwrap();
        assert_eq!(subject.action(), Some("created"));

        let subject = SchemaSubject::new("user.profile.updated".to_string()).unwrap();
        assert_eq!(subject.action(), Some("updated"));

        // No dot
        let subject = SchemaSubject::new("created".to_string()).unwrap();
        assert_eq!(subject.action(), None);
    }

    #[test]
    fn test_is_in_namespace() {
        let subject = SchemaSubject::new("user.created".to_string()).unwrap();
        assert!(subject.is_in_namespace("user"));
        assert!(!subject.is_in_namespace("order"));
    }

    #[test]
    fn test_starts_with() {
        let subject = SchemaSubject::new("user.created".to_string()).unwrap();
        assert!(subject.starts_with("user"));
        assert!(subject.starts_with("user."));
        assert!(!subject.starts_with("order"));
    }

    #[test]
    fn test_matches_pattern_exact() {
        let subject = SchemaSubject::new("user.created".to_string()).unwrap();
        assert!(subject.matches_pattern("user.created"));
        assert!(!subject.matches_pattern("user.updated"));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        let subject = SchemaSubject::new("user.created".to_string()).unwrap();
        assert!(subject.matches_pattern("user.*"));
        assert!(subject.matches_pattern("*.created"));
        assert!(subject.matches_pattern("*.*"));
        assert!(!subject.matches_pattern("order.*"));
    }

    #[test]
    fn test_matches_pattern_double_wildcard() {
        let subject = SchemaSubject::new("user.profile.updated".to_string()).unwrap();
        assert!(subject.matches_pattern("**"));
        assert!(subject.matches_pattern("user.**"));
        assert!(subject.matches_pattern("user.profile.**"));
    }

    #[test]
    fn test_display_trait() {
        let subject = SchemaSubject::new("user.created".to_string()).unwrap();
        assert_eq!(format!("{}", subject), "user.created");
    }

    #[test]
    fn test_try_from_str() {
        let subject: Result<SchemaSubject> = "user.created".try_into();
        assert!(subject.is_ok());
        assert_eq!(subject.unwrap().as_str(), "user.created");

        let invalid: Result<SchemaSubject> = "".try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let subject: Result<SchemaSubject> = "order.placed".to_string().try_into();
        assert!(subject.is_ok());

        let invalid: Result<SchemaSubject> = String::new().try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_into_inner() {
        let subject = SchemaSubject::new("test.subject".to_string()).unwrap();
        let inner = subject.into_inner();
        assert_eq!(inner, "test.subject");
    }

    #[test]
    fn test_equality() {
        let subject1 = SchemaSubject::new("user.created".to_string()).unwrap();
        let subject2 = SchemaSubject::new("user.created".to_string()).unwrap();
        let subject3 = SchemaSubject::new("order.placed".to_string()).unwrap();

        assert_eq!(subject1, subject2);
        assert_ne!(subject1, subject3);
    }

    #[test]
    fn test_cloning() {
        let subject1 = SchemaSubject::new("test.subject".to_string()).unwrap();
        let subject2 = subject1.clone();
        assert_eq!(subject1, subject2);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let subject1 = SchemaSubject::new("user.created".to_string()).unwrap();
        let subject2 = SchemaSubject::new("user.created".to_string()).unwrap();

        let mut set = HashSet::new();
        set.insert(subject1);

        assert!(set.contains(&subject2));
    }

    #[test]
    fn test_serde_serialization() {
        let subject = SchemaSubject::new("user.created".to_string()).unwrap();

        // Serialize
        let json = serde_json::to_string(&subject).unwrap();
        assert_eq!(json, "\"user.created\"");

        // Deserialize
        let deserialized: SchemaSubject = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, subject);
    }

    #[test]
    fn test_as_ref() {
        let subject = SchemaSubject::new("test.subject".to_string()).unwrap();
        let str_ref: &str = subject.as_ref();
        assert_eq!(str_ref, "test.subject");
    }

    #[test]
    fn test_new_unchecked() {
        // Should create without validation (for internal use)
        let subject = SchemaSubject::new_unchecked("INVALID Subject!".to_string());
        assert_eq!(subject.as_str(), "INVALID Subject!");
    }
}
