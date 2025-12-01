use crate::application::dto::{
    RegisterSchemaRequest, RegisterSchemaResponse, UpdateSchemaRequest, ListSchemasResponse,
    SchemaDto, CompatibilityModeDto,
};
use crate::domain::entities::{Schema, CompatibilityMode};
use crate::error::Result;

/// Use Case: Register Schema
///
/// Registers a new schema or creates a new version of an existing schema.
pub struct RegisterSchemaUseCase;

impl RegisterSchemaUseCase {
    pub fn execute(request: RegisterSchemaRequest) -> Result<RegisterSchemaResponse> {
        // Determine compatibility mode (default to None)
        let compatibility_mode = request
            .compatibility_mode
            .map(CompatibilityMode::from)
            .unwrap_or(CompatibilityMode::None);

        // Create schema (version 1)
        let mut schema = Schema::new_v1(
            request.subject,
            request.schema,
            compatibility_mode,
        )?;

        // Set description if provided
        if let Some(description) = request.description {
            schema.set_description(description)?;
        }

        // Add tags if provided
        if let Some(tags) = request.tags {
            for tag in tags {
                schema.add_tag(tag)?;
            }
        }

        Ok(RegisterSchemaResponse {
            schema: SchemaDto::from(&schema),
        })
    }
}

/// Use Case: Create Next Schema Version
///
/// Creates a new version of an existing schema.
pub struct CreateNextSchemaVersionUseCase;

impl CreateNextSchemaVersionUseCase {
    pub fn execute(
        current_schema: &Schema,
        new_schema_definition: serde_json::Value,
        description: Option<String>,
    ) -> Result<SchemaDto> {
        // Create next version
        let mut next_schema = current_schema.create_next_version(new_schema_definition)?;

        // Set description if provided
        if let Some(desc) = description {
            next_schema.set_description(desc)?;
        }

        // Copy tags from previous version
        for tag in current_schema.tags() {
            next_schema.add_tag(tag.clone())?;
        }

        Ok(SchemaDto::from(&next_schema))
    }
}

/// Use Case: Update Schema Metadata
///
/// Updates schema description and tags (doesn't change the schema itself).
pub struct UpdateSchemaMetadataUseCase;

impl UpdateSchemaMetadataUseCase {
    pub fn execute(mut schema: Schema, request: UpdateSchemaRequest) -> Result<SchemaDto> {
        // Update description if provided
        if let Some(description) = request.description {
            if description.is_empty() {
                schema.clear_description();
            } else {
                schema.set_description(description)?;
            }
        }

        // Update tags if provided
        if let Some(tags) = request.tags {
            // Clear existing tags and add new ones
            for existing_tag in schema.tags().to_vec() {
                schema.remove_tag(&existing_tag);
            }
            for tag in tags {
                schema.add_tag(tag)?;
            }
        }

        Ok(SchemaDto::from(&schema))
    }
}

/// Use Case: List Schemas
///
/// Returns a list of all schemas for a subject.
pub struct ListSchemasUseCase;

impl ListSchemasUseCase {
    pub fn execute(schemas: Vec<Schema>) -> ListSchemasResponse {
        let schema_dtos: Vec<SchemaDto> = schemas.iter().map(SchemaDto::from).collect();
        let count = schema_dtos.len();

        ListSchemasResponse {
            schemas: schema_dtos,
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_register_schema() {
        let request = RegisterSchemaRequest {
            subject: "user.created".to_string(),
            schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "email": {"type": "string"}
                }
            }),
            compatibility_mode: Some(CompatibilityModeDto::Backward),
            description: Some("User creation event schema".to_string()),
            tags: Some(vec!["user".to_string(), "core".to_string()]),
        };

        let response = RegisterSchemaUseCase::execute(request);
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.schema.subject, "user.created");
        assert_eq!(response.schema.version, 1);
        assert_eq!(response.schema.tags.len(), 2);
        assert_eq!(response.schema.compatibility_mode, CompatibilityModeDto::Backward);
    }

    #[test]
    fn test_create_next_version() {
        let schema = Schema::new_v1(
            "order.placed".to_string(),
            json!({"type": "object"}),
            CompatibilityMode::None,
        )
        .unwrap();

        let new_schema = json!({
            "type": "object",
            "properties": {
                "amount": {"type": "number"}
            }
        });

        let result = CreateNextSchemaVersionUseCase::execute(
            &schema,
            new_schema,
            Some("Version 2".to_string()),
        );

        assert!(result.is_ok());
        let next = result.unwrap();
        assert_eq!(next.version, 2);
        assert_eq!(next.subject, "order.placed");
    }

    #[test]
    fn test_update_schema_metadata() {
        let mut schema = Schema::new_v1(
            "test.event".to_string(),
            json!({"type": "object"}),
            CompatibilityMode::None,
        )
        .unwrap();

        schema.add_tag("old-tag".to_string()).unwrap();

        let request = UpdateSchemaRequest {
            description: Some("Updated description".to_string()),
            tags: Some(vec!["new-tag".to_string()]),
            compatibility_mode: None,
        };

        let result = UpdateSchemaMetadataUseCase::execute(schema, request);
        assert!(result.is_ok());

        let updated = result.unwrap();
        assert_eq!(updated.description, Some("Updated description".to_string()));
        assert_eq!(updated.tags, vec!["new-tag".to_string()]);
    }

    #[test]
    fn test_list_schemas() {
        let schemas = vec![
            Schema::new_v1(
                "event.one".to_string(),
                json!({"type": "object"}),
                CompatibilityMode::None,
            )
            .unwrap(),
            Schema::new_v1(
                "event.two".to_string(),
                json!({"type": "object"}),
                CompatibilityMode::Backward,
            )
            .unwrap(),
        ];

        let response = ListSchemasUseCase::execute(schemas);
        assert_eq!(response.count, 2);
        assert_eq!(response.schemas.len(), 2);
    }
}
