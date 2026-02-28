use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Input for [`EmbeddedCore::ingest`](super::EmbeddedCore::ingest).
///
/// All fields are plain strings — no value object construction required.
/// In single-tenant mode, `tenant_id` is ignored (always "default").
pub struct IngestEvent<'a> {
    /// The entity this event belongs to (e.g., "order-123").
    pub entity_id: &'a str,
    /// Event type in dot-notation (e.g., "order.placed"). Must be lowercase.
    pub event_type: &'a str,
    /// Arbitrary JSON payload.
    pub payload: serde_json::Value,
    /// Optional metadata (e.g., source system, correlation IDs).
    pub metadata: Option<serde_json::Value>,
    /// Optional tenant ID. In single-tenant mode this is ignored.
    /// In multi-tenant mode, defaults to "default" when `None`.
    pub tenant_id: Option<&'a str>,
}

/// Builder for query parameters passed to [`EmbeddedCore::query`](super::EmbeddedCore::query).
#[derive(Default)]
pub struct Query {
    pub(crate) entity_id: Option<String>,
    pub(crate) event_type: Option<String>,
    pub(crate) event_type_prefix: Option<String>,
    pub(crate) tenant_id: Option<String>,
    pub(crate) limit: Option<usize>,
    pub(crate) since: Option<DateTime<Utc>>,
    pub(crate) until: Option<DateTime<Utc>>,
}

impl Query {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entity_id(mut self, id: impl Into<String>) -> Self {
        self.entity_id = Some(id.into());
        self
    }

    pub fn event_type(mut self, t: impl Into<String>) -> Self {
        self.event_type = Some(t.into());
        self
    }

    pub fn event_type_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.event_type_prefix = Some(prefix.into());
        self
    }

    pub fn tenant_id(mut self, id: impl Into<String>) -> Self {
        self.tenant_id = Some(id.into());
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn since(mut self, t: DateTime<Utc>) -> Self {
        self.since = Some(t);
        self
    }

    pub fn until(mut self, t: DateTime<Utc>) -> Self {
        self.until = Some(t);
        self
    }
}

/// A single event returned from [`EmbeddedCore::query`](super::EmbeddedCore::query).
///
/// All fields are plain Rust types — no value objects exposed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventView {
    pub id: Uuid,
    pub event_type: String,
    pub entity_id: String,
    pub tenant_id: String,
    pub payload: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub version: i64,
}

impl From<&crate::domain::entities::Event> for EventView {
    fn from(e: &crate::domain::entities::Event) -> Self {
        Self {
            id: e.id(),
            event_type: e.event_type_str().to_string(),
            entity_id: e.entity_id_str().to_string(),
            tenant_id: e.tenant_id_str().to_string(),
            payload: e.payload().clone(),
            metadata: e.metadata().cloned(),
            timestamp: e.timestamp(),
            version: e.version(),
        }
    }
}
