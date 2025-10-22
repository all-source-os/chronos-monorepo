use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::Event;

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
