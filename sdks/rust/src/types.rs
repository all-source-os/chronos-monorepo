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

/// Sort order for [`QueryEventsParams`] results.
///
/// Core orders events by `(timestamp, version)`. [`SortOrder::Asc`] returns the
/// oldest event first (the default, preserving event-replay semantics);
/// [`SortOrder::Desc`] returns the newest first. Combined with `limit`, `Desc`
/// lets you fetch the latest events for an entity without folding the whole
/// stream client-side — e.g. `?entity_id=<id>&limit=1&order=desc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Oldest event first, ordered by `(timestamp, version)`. This is the default.
    #[default]
    Asc,
    /// Newest event first, ordered by `(timestamp, version)`.
    Desc,
}

impl SortOrder {
    /// The query-string value Core expects (`"asc"` or `"desc"`).
    pub fn as_str(self) -> &'static str {
        match self {
            SortOrder::Asc => "asc",
            SortOrder::Desc => "desc",
        }
    }
}

/// Query parameters for filtering events. Uses builder pattern.
///
/// Supports two event type filters:
/// - [`event_type`](Self::event_type): exact match (e.g., `"order.placed"`)
/// - [`event_type_prefix`](Self::event_type_prefix): prefix match (e.g., `"order."` matches
///   `"order.placed"`, `"order.shipped"`, etc.)
///
/// Core applies a default limit of 100 events. Use [`has_more`](QueryEventsResponse::has_more)
/// on the response to check if more results are available, then paginate with `offset`.
///
/// Results are ordered by `(timestamp, version)`. The default is ascending
/// (oldest first); pass [`order`](Self::order) (or [`order_desc`](Self::order_desc))
/// for newest-first. Because the order is well-defined, `limit`/`offset`
/// pagination is stable across pages.
///
/// # Example: querying all index.* events with prefix
/// ```
/// # use allsource::QueryEventsParams;
/// let params = QueryEventsParams::new()
///     .event_type_prefix("index.")
///     .limit(50);
/// ```
///
/// # Example: fetch the latest event for an entity
/// ```
/// # use allsource::QueryEventsParams;
/// let params = QueryEventsParams::new()
///     .entity_id("user-123")
///     .order_desc()
///     .limit(1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct QueryEventsParams {
    pub entity_id: Option<String>,
    pub event_type: Option<String>,
    /// Filter by event type prefix. For example, `"index."` matches `"index.created"`,
    /// `"index.strategy.updated"`, `"index.deleted"`, etc.
    /// This is mutually exclusive with `event_type` (exact match).
    pub event_type_prefix: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub since: Option<String>,
    pub until: Option<String>,
    /// Filter by payload fields. Keys and values are matched exactly against top-level payload fields.
    /// Multiple entries use AND logic (all must match).
    pub payload_filter: Option<std::collections::HashMap<String, String>>,
    /// Sort order for the returned events, by `(timestamp, version)`.
    /// `None` leaves Core's default (ascending / oldest first) in effect.
    pub order: Option<SortOrder>,
}

impl QueryEventsParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entity_id(mut self, id: &str) -> Self {
        self.entity_id = Some(id.to_string());
        self
    }

    /// Filter by exact event type (e.g., `"order.placed"`).
    /// For prefix matching, use [`event_type_prefix`](Self::event_type_prefix) instead.
    pub fn event_type(mut self, t: &str) -> Self {
        self.event_type = Some(t.to_string());
        self
    }

    /// Filter by event type prefix (e.g., `"order."` matches `"order.placed"`, `"order.shipped"`).
    /// For exact matching, use [`event_type`](Self::event_type) instead.
    pub fn event_type_prefix(mut self, prefix: &str) -> Self {
        self.event_type_prefix = Some(prefix.to_string());
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

    /// Add a payload field filter. Events must have this key-value pair in their payload.
    /// Call multiple times for AND logic (all must match).
    pub fn payload_filter(mut self, key: &str, value: &str) -> Self {
        self.payload_filter
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Set the sort order of returned events (by `(timestamp, version)`).
    pub fn order(mut self, order: SortOrder) -> Self {
        self.order = Some(order);
        self
    }

    /// Return events newest first. Shorthand for `.order(SortOrder::Desc)`.
    pub fn order_desc(self) -> Self {
        self.order(SortOrder::Desc)
    }

    /// Return events oldest first (Core's default). Shorthand for `.order(SortOrder::Asc)`.
    pub fn order_asc(self) -> Self {
        self.order(SortOrder::Asc)
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
        if let Some(ref v) = self.event_type_prefix {
            pairs.push(("event_type_prefix", v.clone()));
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
        if let Some(ref v) = self.payload_filter {
            if let Ok(json) = serde_json::to_string(v) {
                pairs.push(("payload_filter", json));
            }
        }
        if let Some(v) = self.order {
            pairs.push(("order", v.as_str().to_string()));
        }
        pairs
    }
}

/// Response from querying events.
///
/// Core returns `{"events": [...], "count": N, "total_count": N, "has_more": bool}`.
/// Query Service may return `{"data": [...], "count": N}`.
/// This struct accepts both via serde alias.
///
/// The `total_count` and `has_more` fields are `Option` for backward compatibility
/// with older Core versions that do not include them.
///
/// `events` is ordered by `(timestamp, version)` — ascending (oldest first) by
/// default, or descending when the request set [`QueryEventsParams::order`] to
/// [`SortOrder::Desc`]. This ordering is stable, so `limit`/`offset` pagination
/// over a sorted view is well-defined.
#[derive(Debug, Clone, Deserialize)]
pub struct QueryEventsResponse {
    pub count: u64,
    #[serde(alias = "data")]
    pub events: Vec<Event>,
    /// Total number of matching events (before limit). `None` if the server does not support it.
    pub total_count: Option<u64>,
    /// Whether more results are available beyond the current page. `None` if not supported.
    pub has_more: Option<bool>,
}

/// Summary of a single entity returned by the list-entities endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct EntitySummary {
    pub entity_id: String,
    pub event_count: u64,
    pub last_event_type: String,
    pub last_event_at: String,
}

/// Response from listing entities.
#[derive(Debug, Clone, Deserialize)]
pub struct ListEntitiesResponse {
    pub entities: Vec<EntitySummary>,
    pub total: u64,
    pub has_more: bool,
}

/// A group of entities that share the same payload field values (duplicates).
#[derive(Debug, Clone, Deserialize)]
pub struct DuplicateGroup {
    /// The shared field values that define this group
    pub key: Value,
    /// Entity IDs in this group
    pub entity_ids: Vec<String>,
    /// Number of entities in this group
    pub count: u64,
}

/// Response from duplicate entity detection.
#[derive(Debug, Clone, Deserialize)]
pub struct DetectDuplicatesResponse {
    /// Groups where count > 1
    pub duplicates: Vec<DuplicateGroup>,
    /// Total number of duplicate groups found
    pub total: u64,
    /// Whether more results are available
    pub has_more: bool,
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

/// Durable consumer state as reported by Core's `/api/v1/consumers/:id`.
///
/// `cursor_position` is the WAL offset up to which this consumer has acked.
/// A freshly-registered consumer has `cursor_position: None` (will replay
/// from the beginning on next connection).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsumerState {
    pub consumer_id: String,
    #[serde(default)]
    pub event_type_filters: Vec<String>,
    #[serde(default)]
    pub cursor_position: Option<u64>,
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
        assert!(pairs
            .iter()
            .any(|(k, v)| *k == "entity_id" && v == "order-123"));
        assert!(pairs.iter().any(|(k, v)| *k == "limit" && v == "10"));
    }

    #[test]
    fn test_query_params_order_desc() {
        let params = QueryEventsParams::new()
            .entity_id("user-123")
            .order_desc()
            .limit(1);
        let pairs = params.to_query_pairs();
        assert!(pairs.iter().any(|(k, v)| *k == "order" && v == "desc"));
    }

    #[test]
    fn test_query_params_order_omitted_by_default() {
        let params = QueryEventsParams::new().entity_id("e1");
        let pairs = params.to_query_pairs();
        assert!(
            !pairs.iter().any(|(k, _)| *k == "order"),
            "order must not be sent unless set, so Core's default applies"
        );
    }

    #[test]
    fn test_sort_order_as_str() {
        assert_eq!(SortOrder::Asc.as_str(), "asc");
        assert_eq!(SortOrder::Desc.as_str(), "desc");
        assert_eq!(SortOrder::default(), SortOrder::Asc);
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
