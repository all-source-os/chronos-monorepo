use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Value Object: ArticleId
///
/// Represents a unique identifier for an article in the paywall system.
/// Articles are identified by a slug or URL-safe identifier provided by the creator.
///
/// Domain Rules:
/// - Cannot be empty
/// - Must be between 1 and 256 characters
/// - Must be URL-safe (alphanumeric, hyphens, underscores only)
/// - Case-sensitive
/// - Immutable once created
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArticleId(String);

impl ArticleId {
    /// Create a new ArticleId with validation
    ///
    /// # Errors
    /// Returns error if:
    /// - ID is empty
    /// - ID is longer than 256 characters
    /// - ID contains invalid characters (only a-z, A-Z, 0-9, -, _ allowed)
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::ArticleId;
    ///
    /// let article_id = ArticleId::new("my-awesome-article".to_string()).unwrap();
    /// assert_eq!(article_id.as_str(), "my-awesome-article");
    /// ```
    pub fn new(value: String) -> Result<Self> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    /// Create ArticleId without validation (for internal use, e.g., from trusted storage)
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

    /// Check if this article ID starts with a specific prefix
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }

    /// Check if this article ID ends with a specific suffix
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.0.ends_with(suffix)
    }

    /// Validate an article ID string
    fn validate(value: &str) -> Result<()> {
        // Rule: Cannot be empty
        if value.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Article ID cannot be empty".to_string(),
            ));
        }

        // Rule: Maximum length 256 characters
        if value.len() > 256 {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Article ID cannot exceed 256 characters, got {}",
                value.len()
            )));
        }

        // Rule: URL-safe characters only (alphanumeric, hyphens, underscores)
        if !value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Article ID '{}' contains invalid characters. Only alphanumeric, hyphens, and underscores allowed",
                value
            )));
        }

        Ok(())
    }
}

impl fmt::Display for ArticleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for ArticleId {
    type Error = crate::error::AllSourceError;

    fn try_from(value: &str) -> Result<Self> {
        ArticleId::new(value.to_string())
    }
}

impl TryFrom<String> for ArticleId {
    type Error = crate::error::AllSourceError;

    fn try_from(value: String) -> Result<Self> {
        ArticleId::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_article_ids() {
        // Simple alphanumeric
        let article_id = ArticleId::new("article123".to_string());
        assert!(article_id.is_ok());
        assert_eq!(article_id.unwrap().as_str(), "article123");

        // With hyphens
        let article_id = ArticleId::new("my-awesome-article".to_string());
        assert!(article_id.is_ok());

        // With underscores
        let article_id = ArticleId::new("my_awesome_article".to_string());
        assert!(article_id.is_ok());

        // Mixed case
        let article_id = ArticleId::new("MyAwesomeArticle".to_string());
        assert!(article_id.is_ok());

        // Complex format
        let article_id = ArticleId::new("how-to-scale-to-1M-users_2024".to_string());
        assert!(article_id.is_ok());
    }

    #[test]
    fn test_reject_empty_article_id() {
        let result = ArticleId::new("".to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn test_reject_too_long_article_id() {
        // Create a 257-character string (exceeds max of 256)
        let long_id = "a".repeat(257);
        let result = ArticleId::new(long_id);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("cannot exceed 256 characters"));
        }
    }

    #[test]
    fn test_accept_max_length_article_id() {
        // Exactly 256 characters should be OK
        let max_id = "a".repeat(256);
        let result = ArticleId::new(max_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_invalid_characters() {
        // Space is invalid
        let result = ArticleId::new("article 123".to_string());
        assert!(result.is_err());

        // Special characters are invalid
        let result = ArticleId::new("article@123".to_string());
        assert!(result.is_err());

        let result = ArticleId::new("article.123".to_string());
        assert!(result.is_err());

        let result = ArticleId::new("article/123".to_string());
        assert!(result.is_err());

        let result = ArticleId::new("article?query=1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_display_trait() {
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
        assert_eq!(format!("{}", article_id), "test-article");
    }

    #[test]
    fn test_try_from_str() {
        let article_id: Result<ArticleId> = "valid-article".try_into();
        assert!(article_id.is_ok());
        assert_eq!(article_id.unwrap().as_str(), "valid-article");

        let invalid: Result<ArticleId> = "".try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let article_id: Result<ArticleId> = "valid-article".to_string().try_into();
        assert!(article_id.is_ok());

        let invalid: Result<ArticleId> = String::new().try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_into_inner() {
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
        let inner = article_id.into_inner();
        assert_eq!(inner, "test-article");
    }

    #[test]
    fn test_starts_with() {
        let article_id = ArticleId::new("kubernetes-tutorial".to_string()).unwrap();
        assert!(article_id.starts_with("kubernetes"));
        assert!(!article_id.starts_with("docker"));
    }

    #[test]
    fn test_ends_with() {
        let article_id = ArticleId::new("kubernetes-tutorial".to_string()).unwrap();
        assert!(article_id.ends_with("tutorial"));
        assert!(!article_id.ends_with("guide"));
    }

    #[test]
    fn test_equality() {
        let id1 = ArticleId::new("article-a".to_string()).unwrap();
        let id2 = ArticleId::new("article-a".to_string()).unwrap();
        let id3 = ArticleId::new("article-b".to_string()).unwrap();

        // Value equality
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_cloning() {
        let id1 = ArticleId::new("article".to_string()).unwrap();
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let id1 = ArticleId::new("article".to_string()).unwrap();
        let id2 = ArticleId::new("article".to_string()).unwrap();

        let mut set = HashSet::new();
        set.insert(id1);

        // Should find the same value (value equality)
        assert!(set.contains(&id2));
    }

    #[test]
    fn test_serde_serialization() {
        let article_id = ArticleId::new("test-article".to_string()).unwrap();

        // Serialize
        let json = serde_json::to_string(&article_id).unwrap();
        assert_eq!(json, "\"test-article\"");

        // Deserialize
        let deserialized: ArticleId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, article_id);
    }

    #[test]
    fn test_new_unchecked() {
        // Should create without validation (for internal use)
        let article_id = ArticleId::new_unchecked("invalid chars!@#".to_string());
        assert_eq!(article_id.as_str(), "invalid chars!@#");
    }
}
