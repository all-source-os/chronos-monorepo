//! Schema enforcement for Prime node and edge types.
//!
//! Schemas are stored as events (`prime.schema.registered`) so they're part of
//! the event stream and survive restarts. Validation happens before event
//! ingestion (fail fast).
//!
//! Schemas are optional — if no schema is registered for a type, any properties
//! are accepted.
//!
//! Uses a lightweight required-fields approach (no full JSON Schema library) to
//! keep dependencies minimal. The schema `Value` is stored as-is for future
//! extensibility.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::{
    application::services::projection::Projection,
    domain::entities::Event,
    error::Result,
};

/// Event type for schema registration.
pub const SCHEMA_REGISTERED: &str = "prime.schema.registered";

/// A registered schema entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEntry {
    /// The node type or edge relation this schema applies to.
    pub type_name: String,
    /// Whether this is a node schema or edge schema.
    pub kind: SchemaKind,
    /// The schema definition. Currently expects `{"required": ["field1", "field2"]}`.
    pub schema: Value,
}

/// Whether the schema applies to nodes or edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaKind {
    Node,
    Edge,
}

/// Validation error when properties don't match the schema.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub type_name: String,
    pub missing_fields: Vec<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "validation failed for {}: missing required fields: {}",
            self.type_name,
            self.missing_fields.join(", ")
        )
    }
}

/// Projection that maintains registered schemas and provides validation.
pub struct SchemaProjection {
    name: String,
    /// node_type -> SchemaEntry
    node_schemas: Arc<DashMap<String, SchemaEntry>>,
    /// relation -> SchemaEntry
    edge_schemas: Arc<DashMap<String, SchemaEntry>>,
}

impl SchemaProjection {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            node_schemas: Arc::new(DashMap::new()),
            edge_schemas: Arc::new(DashMap::new()),
        }
    }

    /// Register a schema for a node type.
    pub fn register_node_schema(&self, node_type: &str, schema: Value) {
        self.node_schemas.insert(
            node_type.to_string(),
            SchemaEntry {
                type_name: node_type.to_string(),
                kind: SchemaKind::Node,
                schema,
            },
        );
    }

    /// Register a schema for an edge relation.
    pub fn register_edge_schema(&self, relation: &str, schema: Value) {
        self.edge_schemas.insert(
            relation.to_string(),
            SchemaEntry {
                type_name: relation.to_string(),
                kind: SchemaKind::Edge,
                schema,
            },
        );
    }

    /// Validate node properties against the registered schema.
    /// Returns `Ok(())` if no schema is registered or validation passes.
    pub fn validate_node(&self, node_type: &str, properties: &Value) -> std::result::Result<(), ValidationError> {
        if let Some(entry) = self.node_schemas.get(node_type) {
            validate_properties(node_type, &entry.schema, properties)
        } else {
            Ok(()) // No schema = accept anything
        }
    }

    /// Validate edge properties against the registered schema.
    /// Returns `Ok(())` if no schema is registered or validation passes.
    pub fn validate_edge(
        &self,
        relation: &str,
        properties: Option<&Value>,
    ) -> std::result::Result<(), ValidationError> {
        if let Some(entry) = self.edge_schemas.get(relation) {
            let props = properties.cloned().unwrap_or(Value::Object(Default::default()));
            validate_properties(relation, &entry.schema, &props)
        } else {
            Ok(())
        }
    }

    /// List all registered schemas.
    pub fn schemas(&self) -> Vec<SchemaEntry> {
        let mut result: Vec<SchemaEntry> = self
            .node_schemas
            .iter()
            .map(|e| e.value().clone())
            .collect();
        result.extend(self.edge_schemas.iter().map(|e| e.value().clone()));
        result
    }
}

/// Validate properties against a schema. Currently supports `required` field.
fn validate_properties(
    type_name: &str,
    schema: &Value,
    properties: &Value,
) -> std::result::Result<(), ValidationError> {
    let mut missing = Vec::new();

    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        for field in required {
            if let Some(field_name) = field.as_str()
                && properties.get(field_name).is_none()
            {
                missing.push(field_name.to_string());
            }
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(ValidationError {
            type_name: type_name.to_string(),
            missing_fields: missing,
        })
    }
}

impl Projection for SchemaProjection {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&self, event: &Event) -> Result<()> {
        if event.event_type_str() != SCHEMA_REGISTERED {
            return Ok(());
        }

        let payload = &event.payload;
        let type_name = match payload.get("type_name").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return Ok(()),
        };
        let kind = match payload.get("kind").and_then(|v| v.as_str()) {
            Some("node") => SchemaKind::Node,
            Some("edge") => SchemaKind::Edge,
            _ => return Ok(()),
        };
        let schema = match payload.get("schema") {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        match kind {
            SchemaKind::Node => self.register_node_schema(&type_name, schema),
            SchemaKind::Edge => self.register_edge_schema(&type_name, schema),
        }

        Ok(())
    }

    fn get_state(&self, _entity_id: &str) -> Option<Value> {
        serde_json::to_value(self.schemas()).ok()
    }

    fn clear(&self) {
        self.node_schemas.clear();
        self.edge_schemas.clear();
    }

    fn snapshot(&self) -> Option<Value> {
        serde_json::to_value(self.schemas()).ok()
    }

    fn restore(&self, snapshot: &Value) -> Result<()> {
        let entries: Vec<SchemaEntry> = serde_json::from_value(snapshot.clone())
            .map_err(|e| crate::error::AllSourceError::StorageError(e.to_string()))?;
        self.node_schemas.clear();
        self.edge_schemas.clear();
        for entry in entries {
            match entry.kind {
                SchemaKind::Node => {
                    self.node_schemas.insert(entry.type_name.clone(), entry);
                }
                SchemaKind::Edge => {
                    self.edge_schemas.insert(entry.type_name.clone(), entry);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_validate_node_with_required_fields_passes() {
        let proj = SchemaProjection::new("schema");
        proj.register_node_schema(
            "person",
            serde_json::json!({"required": ["name"]}),
        );

        let result = proj.validate_node("person", &serde_json::json!({"name": "Alice"}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_node_missing_required_field_fails() {
        let proj = SchemaProjection::new("schema");
        proj.register_node_schema(
            "person",
            serde_json::json!({"required": ["name", "email"]}),
        );

        let result = proj.validate_node("person", &serde_json::json!({"name": "Alice"}));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.missing_fields, vec!["email"]);
    }

    #[test]
    fn test_validate_node_no_schema_accepts_anything() {
        let proj = SchemaProjection::new("schema");

        let result = proj.validate_node("unknown_type", &serde_json::json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_edge_with_schema() {
        let proj = SchemaProjection::new("schema");
        proj.register_edge_schema(
            "works_on",
            serde_json::json!({"required": ["since"]}),
        );

        let result = proj.validate_edge(
            "works_on",
            Some(&serde_json::json!({"since": "2026-01"})),
        );
        assert!(result.is_ok());

        let result = proj.validate_edge("works_on", Some(&serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_edge_no_properties_fails_if_required() {
        let proj = SchemaProjection::new("schema");
        proj.register_edge_schema(
            "works_on",
            serde_json::json!({"required": ["since"]}),
        );

        let result = proj.validate_edge("works_on", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_edge_no_schema_accepts_anything() {
        let proj = SchemaProjection::new("schema");

        let result = proj.validate_edge("unknown_relation", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schemas_list() {
        let proj = SchemaProjection::new("schema");
        proj.register_node_schema("person", serde_json::json!({"required": ["name"]}));
        proj.register_edge_schema("works_on", serde_json::json!({"required": ["since"]}));

        let schemas = proj.schemas();
        assert_eq!(schemas.len(), 2);
    }

    #[test]
    fn test_process_schema_registered_event() {
        let proj = SchemaProjection::new("schema");

        let event = Event::reconstruct_from_strings(
            Uuid::new_v4(),
            SCHEMA_REGISTERED.to_string(),
            "schema:person".to_string(),
            "default".to_string(),
            serde_json::json!({
                "type_name": "person",
                "kind": "node",
                "schema": {"required": ["name"]},
            }),
            Utc::now(),
            None,
            1,
        );
        proj.process(&event).unwrap();

        let result = proj.validate_node("person", &serde_json::json!({}));
        assert!(result.is_err());

        let result = proj.validate_node("person", &serde_json::json!({"name": "Alice"}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_snapshot_and_restore() {
        let proj = SchemaProjection::new("schema");
        proj.register_node_schema("person", serde_json::json!({"required": ["name"]}));

        let snap = proj.snapshot().expect("snapshot should be Some");

        proj.clear();
        assert!(proj.validate_node("person", &serde_json::json!({})).is_ok()); // no schema = pass

        proj.restore(&snap).unwrap();
        assert!(proj.validate_node("person", &serde_json::json!({})).is_err()); // schema restored
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError {
            type_name: "person".to_string(),
            missing_fields: vec!["name".to_string(), "email".to_string()],
        };
        assert_eq!(
            err.to_string(),
            "validation failed for person: missing required fields: name, email"
        );
    }
}
