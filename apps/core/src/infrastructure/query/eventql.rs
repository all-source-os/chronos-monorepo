//! EventQL — SQL-like query language for AllSource events
//!
//! Uses Apache DataFusion to execute SQL queries over events materialized
//! as Arrow RecordBatches. Events are registered as a virtual table named
//! `events` with columns: id, event_type, entity_id, tenant_id, payload,
//! metadata, timestamp, version.
//!
//! ## Supported syntax
//!
//! ```sql
//! SELECT * FROM events WHERE event_type = 'user.created' LIMIT 10
//! SELECT entity_id, COUNT(*) as cnt FROM events GROUP BY entity_id ORDER BY cnt DESC
//! SELECT * FROM events WHERE timestamp > '2026-01-01T00:00:00Z' AND entity_id = 'user-123'
//! ```
//!
//! ## Limitations
//!
//! - `payload` and `metadata` are stored as UTF-8 JSON strings (use JSON functions or LIKE for filtering)
//! - No JOINs across other tables (events is the only registered table)
//! - Query operates on an in-memory snapshot of events at execution time

use arrow::{
    array::{Array, Int64Array, RecordBatch, StringArray, TimestampMicrosecondArray},
    datatypes::{DataType, Field, Schema, TimeUnit},
};
use datafusion::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::entities::Event;

/// EventQL query request
#[derive(Debug, Clone, Deserialize)]
pub struct EventQLRequest {
    /// SQL query string (table name must be `events`)
    pub query: String,
    /// Optional tenant_id filter applied before SQL execution
    pub tenant_id: Option<String>,
}

/// A single row in EventQL results
#[derive(Debug, Clone, Serialize)]
pub struct EventQLRow {
    /// Column values as JSON
    pub columns: Vec<serde_json::Value>,
}

/// EventQL query response
#[derive(Debug, Clone, Serialize)]
pub struct EventQLResponse {
    /// Column names from the result schema
    pub columns: Vec<String>,
    /// Result rows
    pub rows: Vec<EventQLRow>,
    /// Number of rows returned
    pub row_count: usize,
}

/// Build the Arrow schema for the events table
fn events_schema() -> Schema {
    Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("event_type", DataType::Utf8, false),
        Field::new("entity_id", DataType::Utf8, false),
        Field::new("tenant_id", DataType::Utf8, false),
        Field::new("payload", DataType::Utf8, false),
        Field::new("metadata", DataType::Utf8, true),
        Field::new(
            "timestamp",
            DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            false,
        ),
        Field::new("version", DataType::Int64, false),
    ])
}

/// Convert a slice of Events into an Arrow RecordBatch
fn events_to_record_batch(events: &[Event]) -> Result<RecordBatch, arrow::error::ArrowError> {
    let schema = Arc::new(events_schema());

    let ids: Vec<String> = events.iter().map(|e| e.id.to_string()).collect();
    let event_types: Vec<String> = events
        .iter()
        .map(|e| e.event_type_str().to_string())
        .collect();
    let entity_ids: Vec<String> = events
        .iter()
        .map(|e| e.entity_id_str().to_string())
        .collect();
    let tenant_ids: Vec<String> = events
        .iter()
        .map(|e| e.tenant_id_str().to_string())
        .collect();
    let payloads: Vec<String> = events.iter().map(|e| e.payload.to_string()).collect();
    let metadatas: Vec<Option<String>> = events
        .iter()
        .map(|e| e.metadata.as_ref().map(|m| m.to_string()))
        .collect();
    let timestamps: Vec<i64> = events
        .iter()
        .map(|e| e.timestamp.timestamp_micros())
        .collect();
    let versions: Vec<i64> = events.iter().map(|e| e.version).collect();

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(event_types)),
            Arc::new(StringArray::from(entity_ids)),
            Arc::new(StringArray::from(tenant_ids)),
            Arc::new(StringArray::from(payloads)),
            Arc::new(StringArray::from(metadatas)),
            Arc::new(TimestampMicrosecondArray::from(timestamps).with_timezone("UTC")),
            Arc::new(Int64Array::from(versions)),
        ],
    )
}

/// Execute an EventQL query against a set of events
pub async fn execute_eventql(
    events: &[Event],
    request: &EventQLRequest,
) -> Result<EventQLResponse, String> {
    // Validate: reject destructive statements
    let lower = request.query.trim().to_lowercase();
    if lower.starts_with("insert")
        || lower.starts_with("update")
        || lower.starts_with("delete")
        || lower.starts_with("drop")
        || lower.starts_with("alter")
        || lower.starts_with("create")
    {
        return Err("EventQL is read-only: only SELECT queries are allowed".to_string());
    }

    // Apply tenant filter if specified
    let filtered: Vec<&Event> = if let Some(ref tid) = request.tenant_id {
        events.iter().filter(|e| e.tenant_id_str() == tid).collect()
    } else {
        events.iter().collect()
    };

    // Convert to Arrow
    let owned: Vec<Event> = filtered.into_iter().cloned().collect();
    let batch =
        events_to_record_batch(&owned).map_err(|e| format!("Arrow conversion error: {e}"))?;

    // Create DataFusion context and register the events table
    let ctx = SessionContext::new();
    ctx.register_batch("events", batch)
        .map_err(|e| format!("Failed to register events table: {e}"))?;

    // Execute the SQL query
    let df = ctx
        .sql(&request.query)
        .await
        .map_err(|e| format!("SQL error: {e}"))?;

    let result_batches = df
        .collect()
        .await
        .map_err(|e| format!("Execution error: {e}"))?;

    // Extract column names from the result schema
    let columns: Vec<String> = if let Some(first) = result_batches.first() {
        first
            .schema()
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect()
    } else {
        return Ok(EventQLResponse {
            columns: vec![],
            rows: vec![],
            row_count: 0,
        });
    };

    // Convert result batches to JSON rows
    let mut rows = Vec::new();
    for batch in &result_batches {
        for row_idx in 0..batch.num_rows() {
            let mut col_values = Vec::new();
            for col_idx in 0..batch.num_columns() {
                let col = batch.column(col_idx);
                let json_val = if col.is_null(row_idx) {
                    serde_json::Value::Null
                } else {
                    // Use arrow's display formatter for generic value extraction
                    let formatted = arrow::util::display::array_value_to_string(col, row_idx)
                        .unwrap_or_else(|_| "null".to_string());
                    if formatted == "null" {
                        serde_json::Value::Null
                    } else if let Ok(n) = formatted.parse::<i64>() {
                        serde_json::Value::Number(n.into())
                    } else if let Ok(f) = formatted.parse::<f64>() {
                        serde_json::json!(f)
                    } else {
                        serde_json::Value::String(formatted)
                    }
                };
                col_values.push(json_val);
            }
            rows.push(EventQLRow {
                columns: col_values,
            });
        }
    }

    let row_count = rows.len();
    Ok(EventQLResponse {
        columns,
        rows,
        row_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Event;

    fn make_test_events() -> Vec<Event> {
        vec![
            Event::from_strings(
                "user.created".to_string(),
                "user-1".to_string(),
                "tenant-a".to_string(),
                serde_json::json!({"name": "Alice", "age": 30}),
                Some(serde_json::json!({"source": "web"})),
            )
            .unwrap(),
            Event::from_strings(
                "user.created".to_string(),
                "user-2".to_string(),
                "tenant-a".to_string(),
                serde_json::json!({"name": "Bob", "age": 25}),
                None,
            )
            .unwrap(),
            Event::from_strings(
                "order.placed".to_string(),
                "order-1".to_string(),
                "tenant-a".to_string(),
                serde_json::json!({"total": 99.99}),
                None,
            )
            .unwrap(),
            Event::from_strings(
                "user.created".to_string(),
                "user-3".to_string(),
                "tenant-b".to_string(),
                serde_json::json!({"name": "Charlie"}),
                None,
            )
            .unwrap(),
        ]
    }

    #[tokio::test]
    async fn test_select_all() {
        let events = make_test_events();
        let req = EventQLRequest {
            query: "SELECT id, event_type, entity_id FROM events".to_string(),
            tenant_id: None,
        };
        let result = execute_eventql(&events, &req).await.unwrap();
        assert_eq!(result.row_count, 4);
        assert_eq!(result.columns, vec!["id", "event_type", "entity_id"]);
    }

    #[tokio::test]
    async fn test_where_filter() {
        let events = make_test_events();
        let req = EventQLRequest {
            query: "SELECT entity_id FROM events WHERE event_type = 'order.placed'".to_string(),
            tenant_id: None,
        };
        let result = execute_eventql(&events, &req).await.unwrap();
        assert_eq!(result.row_count, 1);
    }

    #[tokio::test]
    async fn test_group_by_count() {
        let events = make_test_events();
        let req = EventQLRequest {
            query: "SELECT event_type, COUNT(*) as cnt FROM events GROUP BY event_type ORDER BY cnt DESC".to_string(),
            tenant_id: None,
        };
        let result = execute_eventql(&events, &req).await.unwrap();
        assert_eq!(result.row_count, 2); // user.created and order.placed
        assert_eq!(result.columns, vec!["event_type", "cnt"]);
    }

    #[tokio::test]
    async fn test_tenant_filter() {
        let events = make_test_events();
        let req = EventQLRequest {
            query: "SELECT * FROM events".to_string(),
            tenant_id: Some("tenant-b".to_string()),
        };
        let result = execute_eventql(&events, &req).await.unwrap();
        assert_eq!(result.row_count, 1);
    }

    #[tokio::test]
    async fn test_limit() {
        let events = make_test_events();
        let req = EventQLRequest {
            query: "SELECT * FROM events LIMIT 2".to_string(),
            tenant_id: None,
        };
        let result = execute_eventql(&events, &req).await.unwrap();
        assert_eq!(result.row_count, 2);
    }

    #[tokio::test]
    async fn test_reject_insert() {
        let events = make_test_events();
        let req = EventQLRequest {
            query: "INSERT INTO events VALUES ('a','b','c','d','{}',NULL,0,1)".to_string(),
            tenant_id: None,
        };
        let result = execute_eventql(&events, &req).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-only"));
    }

    #[tokio::test]
    async fn test_empty_events() {
        let events: Vec<Event> = vec![];
        let req = EventQLRequest {
            query: "SELECT * FROM events".to_string(),
            tenant_id: None,
        };
        let result = execute_eventql(&events, &req).await.unwrap();
        assert_eq!(result.row_count, 0);
    }

    #[tokio::test]
    async fn test_order_by() {
        let events = make_test_events();
        let req = EventQLRequest {
            query: "SELECT entity_id FROM events ORDER BY entity_id ASC".to_string(),
            tenant_id: None,
        };
        let result = execute_eventql(&events, &req).await.unwrap();
        assert_eq!(result.row_count, 4);
    }

    #[tokio::test]
    async fn test_payload_like() {
        let events = make_test_events();
        let req = EventQLRequest {
            query: "SELECT entity_id FROM events WHERE payload LIKE '%Alice%'".to_string(),
            tenant_id: None,
        };
        let result = execute_eventql(&events, &req).await.unwrap();
        assert_eq!(result.row_count, 1);
    }

    #[tokio::test]
    async fn test_reject_drop() {
        let events = make_test_events();
        let req = EventQLRequest {
            query: "DROP TABLE events".to_string(),
            tenant_id: None,
        };
        let result = execute_eventql(&events, &req).await;
        assert!(result.is_err());
    }
}
