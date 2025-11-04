use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::domain::entities::Event;

/// DTO for ingesting a new event
#[derive(Debug, Deserialize)]
pub struct IngestEventRequest {
    pub event_type: String,
    pub entity_id: String,
    pub tenant_id: Option<String>, // Optional, defaults to "default"
    pub payload: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

/// DTO for event ingestion response
#[derive(Debug, Serialize)]
pub struct IngestEventResponse {
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

impl IngestEventResponse {
    pub fn from_event(event: &Event) -> Self {
        Self {
            event_id: event.id(),
            timestamp: event.timestamp(),
        }
    }
}

/// DTO for querying events
#[derive(Debug, Deserialize)]
pub struct QueryEventsRequest {
    /// Filter by entity ID
    pub entity_id: Option<String>,

    /// Filter by event type
    pub event_type: Option<String>,

    /// Tenant ID (required for multi-tenancy)
    pub tenant_id: Option<String>,

    /// Time-travel: get events as of this timestamp
    pub as_of: Option<DateTime<Utc>>,

    /// Get events since this timestamp
    pub since: Option<DateTime<Utc>>,

    /// Get events until this timestamp
    pub until: Option<DateTime<Utc>>,

    /// Limit number of results
    pub limit: Option<usize>,
}

/// DTO for query response
#[derive(Debug, Serialize)]
pub struct QueryEventsResponse {
    pub events: Vec<EventDto>,
    pub count: usize,
}

/// DTO for a single event in responses
#[derive(Debug, Serialize, Clone)]
pub struct EventDto {
    pub id: Uuid,
    pub event_type: String,
    pub entity_id: String,
    pub tenant_id: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
    pub version: i64,
}

impl From<&Event> for EventDto {
    fn from(event: &Event) -> Self {
        Self {
            id: event.id(),
            event_type: event.event_type().to_string(),
            entity_id: event.entity_id().to_string(),
            tenant_id: event.tenant_id().to_string(),
            payload: event.payload().clone(),
            timestamp: event.timestamp(),
            metadata: event.metadata().cloned(),
            version: event.version(),
        }
    }
}

impl From<Event> for EventDto {
    fn from(event: Event) -> Self {
        EventDto::from(&event)
    }
}
