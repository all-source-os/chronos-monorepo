use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Core event structure - immutable, timestamped, and traceable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier
    pub id: Uuid,

    /// Event type (e.g., "user.created", "order.placed")
    pub event_type: String,

    /// Entity this event relates to (e.g., user_id, order_id)
    pub entity_id: String,

    /// Event payload (arbitrary JSON)
    pub payload: serde_json::Value,

    /// Timestamp when event occurred
    pub timestamp: DateTime<Utc>,

    /// Optional metadata
    pub metadata: Option<serde_json::Value>,

    /// Version for optimistic locking
    pub version: i64,
}

impl Event {
    pub fn new(
        event_type: String,
        entity_id: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            entity_id,
            payload,
            timestamp: Utc::now(),
            metadata: None,
            version: 1,
        }
    }
}

/// Request to ingest a new event
#[derive(Debug, Deserialize)]
pub struct IngestEventRequest {
    pub event_type: String,
    pub entity_id: String,
    pub payload: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

/// Response after ingesting an event
#[derive(Debug, Serialize)]
pub struct IngestEventResponse {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

/// Query parameters for retrieving events
#[derive(Debug, Deserialize)]
pub struct QueryEventsRequest {
    /// Filter by entity ID
    pub entity_id: Option<String>,

    /// Filter by event type
    pub event_type: Option<String>,

    /// Time-travel: get events as of this timestamp
    pub as_of: Option<DateTime<Utc>>,

    /// Get events since this timestamp
    pub since: Option<DateTime<Utc>>,

    /// Get events until this timestamp
    pub until: Option<DateTime<Utc>>,

    /// Limit number of results
    pub limit: Option<usize>,
}

/// Response containing queried events
#[derive(Debug, Serialize)]
pub struct QueryEventsResponse {
    pub events: Vec<Event>,
    pub count: usize,
}
