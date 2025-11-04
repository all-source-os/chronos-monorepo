use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use crate::domain::entities::{Schema, CompatibilityMode};

/// DTO for registering a new schema
#[derive(Debug, Deserialize)]
pub struct RegisterSchemaRequest {
    pub subject: String,
    pub schema: Value,
    pub compatibility_mode: Option<CompatibilityModeDto>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// DTO for updating a schema
#[derive(Debug, Deserialize)]
pub struct UpdateSchemaRequest {
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub compatibility_mode: Option<CompatibilityModeDto>,
}

/// DTO for compatibility mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityModeDto {
    None,
    Backward,
    Forward,
    Full,
}

impl From<CompatibilityMode> for CompatibilityModeDto {
    fn from(mode: CompatibilityMode) -> Self {
        match mode {
            CompatibilityMode::None => CompatibilityModeDto::None,
            CompatibilityMode::Backward => CompatibilityModeDto::Backward,
            CompatibilityMode::Forward => CompatibilityModeDto::Forward,
            CompatibilityMode::Full => CompatibilityModeDto::Full,
        }
    }
}

impl From<CompatibilityModeDto> for CompatibilityMode {
    fn from(dto: CompatibilityModeDto) -> Self {
        match dto {
            CompatibilityModeDto::None => CompatibilityMode::None,
            CompatibilityModeDto::Backward => CompatibilityMode::Backward,
            CompatibilityModeDto::Forward => CompatibilityMode::Forward,
            CompatibilityModeDto::Full => CompatibilityMode::Full,
        }
    }
}

/// DTO for schema response
#[derive(Debug, Serialize)]
pub struct SchemaDto {
    pub id: Uuid,
    pub subject: String,
    pub version: u32,
    pub schema: Value,
    pub compatibility_mode: CompatibilityModeDto,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl From<&Schema> for SchemaDto {
    fn from(schema: &Schema) -> Self {
        Self {
            id: schema.id(),
            subject: schema.subject().to_string(),
            version: schema.version(),
            schema: schema.schema_definition().clone(),
            compatibility_mode: schema.compatibility_mode().into(),
            description: schema.description().map(String::from),
            tags: schema.tags().to_vec(),
            created_at: schema.created_at(),
        }
    }
}

impl From<Schema> for SchemaDto {
    fn from(schema: Schema) -> Self {
        SchemaDto::from(&schema)
    }
}

/// Response for schema registration
#[derive(Debug, Serialize)]
pub struct RegisterSchemaResponse {
    pub schema: SchemaDto,
}

/// Response for listing schemas
#[derive(Debug, Serialize)]
pub struct ListSchemasResponse {
    pub schemas: Vec<SchemaDto>,
    pub count: usize,
}

/// Request for validating an event against a schema
#[derive(Debug, Deserialize)]
pub struct ValidateEventRequest {
    pub subject: String,
    pub event: Value,
}

/// Response for event validation
#[derive(Debug, Serialize)]
pub struct ValidateEventResponse {
    pub valid: bool,
    pub errors: Option<Vec<String>>,
}
