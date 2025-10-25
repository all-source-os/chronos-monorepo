use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Compatibility mode for schema evolution
///
/// Defines how schema changes are validated when registering new versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityMode {
    /// No compatibility checking
    None,
    /// New schema must be backward compatible (can read old data)
    Backward,
    /// New schema must be forward compatible (old readers can read new data)
    Forward,
    /// New schema must be both backward and forward compatible
    Full,
}

impl Default for CompatibilityMode {
    fn default() -> Self {
        Self::Backward
    }
}

impl CompatibilityMode {
    /// Check if this mode requires backward compatibility
    pub fn requires_backward(&self) -> bool {
        matches!(self, Self::Backward | Self::Full)
    }

    /// Check if this mode requires forward compatibility
    pub fn requires_forward(&self) -> bool {
        matches!(self, Self::Forward | Self::Full)
    }

    /// Check if any compatibility is required
    pub fn requires_compatibility(&self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Domain Entity: Schema
///
/// Represents a versioned schema definition for event validation.
/// Schemas define the structure and validation rules for event payloads.
///
/// Domain Rules:
/// - Subject cannot be empty
/// - Version starts at 1 and increments
/// - Schema must be valid JSON
/// - Once registered, schemas are immutable
/// - New versions must respect compatibility mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    id: Uuid,
    subject: String,
    version: u32,
    schema_definition: JsonValue,
    created_at: DateTime<Utc>,
    description: Option<String>,
    tags: Vec<String>,
    compatibility_mode: CompatibilityMode,
}

impl Schema {
    /// Create a new schema with validation
    ///
    /// # Arguments
    /// * `subject` - The subject/topic this schema applies to (e.g., "order.placed")
    /// * `version` - Version number (must be >= 1)
    /// * `schema_definition` - JSON Schema definition
    /// * `compatibility_mode` - How to validate future schema changes
    pub fn new(
        subject: String,
        version: u32,
        schema_definition: JsonValue,
        compatibility_mode: CompatibilityMode,
    ) -> Result<Self> {
        Self::validate_subject(&subject)?;
        Self::validate_version(version)?;
        Self::validate_schema(&schema_definition)?;

        Ok(Self {
            id: Uuid::new_v4(),
            subject,
            version,
            schema_definition,
            created_at: Utc::now(),
            description: None,
            tags: Vec::new(),
            compatibility_mode,
        })
    }

    /// Create first version of a schema
    pub fn new_v1(
        subject: String,
        schema_definition: JsonValue,
        compatibility_mode: CompatibilityMode,
    ) -> Result<Self> {
        Self::new(subject, 1, schema_definition, compatibility_mode)
    }

    /// Reconstruct schema from storage (bypasses validation)
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        subject: String,
        version: u32,
        schema_definition: JsonValue,
        created_at: DateTime<Utc>,
        description: Option<String>,
        tags: Vec<String>,
        compatibility_mode: CompatibilityMode,
    ) -> Self {
        Self {
            id,
            subject,
            version,
            schema_definition,
            created_at,
            description,
            tags,
            compatibility_mode,
        }
    }

    // Getters

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn schema_definition(&self) -> &JsonValue {
        &self.schema_definition
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn compatibility_mode(&self) -> CompatibilityMode {
        self.compatibility_mode
    }

    // Domain behavior methods

    /// Add or update description
    pub fn set_description(&mut self, description: String) -> Result<()> {
        Self::validate_description(&description)?;
        self.description = Some(description);
        Ok(())
    }

    /// Clear description
    pub fn clear_description(&mut self) {
        self.description = None;
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) -> Result<()> {
        Self::validate_tag(&tag)?;

        if self.tags.contains(&tag) {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Tag '{}' already exists", tag),
            ));
        }

        self.tags.push(tag);
        Ok(())
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) -> Result<()> {
        let initial_len = self.tags.len();
        self.tags.retain(|t| t != tag);

        if self.tags.len() == initial_len {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Tag '{}' not found", tag),
            ));
        }

        Ok(())
    }

    /// Check if schema has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Check if this schema is the first version
    pub fn is_first_version(&self) -> bool {
        self.version == 1
    }

    /// Check if this schema applies to a subject
    pub fn applies_to(&self, subject: &str) -> bool {
        self.subject == subject
    }

    /// Create next version of schema
    pub fn create_next_version(&self, new_schema: JsonValue) -> Result<Schema> {
        Schema::new(
            self.subject.clone(),
            self.version + 1,
            new_schema,
            self.compatibility_mode,
        )
    }

    // Validation methods

    fn validate_subject(subject: &str) -> Result<()> {
        if subject.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Schema subject cannot be empty".to_string(),
            ));
        }

        if subject.len() > 256 {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Schema subject cannot exceed 256 characters, got {}", subject.len()),
            ));
        }

        // Subject should follow similar naming as event types
        if !subject.chars().all(|c| c.is_lowercase() || c.is_numeric() || c == '.' || c == '_' || c == '-') {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Schema subject '{}' must be lowercase with dots, underscores, or hyphens", subject),
            ));
        }

        Ok(())
    }

    fn validate_version(version: u32) -> Result<()> {
        if version == 0 {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Schema version must be >= 1".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_schema(schema: &JsonValue) -> Result<()> {
        if schema.is_null() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Schema definition cannot be null".to_string(),
            ));
        }

        // Basic validation: should be an object with "type" property
        if let Some(obj) = schema.as_object() {
            if !obj.contains_key("type") && !obj.contains_key("$schema") {
                return Err(crate::error::AllSourceError::InvalidInput(
                    "Schema definition should contain 'type' or '$schema' property".to_string(),
                ));
            }
        } else {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Schema definition must be a JSON object".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_description(description: &str) -> Result<()> {
        if description.len() > 1000 {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Schema description cannot exceed 1000 characters, got {}", description.len()),
            ));
        }
        Ok(())
    }

    fn validate_tag(tag: &str) -> Result<()> {
        if tag.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Tag cannot be empty".to_string(),
            ));
        }

        if tag.len() > 50 {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Tag cannot exceed 50 characters, got {}", tag.len()),
            ));
        }

        if !tag.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Tag '{}' must be alphanumeric with hyphens or underscores", tag),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_schema() -> JsonValue {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "number" }
            }
        })
    }

    #[test]
    fn test_create_schema() {
        let schema = Schema::new(
            "user.created".to_string(),
            1,
            valid_schema(),
            CompatibilityMode::Backward,
        );

        assert!(schema.is_ok());
        let schema = schema.unwrap();
        assert_eq!(schema.subject(), "user.created");
        assert_eq!(schema.version(), 1);
        assert_eq!(schema.compatibility_mode(), CompatibilityMode::Backward);
    }

    #[test]
    fn test_create_v1_schema() {
        let schema = Schema::new_v1(
            "order.placed".to_string(),
            valid_schema(),
            CompatibilityMode::Full,
        );

        assert!(schema.is_ok());
        let schema = schema.unwrap();
        assert_eq!(schema.version(), 1);
        assert!(schema.is_first_version());
    }

    #[test]
    fn test_reject_empty_subject() {
        let result = Schema::new(
            "".to_string(),
            1,
            valid_schema(),
            CompatibilityMode::None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_too_long_subject() {
        let long_subject = "a".repeat(257);
        let result = Schema::new(
            long_subject,
            1,
            valid_schema(),
            CompatibilityMode::None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_invalid_subject_characters() {
        let result = Schema::new(
            "User.Created".to_string(), // Uppercase not allowed
            1,
            valid_schema(),
            CompatibilityMode::None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_accept_valid_subjects() {
        let subjects = vec![
            "user.created",
            "order_placed",
            "payment-processed",
            "event.v2.updated",
        ];

        for subject in subjects {
            let result = Schema::new(
                subject.to_string(),
                1,
                valid_schema(),
                CompatibilityMode::None,
            );
            assert!(result.is_ok(), "Subject '{}' should be valid", subject);
        }
    }

    #[test]
    fn test_reject_zero_version() {
        let result = Schema::new(
            "test.event".to_string(),
            0, // Invalid
            valid_schema(),
            CompatibilityMode::None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_null_schema() {
        let result = Schema::new(
            "test.event".to_string(),
            1,
            JsonValue::Null,
            CompatibilityMode::None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_non_object_schema() {
        let result = Schema::new(
            "test.event".to_string(),
            1,
            json!("not an object"),
            CompatibilityMode::None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_schema_without_type() {
        let result = Schema::new(
            "test.event".to_string(),
            1,
            json!({"properties": {}}), // Missing "type"
            CompatibilityMode::None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_accept_schema_with_schema_property() {
        let result = Schema::new(
            "test.event".to_string(),
            1,
            json!({"$schema": "http://json-schema.org/draft-07/schema#"}),
            CompatibilityMode::None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_set_description() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        assert!(schema.description().is_none());

        let result = schema.set_description("Test schema".to_string());
        assert!(result.is_ok());
        assert_eq!(schema.description(), Some("Test schema"));
    }

    #[test]
    fn test_reject_too_long_description() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        let long_desc = "a".repeat(1001);
        let result = schema.set_description(long_desc);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_description() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        schema.set_description("Test".to_string()).unwrap();
        assert!(schema.description().is_some());

        schema.clear_description();
        assert!(schema.description().is_none());
    }

    #[test]
    fn test_add_tag() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        assert_eq!(schema.tags().len(), 0);

        let result = schema.add_tag("production".to_string());
        assert!(result.is_ok());
        assert_eq!(schema.tags().len(), 1);
        assert!(schema.has_tag("production"));
    }

    #[test]
    fn test_reject_duplicate_tag() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        schema.add_tag("test".to_string()).unwrap();
        let result = schema.add_tag("test".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_tag() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        schema.add_tag("tag1".to_string()).unwrap();
        schema.add_tag("tag2".to_string()).unwrap();

        let result = schema.remove_tag("tag1");
        assert!(result.is_ok());
        assert_eq!(schema.tags().len(), 1);
        assert!(!schema.has_tag("tag1"));
        assert!(schema.has_tag("tag2"));
    }

    #[test]
    fn test_remove_nonexistent_tag() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        let result = schema.remove_tag("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_invalid_tags() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        // Empty tag
        assert!(schema.add_tag("".to_string()).is_err());

        // Too long tag
        assert!(schema.add_tag("a".repeat(51)).is_err());

        // Invalid characters
        assert!(schema.add_tag("tag with spaces".to_string()).is_err());
        assert!(schema.add_tag("tag@invalid".to_string()).is_err());
    }

    #[test]
    fn test_accept_valid_tags() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        let valid_tags = vec!["production", "test-env", "v2_schema", "important123"];

        for tag in valid_tags {
            assert!(schema.add_tag(tag.to_string()).is_ok());
        }
    }

    #[test]
    fn test_is_first_version() {
        let schema_v1 = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        let schema_v2 = Schema::new(
            "test.event".to_string(),
            2,
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        assert!(schema_v1.is_first_version());
        assert!(!schema_v2.is_first_version());
    }

    #[test]
    fn test_applies_to() {
        let schema = Schema::new_v1(
            "user.created".to_string(),
            valid_schema(),
            CompatibilityMode::None,
        ).unwrap();

        assert!(schema.applies_to("user.created"));
        assert!(!schema.applies_to("order.placed"));
    }

    #[test]
    fn test_create_next_version() {
        let schema_v1 = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::Backward,
        ).unwrap();

        let new_schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "number" },
                "email": { "type": "string" }  // New field
            }
        });

        let schema_v2 = schema_v1.create_next_version(new_schema);
        assert!(schema_v2.is_ok());

        let schema_v2 = schema_v2.unwrap();
        assert_eq!(schema_v2.version(), 2);
        assert_eq!(schema_v2.subject(), "test.event");
        assert_eq!(schema_v2.compatibility_mode(), CompatibilityMode::Backward);
    }

    #[test]
    fn test_compatibility_mode_checks() {
        assert!(CompatibilityMode::Backward.requires_backward());
        assert!(!CompatibilityMode::Backward.requires_forward());

        assert!(!CompatibilityMode::Forward.requires_backward());
        assert!(CompatibilityMode::Forward.requires_forward());

        assert!(CompatibilityMode::Full.requires_backward());
        assert!(CompatibilityMode::Full.requires_forward());

        assert!(!CompatibilityMode::None.requires_backward());
        assert!(!CompatibilityMode::None.requires_forward());
        assert!(!CompatibilityMode::None.requires_compatibility());
    }

    #[test]
    fn test_serde_serialization() {
        let schema = Schema::new_v1(
            "test.event".to_string(),
            valid_schema(),
            CompatibilityMode::Backward,
        ).unwrap();

        // Should be able to serialize
        let json = serde_json::to_string(&schema);
        assert!(json.is_ok());

        // Should be able to deserialize
        let deserialized = serde_json::from_str::<Schema>(&json.unwrap());
        assert!(deserialized.is_ok());
    }
}
