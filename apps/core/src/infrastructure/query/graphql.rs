//! Lightweight GraphQL API for AllSource events and projections
//!
//! Implements a minimal GraphQL executor (no external dependencies) that supports:
//! - Query `events(entity_id, event_type, limit)` → list of events
//! - Query `event(id)` → single event by ID
//! - Query `projections` → list projection names
//! - Query `projection(name, entity_id)` → projection state
//! - Query `stats` → store statistics
//!
//! ## Example query
//!
//! ```graphql
//! {
//!   events(event_type: "user.created", limit: 10) {
//!     id
//!     event_type
//!     entity_id
//!     payload
//!     timestamp
//!   }
//! }
//! ```
//!
//! ## Limitations
//!
//! - No mutations (read-only API)
//! - No subscriptions (use WebSocket streaming instead)
//! - No fragments, variables, or aliases
//! - Field selection is advisory (all requested fields returned, unrequested fields omitted)

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// GraphQL request body
#[derive(Debug, Clone, Deserialize)]
pub struct GraphQLRequest {
    pub query: String,
    #[serde(default)]
    pub variables: Option<HashMap<String, JsonValue>>,
}

/// GraphQL response body
#[derive(Debug, Clone, Serialize)]
pub struct GraphQLResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<GraphQLError>,
}

/// A GraphQL error
#[derive(Debug, Clone, Serialize)]
pub struct GraphQLError {
    pub message: String,
}

/// Parsed top-level query field
#[derive(Debug, Clone)]
pub struct QueryField {
    pub name: String,
    pub arguments: HashMap<String, String>,
    pub fields: Vec<String>,
}

/// Parse a simple GraphQL query into query fields
///
/// Supports: `{ fieldName(arg: "value", arg2: "value2") { subfield1 subfield2 } }`
pub fn parse_query(query: &str) -> Result<Vec<QueryField>, String> {
    let trimmed = query.trim();

    // Strip optional "query" keyword and find the outer braces
    let body = if let Some(rest) = trimmed.strip_prefix("query") {
        let rest = rest.trim();
        // Skip optional query name
        if let Some(idx) = rest.find('{') {
            &rest[idx..]
        } else {
            return Err("Expected '{' after query keyword".to_string());
        }
    } else if trimmed.starts_with('{') {
        trimmed
    } else if trimmed.starts_with("mutation") {
        return Err("Mutations are not supported (read-only API)".to_string());
    } else if trimmed.starts_with("subscription") {
        return Err("Subscriptions are not supported (use WebSocket streaming)".to_string());
    } else {
        return Err("Expected query to start with '{' or 'query'".to_string());
    };

    // Remove outer braces
    let body = body
        .strip_prefix('{')
        .and_then(|b| b.trim().strip_suffix('}'))
        .ok_or("Malformed query: missing outer braces")?
        .trim();

    if body.is_empty() {
        return Err("Empty query body".to_string());
    }

    let mut fields = Vec::new();
    let mut remaining = body;

    while !remaining.is_empty() {
        remaining = remaining.trim();
        if remaining.is_empty() {
            break;
        }

        // Extract field name
        let name_end = remaining
            .find(|c: char| c == '(' || c == '{' || c.is_whitespace())
            .unwrap_or(remaining.len());
        let name = remaining[..name_end].trim().to_string();
        if name.is_empty() {
            break;
        }
        remaining = remaining[name_end..].trim();

        // Extract arguments if present
        let mut arguments = HashMap::new();
        if remaining.starts_with('(') {
            let close = remaining.find(')').ok_or("Unclosed argument list")?;
            let args_str = &remaining[1..close];
            for arg in args_str.split(',') {
                let arg = arg.trim();
                if let Some((key, val)) = arg.split_once(':') {
                    let key = key.trim().to_string();
                    let val = val.trim().trim_matches('"').to_string();
                    arguments.insert(key, val);
                }
            }
            remaining = remaining[close + 1..].trim();
        }

        // Extract sub-fields if present
        let mut sub_fields = Vec::new();
        if remaining.starts_with('{') {
            let close = find_matching_brace(remaining).ok_or("Unclosed field selection")?;
            let fields_str = &remaining[1..close];
            for field in fields_str.split_whitespace() {
                let field = field.trim();
                if !field.is_empty() {
                    sub_fields.push(field.to_string());
                }
            }
            remaining = remaining[close + 1..].trim();
        }

        fields.push(QueryField {
            name,
            arguments,
            fields: sub_fields,
        });
    }

    if fields.is_empty() {
        return Err("No fields found in query".to_string());
    }

    Ok(fields)
}

/// Find the matching closing brace for the opening brace at position 0
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.chars().enumerate() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Convert an Event to a filtered JSON object based on requested fields
pub fn event_to_json(event: &crate::domain::entities::Event, fields: &[String]) -> JsonValue {
    let mut map = serde_json::Map::new();

    let all_fields = fields.is_empty();

    if all_fields || fields.iter().any(|f| f == "id") {
        map.insert("id".to_string(), JsonValue::String(event.id.to_string()));
    }
    if all_fields || fields.iter().any(|f| f == "event_type") {
        map.insert(
            "event_type".to_string(),
            JsonValue::String(event.event_type_str().to_string()),
        );
    }
    if all_fields || fields.iter().any(|f| f == "entity_id") {
        map.insert(
            "entity_id".to_string(),
            JsonValue::String(event.entity_id_str().to_string()),
        );
    }
    if all_fields || fields.iter().any(|f| f == "tenant_id") {
        map.insert(
            "tenant_id".to_string(),
            JsonValue::String(event.tenant_id_str().to_string()),
        );
    }
    if all_fields || fields.iter().any(|f| f == "payload") {
        map.insert("payload".to_string(), event.payload.clone());
    }
    if all_fields || fields.iter().any(|f| f == "metadata") {
        map.insert(
            "metadata".to_string(),
            event.metadata.clone().unwrap_or(JsonValue::Null),
        );
    }
    if all_fields || fields.iter().any(|f| f == "timestamp") {
        map.insert(
            "timestamp".to_string(),
            JsonValue::String(event.timestamp.to_rfc3339()),
        );
    }
    if all_fields || fields.iter().any(|f| f == "version") {
        map.insert("version".to_string(), serde_json::json!(event.version));
    }

    JsonValue::Object(map)
}

/// Introspection schema response for `__schema` queries
pub fn introspection_schema() -> JsonValue {
    serde_json::json!({
        "queryType": { "name": "Query" },
        "types": [
            {
                "name": "Event",
                "fields": [
                    { "name": "id", "type": "String!" },
                    { "name": "event_type", "type": "String!" },
                    { "name": "entity_id", "type": "String!" },
                    { "name": "tenant_id", "type": "String!" },
                    { "name": "payload", "type": "JSON!" },
                    { "name": "metadata", "type": "JSON" },
                    { "name": "timestamp", "type": "DateTime!" },
                    { "name": "version", "type": "Int!" }
                ]
            },
            {
                "name": "Query",
                "fields": [
                    { "name": "events", "args": ["entity_id", "event_type", "tenant_id", "limit"], "type": "[Event!]!" },
                    { "name": "event", "args": ["id"], "type": "Event" },
                    { "name": "projections", "type": "[String!]!" },
                    { "name": "projection", "args": ["name", "entity_id"], "type": "JSON" },
                    { "name": "stats", "type": "JSON!" }
                ]
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_query() {
        let q = r"{ events { id event_type } }";
        let fields = parse_query(q).unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "events");
        assert_eq!(fields[0].fields, vec!["id", "event_type"]);
    }

    #[test]
    fn test_parse_with_arguments() {
        let q = r#"{ events(event_type: "user.created", limit: "10") { id entity_id } }"#;
        let fields = parse_query(q).unwrap();
        assert_eq!(
            fields[0].arguments.get("event_type").unwrap(),
            "user.created"
        );
        assert_eq!(fields[0].arguments.get("limit").unwrap(), "10");
    }

    #[test]
    fn test_parse_multiple_fields() {
        let q = r"{ events { id } stats }";
        let fields = parse_query(q).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "events");
        assert_eq!(fields[1].name, "stats");
    }

    #[test]
    fn test_parse_query_keyword() {
        let q = r"query { events { id } }";
        let fields = parse_query(q).unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "events");
    }

    #[test]
    fn test_reject_mutation() {
        let q = r#"mutation { createEvent(type: "test") { id } }"#;
        let result = parse_query(q);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Mutations"));
    }

    #[test]
    fn test_reject_subscription() {
        let q = r"subscription { newEvents { id } }";
        let result = parse_query(q);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Subscriptions"));
    }

    #[test]
    fn test_empty_query() {
        let q = r"{ }";
        let result = parse_query(q);
        assert!(result.is_err());
    }

    #[test]
    fn test_event_to_json_all_fields() {
        let event = crate::domain::entities::Event::from_strings(
            "user.created".to_string(),
            "user-1".to_string(),
            "default".to_string(),
            serde_json::json!({"name": "Alice"}),
            None,
        )
        .unwrap();
        let json = event_to_json(&event, &[]);
        assert!(json.get("id").is_some());
        assert!(json.get("event_type").is_some());
        assert!(json.get("payload").is_some());
    }

    #[test]
    fn test_event_to_json_selected_fields() {
        let event = crate::domain::entities::Event::from_strings(
            "user.created".to_string(),
            "user-1".to_string(),
            "default".to_string(),
            serde_json::json!({"name": "Alice"}),
            None,
        )
        .unwrap();
        let json = event_to_json(&event, &["id".to_string(), "event_type".to_string()]);
        assert!(json.get("id").is_some());
        assert!(json.get("event_type").is_some());
        assert!(json.get("payload").is_none());
    }

    #[test]
    fn test_introspection_schema() {
        let schema = introspection_schema();
        assert!(schema.get("queryType").is_some());
        assert!(schema.get("types").is_some());
    }
}
