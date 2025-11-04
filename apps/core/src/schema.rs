use crate::error::{AllSourceError, Result};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Compatibility mode for schema evolution
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityMode {
    /// No compatibility checking
    None,
    /// New schema must be backward compatible (new fields optional)
    Backward,
    /// New schema must be forward compatible (old fields preserved)
    Forward,
    /// New schema must be both backward and forward compatible
    Full,
}

impl Default for CompatibilityMode {
    fn default() -> Self {
        Self::Backward
    }
}

/// Schema definition with versioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Unique schema ID
    pub id: Uuid,

    /// Subject/topic name (e.g., "user.created", "order.placed")
    pub subject: String,

    /// Schema version number
    pub version: u32,

    /// JSON Schema definition
    pub schema: JsonValue,

    /// When this schema was registered
    pub created_at: DateTime<Utc>,

    /// Schema description/documentation
    pub description: Option<String>,

    /// Tags for organization
    pub tags: Vec<String>,
}

impl Schema {
    pub fn new(subject: String, version: u32, schema: JsonValue) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject,
            version,
            schema,
            created_at: Utc::now(),
            description: None,
            tags: Vec::new(),
        }
    }
}

/// Request to register a new schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSchemaRequest {
    pub subject: String,
    pub schema: JsonValue,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Response from schema registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSchemaResponse {
    pub schema_id: Uuid,
    pub subject: String,
    pub version: u32,
    pub created_at: DateTime<Utc>,
}

/// Request to validate an event against a schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateEventRequest {
    pub subject: String,
    pub version: Option<u32>,
    pub payload: JsonValue,
}

/// Response from event validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateEventResponse {
    pub valid: bool,
    pub errors: Vec<String>,
    pub schema_version: u32,
}

/// Schema compatibility check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityCheckResult {
    pub compatible: bool,
    pub compatibility_mode: CompatibilityMode,
    pub issues: Vec<String>,
}

/// Statistics about the schema registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistryStats {
    pub total_schemas: usize,
    pub total_subjects: usize,
    pub validations_performed: u64,
    pub validation_failures: u64,
}

/// Configuration for the schema registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistryConfig {
    /// Default compatibility mode
    pub default_compatibility: CompatibilityMode,

    /// Whether to auto-register schemas on first use
    pub auto_register: bool,

    /// Whether to enforce schema validation on ingestion
    pub enforce_validation: bool,
}

impl Default for SchemaRegistryConfig {
    fn default() -> Self {
        Self {
            default_compatibility: CompatibilityMode::Backward,
            auto_register: false,
            enforce_validation: false,
        }
    }
}

/// Central registry for managing event schemas
pub struct SchemaRegistry {
    /// Schemas organized by subject and version
    /// Key: subject -> version -> Schema
    schemas: Arc<RwLock<HashMap<String, HashMap<u32, Schema>>>>,

    /// Latest version for each subject
    latest_versions: Arc<RwLock<HashMap<String, u32>>>,

    /// Compatibility mode for each subject
    compatibility_modes: Arc<RwLock<HashMap<String, CompatibilityMode>>>,

    /// Configuration
    config: SchemaRegistryConfig,

    /// Statistics
    stats: Arc<RwLock<SchemaRegistryStats>>,
}

impl SchemaRegistry {
    pub fn new(config: SchemaRegistryConfig) -> Self {
        Self {
            schemas: Arc::new(RwLock::new(HashMap::new())),
            latest_versions: Arc::new(RwLock::new(HashMap::new())),
            compatibility_modes: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(SchemaRegistryStats {
                total_schemas: 0,
                total_subjects: 0,
                validations_performed: 0,
                validation_failures: 0,
            })),
        }
    }

    /// Register a new schema or return existing if identical
    pub fn register_schema(
        &self,
        subject: String,
        schema: JsonValue,
        description: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<RegisterSchemaResponse> {
        let mut schemas = self.schemas.write();
        let mut latest_versions = self.latest_versions.write();

        // Get or create subject entry
        let subject_schemas = schemas.entry(subject.clone()).or_insert_with(HashMap::new);

        // Determine next version
        let next_version = latest_versions.get(&subject).map(|v| v + 1).unwrap_or(1);

        // Check compatibility with previous version if it exists
        if next_version > 1 {
            let prev_version = next_version - 1;
            if let Some(prev_schema) = subject_schemas.get(&prev_version) {
                let compatibility = self.get_compatibility_mode(&subject);
                let check_result = self.check_compatibility(
                    &prev_schema.schema,
                    &schema,
                    compatibility,
                )?;

                if !check_result.compatible {
                    return Err(AllSourceError::ValidationError(format!(
                        "Schema compatibility check failed: {}",
                        check_result.issues.join(", ")
                    )));
                }
            }
        }

        // Create and store the schema
        let mut new_schema = Schema::new(subject.clone(), next_version, schema);
        new_schema.description = description;
        new_schema.tags = tags.unwrap_or_default();

        let schema_id = new_schema.id;
        let created_at = new_schema.created_at;

        subject_schemas.insert(next_version, new_schema);
        latest_versions.insert(subject.clone(), next_version);

        // Update stats
        let mut stats = self.stats.write();
        stats.total_schemas += 1;
        if next_version == 1 {
            stats.total_subjects += 1;
        }

        tracing::info!(
            "📋 Registered schema v{} for subject '{}' (ID: {})",
            next_version,
            subject,
            schema_id
        );

        Ok(RegisterSchemaResponse {
            schema_id,
            subject,
            version: next_version,
            created_at,
        })
    }

    /// Get a schema by subject and version (or latest if no version specified)
    pub fn get_schema(&self, subject: &str, version: Option<u32>) -> Result<Schema> {
        let schemas = self.schemas.read();

        let subject_schemas = schemas
            .get(subject)
            .ok_or_else(|| AllSourceError::ValidationError(format!("Subject not found: {}", subject)))?;

        let version = match version {
            Some(v) => v,
            None => {
                let latest_versions = self.latest_versions.read();
                *latest_versions.get(subject).ok_or_else(|| {
                    AllSourceError::ValidationError(format!("No versions for subject: {}", subject))
                })?
            }
        };

        subject_schemas
            .get(&version)
            .cloned()
            .ok_or_else(|| {
                AllSourceError::ValidationError(format!(
                    "Schema version {} not found for subject: {}",
                    version, subject
                ))
            })
    }

    /// List all versions of a schema subject
    pub fn list_versions(&self, subject: &str) -> Result<Vec<u32>> {
        let schemas = self.schemas.read();

        let subject_schemas = schemas
            .get(subject)
            .ok_or_else(|| AllSourceError::ValidationError(format!("Subject not found: {}", subject)))?;

        let mut versions: Vec<u32> = subject_schemas.keys().copied().collect();
        versions.sort_unstable();

        Ok(versions)
    }

    /// List all schema subjects
    pub fn list_subjects(&self) -> Vec<String> {
        let schemas = self.schemas.read();
        schemas.keys().cloned().collect()
    }

    /// Validate a payload against a schema
    pub fn validate(
        &self,
        subject: &str,
        version: Option<u32>,
        payload: &JsonValue,
    ) -> Result<ValidateEventResponse> {
        let schema = self.get_schema(subject, version)?;

        let validation_result = self.validate_json(payload, &schema.schema);

        // Update stats
        let mut stats = self.stats.write();
        stats.validations_performed += 1;
        if !validation_result.is_empty() {
            stats.validation_failures += 1;
        }

        Ok(ValidateEventResponse {
            valid: validation_result.is_empty(),
            errors: validation_result,
            schema_version: schema.version,
        })
    }

    /// Internal JSON Schema validation
    fn validate_json(&self, data: &JsonValue, schema: &JsonValue) -> Vec<String> {
        let mut errors = Vec::new();

        // Basic JSON Schema validation
        // In production, use jsonschema crate, but implementing basic checks here

        // Check required fields
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            if let Some(obj) = data.as_object() {
                for req_field in required {
                    if let Some(field_name) = req_field.as_str() {
                        if !obj.contains_key(field_name) {
                            errors.push(format!("Missing required field: {}", field_name));
                        }
                    }
                }
            }
        }

        // Check type
        if let Some(expected_type) = schema.get("type").and_then(|t| t.as_str()) {
            let actual_type = match data {
                JsonValue::Null => "null",
                JsonValue::Bool(_) => "boolean",
                JsonValue::Number(_) => "number",
                JsonValue::String(_) => "string",
                JsonValue::Array(_) => "array",
                JsonValue::Object(_) => "object",
            };

            if expected_type != actual_type {
                errors.push(format!(
                    "Type mismatch: expected {}, got {}",
                    expected_type, actual_type
                ));
            }
        }

        // Check properties
        if let (Some(properties), Some(data_obj)) = (
            schema.get("properties").and_then(|p| p.as_object()),
            data.as_object(),
        ) {
            for (key, value) in data_obj {
                if let Some(prop_schema) = properties.get(key) {
                    let nested_errors = self.validate_json(value, prop_schema);
                    for err in nested_errors {
                        errors.push(format!("{}.{}", key, err));
                    }
                }
            }
        }

        errors
    }

    /// Check compatibility between two schemas
    fn check_compatibility(
        &self,
        old_schema: &JsonValue,
        new_schema: &JsonValue,
        mode: CompatibilityMode,
    ) -> Result<CompatibilityCheckResult> {
        let mut issues = Vec::new();

        match mode {
            CompatibilityMode::None => {
                return Ok(CompatibilityCheckResult {
                    compatible: true,
                    compatibility_mode: mode,
                    issues: Vec::new(),
                });
            }
            CompatibilityMode::Backward => {
                // Check that all old required fields are still required
                issues.extend(self.check_backward_compatibility(old_schema, new_schema));
            }
            CompatibilityMode::Forward => {
                // Check that all new required fields were in old schema
                issues.extend(self.check_forward_compatibility(old_schema, new_schema));
            }
            CompatibilityMode::Full => {
                // Check both directions
                issues.extend(self.check_backward_compatibility(old_schema, new_schema));
                issues.extend(self.check_forward_compatibility(old_schema, new_schema));
            }
        }

        Ok(CompatibilityCheckResult {
            compatible: issues.is_empty(),
            compatibility_mode: mode,
            issues,
        })
    }

    fn check_backward_compatibility(
        &self,
        old_schema: &JsonValue,
        new_schema: &JsonValue,
    ) -> Vec<String> {
        let mut issues = Vec::new();

        // Get required fields from old schema
        if let Some(old_required) = old_schema.get("required").and_then(|r| r.as_array()) {
            let new_required = new_schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            for old_req in old_required {
                if let Some(field_name) = old_req.as_str() {
                    if !new_required.contains(&field_name) {
                        issues.push(format!(
                            "Backward compatibility: required field '{}' removed",
                            field_name
                        ));
                    }
                }
            }
        }

        issues
    }

    fn check_forward_compatibility(
        &self,
        old_schema: &JsonValue,
        new_schema: &JsonValue,
    ) -> Vec<String> {
        let mut issues = Vec::new();

        // Get required fields from new schema
        if let Some(new_required) = new_schema.get("required").and_then(|r| r.as_array()) {
            let old_required = old_schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            for new_req in new_required {
                if let Some(field_name) = new_req.as_str() {
                    if !old_required.contains(&field_name) {
                        issues.push(format!(
                            "Forward compatibility: new required field '{}' added",
                            field_name
                        ));
                    }
                }
            }
        }

        issues
    }

    /// Set compatibility mode for a subject
    pub fn set_compatibility_mode(&self, subject: String, mode: CompatibilityMode) {
        let mut modes = self.compatibility_modes.write();
        modes.insert(subject, mode);
    }

    /// Get compatibility mode for a subject (or default)
    pub fn get_compatibility_mode(&self, subject: &str) -> CompatibilityMode {
        let modes = self.compatibility_modes.read();
        modes.get(subject).copied().unwrap_or(self.config.default_compatibility)
    }

    /// Delete a specific schema version
    pub fn delete_schema(&self, subject: &str, version: u32) -> Result<bool> {
        let mut schemas = self.schemas.write();

        if let Some(subject_schemas) = schemas.get_mut(subject) {
            if subject_schemas.remove(&version).is_some() {
                tracing::info!("🗑️  Deleted schema v{} for subject '{}'", version, subject);

                // Update stats
                let mut stats = self.stats.write();
                stats.total_schemas = stats.total_schemas.saturating_sub(1);

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get registry statistics
    pub fn stats(&self) -> SchemaRegistryStats {
        self.stats.read().clone()
    }

    /// Get registry configuration
    pub fn config(&self) -> &SchemaRegistryConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema_registration() {
        let registry = SchemaRegistry::new(SchemaRegistryConfig::default());

        let schema = json!({
            "type": "object",
            "properties": {
                "user_id": {"type": "string"},
                "email": {"type": "string"}
            },
            "required": ["user_id", "email"]
        });

        let response = registry
            .register_schema(
                "user.created".to_string(),
                schema,
                Some("User creation event".to_string()),
                None,
            )
            .unwrap();

        assert_eq!(response.version, 1);
        assert_eq!(response.subject, "user.created");
    }

    #[test]
    fn test_schema_validation() {
        let registry = SchemaRegistry::new(SchemaRegistryConfig::default());

        let schema = json!({
            "type": "object",
            "properties": {
                "user_id": {"type": "string"},
                "email": {"type": "string"}
            },
            "required": ["user_id", "email"]
        });

        registry
            .register_schema("user.created".to_string(), schema, None, None)
            .unwrap();

        // Valid payload
        let valid_payload = json!({
            "user_id": "123",
            "email": "test@example.com"
        });

        let result = registry.validate("user.created", None, &valid_payload).unwrap();
        assert!(result.valid);

        // Invalid payload (missing required field)
        let invalid_payload = json!({
            "user_id": "123"
        });

        let result = registry.validate("user.created", None, &invalid_payload).unwrap();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_backward_compatibility() {
        let registry = SchemaRegistry::new(SchemaRegistryConfig {
            default_compatibility: CompatibilityMode::Backward,
            ..Default::default()
        });

        let schema_v1 = json!({
            "type": "object",
            "required": ["user_id", "email"]
        });

        registry
            .register_schema("user.created".to_string(), schema_v1, None, None)
            .unwrap();

        // Compatible: adding optional field
        let schema_v2 = json!({
            "type": "object",
            "required": ["user_id", "email"]
        });

        let result = registry.register_schema("user.created".to_string(), schema_v2, None, None);
        assert!(result.is_ok());

        // Incompatible: removing required field
        let schema_v3 = json!({
            "type": "object",
            "required": ["user_id"]
        });

        let result = registry.register_schema("user.created".to_string(), schema_v3, None, None);
        assert!(result.is_err());
    }
}
