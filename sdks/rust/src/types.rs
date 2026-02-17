use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// An event to ingest into AllSource.
#[derive(Debug, Clone, Serialize)]
pub struct IngestEventInput {
    /// Event type (e.g., "user.signup", "order.placed").
    pub event_type: String,
    /// Entity this event belongs to (e.g., "user-123", "order-456").
    pub entity_id: String,
    /// Arbitrary JSON payload.
    pub payload: Value,
    /// Optional metadata (correlation IDs, source info).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// A stored event returned from AllSource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: String,
    pub entity_id: String,
    pub payload: Value,
    #[serde(default)]
    pub metadata: Value,
    pub timestamp: String,
    #[serde(default)]
    pub version: Option<i64>,
    #[serde(default)]
    pub tenant_id: Option<String>,
}

/// Query parameters for filtering events. Uses builder pattern.
#[derive(Debug, Clone, Default)]
pub struct QueryEventsParams {
    pub entity_id: Option<String>,
    pub event_type: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub since: Option<String>,
    pub until: Option<String>,
}

impl QueryEventsParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entity_id(mut self, id: &str) -> Self {
        self.entity_id = Some(id.to_string());
        self
    }

    pub fn event_type(mut self, t: &str) -> Self {
        self.event_type = Some(t.to_string());
        self
    }

    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn offset(mut self, n: u32) -> Self {
        self.offset = Some(n);
        self
    }

    pub fn since(mut self, ts: &str) -> Self {
        self.since = Some(ts.to_string());
        self
    }

    pub fn until(mut self, ts: &str) -> Self {
        self.until = Some(ts.to_string());
        self
    }

    /// Convert to query string pairs for reqwest.
    pub(crate) fn to_query_pairs(&self) -> Vec<(&str, String)> {
        let mut pairs = Vec::new();
        if let Some(ref v) = self.entity_id {
            pairs.push(("entity_id", v.clone()));
        }
        if let Some(ref v) = self.event_type {
            pairs.push(("event_type", v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit", v.to_string()));
        }
        if let Some(v) = self.offset {
            pairs.push(("offset", v.to_string()));
        }
        if let Some(ref v) = self.since {
            pairs.push(("since", v.clone()));
        }
        if let Some(ref v) = self.until {
            pairs.push(("until", v.clone()));
        }
        pairs
    }
}

/// Response from querying events.
///
/// Core returns `{"events": [...], "count": N}`.
/// Query Service may return `{"data": [...], "count": N}`.
/// This struct accepts both via serde alias.
#[derive(Debug, Clone, Deserialize)]
pub struct QueryEventsResponse {
    pub count: u64,
    #[serde(alias = "data")]
    pub events: Vec<Event>,
}

/// Response from ingesting an event via Core.
#[derive(Debug, Clone, Deserialize)]
pub struct IngestResponse {
    pub event_id: String,
    pub timestamp: String,
}

/// Response from batch ingesting events via Core.
///
/// Core returns `{total, ingested, events: [{event_id, timestamp}]}`.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchIngestResponse {
    pub total: u64,
    pub ingested: u64,
    pub events: Vec<IngestResponse>,
}

/// A projection state.
#[derive(Debug, Clone, Deserialize)]
pub struct Projection {
    pub name: String,
    #[serde(default)]
    pub state: Value,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Projections list response.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectionsResponse {
    pub projections: Vec<Projection>,
    pub total: u64,
}

/// Health check response.
#[derive(Debug, Clone, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_response_core_format() {
        let json = r#"{"events": [{"id": "abc", "event_type": "test", "entity_id": "e1", "payload": {}, "timestamp": "2026-01-01T00:00:00Z", "version": 1, "tenant_id": "default"}], "count": 1}"#;
        let resp: QueryEventsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.count, 1);
        assert_eq!(resp.events.len(), 1);
        assert_eq!(resp.events[0].event_type, "test");
    }

    #[test]
    fn test_query_response_qs_format() {
        let json = r#"{"data": [{"id": "abc", "event_type": "test", "entity_id": "e1", "payload": {}, "timestamp": "2026-01-01T00:00:00Z"}], "count": 1}"#;
        let resp: QueryEventsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.count, 1);
        assert_eq!(resp.events.len(), 1);
    }

    #[test]
    fn test_ingest_response_deser() {
        let json = r#"{"event_id": "550e8400-e29b-41d4-a716-446655440000", "timestamp": "2026-01-01T00:00:00Z"}"#;
        let resp: IngestResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.event_id.is_empty());
    }

    #[test]
    fn test_batch_response_deser() {
        let json = r#"{"total": 2, "ingested": 2, "events": [{"event_id": "a", "timestamp": "t1"}, {"event_id": "b", "timestamp": "t2"}]}"#;
        let resp: BatchIngestResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total, 2);
        assert_eq!(resp.ingested, 2);
        assert_eq!(resp.events.len(), 2);
    }

    #[test]
    fn test_query_params_builder() {
        let params = QueryEventsParams::new()
            .entity_id("order-123")
            .event_type("order.placed")
            .limit(10)
            .since("2026-01-01T00:00:00Z");
        let pairs = params.to_query_pairs();
        assert_eq!(pairs.len(), 4);
        assert!(pairs.iter().any(|(k, v)| *k == "entity_id" && v == "order-123"));
        assert!(pairs.iter().any(|(k, v)| *k == "limit" && v == "10"));
    }

    #[test]
    fn test_health_response_with_extras() {
        let json = r#"{"status": "healthy", "version": "0.10.5", "uptime_seconds": 3600}"#;
        let resp: HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, "healthy");
        assert_eq!(resp.version.as_deref(), Some("0.10.5"));
        assert!(resp.extra.contains_key("uptime_seconds"));
    }
}
