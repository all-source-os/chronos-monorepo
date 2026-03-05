use serde::{Deserialize, Serialize};

use super::EventDto;

/// Request to register a durable consumer
#[derive(Debug, Deserialize)]
pub struct RegisterConsumerRequest {
    pub consumer_id: String,
    #[serde(default)]
    pub event_type_filters: Vec<String>,
}

/// Response from consumer registration
#[derive(Debug, Serialize)]
pub struct ConsumerResponse {
    pub consumer_id: String,
    pub event_type_filters: Vec<String>,
    pub cursor_position: Option<u64>,
}

/// Request to acknowledge processed events
#[derive(Debug, Deserialize)]
pub struct AckRequest {
    pub position: u64,
}

/// Response from consumer event polling
#[derive(Debug, Serialize)]
pub struct ConsumerEventsResponse {
    pub events: Vec<ConsumerEventDto>,
    pub count: usize,
}

/// Event DTO with position for consumer polling
#[derive(Debug, Serialize)]
pub struct ConsumerEventDto {
    /// Global offset position (used for acking)
    pub position: u64,
    #[serde(flatten)]
    pub event: EventDto,
}
