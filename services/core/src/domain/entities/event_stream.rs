use crate::domain::entities::Event;
use crate::domain::value_objects::{EntityId, PartitionKey};
use crate::error::{AllSourceError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event Stream aggregate enforcing gapless version numbers
///
/// Inspired by SierraDB's watermark pattern for consistent event sourcing.
/// Ensures no gaps in version sequences, critical for proper event replay.
///
/// # SierraDB Pattern
/// - Watermark tracks "highest continuously confirmed sequence"
/// - Prevents gaps that would break event sourcing guarantees
/// - Uses optimistic locking for concurrency control
///
/// # Invariants
/// - Versions start at 1 and increment sequentially
/// - No gaps allowed in version sequence
/// - Watermark <= max version always
/// - All versions below watermark are confirmed (gapless)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStream {
    /// Stream identifier (usually entity ID)
    stream_id: EntityId,

    /// Partition key for distribution
    partition_key: PartitionKey,

    /// Current version (last event)
    current_version: u64,

    /// Watermark: highest continuously confirmed version
    /// All versions <= watermark are guaranteed gapless
    watermark: u64,

    /// Events in this stream
    events: Vec<Event>,

    /// Expected version for optimistic locking
    /// Used to detect concurrent modifications
    expected_version: Option<u64>,

    /// Stream metadata
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl EventStream {
    /// Create a new event stream
    pub fn new(stream_id: EntityId) -> Self {
        let partition_key = PartitionKey::from_entity_id(stream_id.as_str());
        let now = Utc::now();

        Self {
            stream_id,
            partition_key,
            current_version: 0,
            watermark: 0,
            events: Vec::new(),
            expected_version: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Reconstruct an EventStream from persistent storage
    ///
    /// Used by repository implementations to restore streams from database.
    /// Bypasses validation since data is already validated at creation time.
    ///
    /// # Arguments
    /// - `stream_id`: Entity ID of the stream
    /// - `partition_key`: Pre-computed partition assignment
    /// - `current_version`: Latest version number
    /// - `watermark`: Highest continuously confirmed version
    /// - `events`: All events in the stream
    /// - `expected_version`: Optional optimistic lock version
    /// - `created_at`: Stream creation timestamp
    /// - `updated_at`: Last modification timestamp
    pub fn reconstruct(
        stream_id: EntityId,
        partition_key: PartitionKey,
        current_version: u64,
        watermark: u64,
        events: Vec<Event>,
        expected_version: Option<u64>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self> {
        // Basic validation
        if watermark > current_version {
            return Err(AllSourceError::InvalidInput(format!(
                "Watermark ({}) cannot exceed current version ({})",
                watermark, current_version
            )));
        }

        if events.len() as u64 != current_version {
            return Err(AllSourceError::InvalidInput(format!(
                "Event count ({}) must match current version ({})",
                events.len(), current_version
            )));
        }

        Ok(Self {
            stream_id,
            partition_key,
            current_version,
            watermark,
            events,
            expected_version,
            created_at,
            updated_at,
        })
    }

    /// Append an event with optimistic locking
    ///
    /// # SierraDB Pattern
    /// - Checks expected_version matches current_version
    /// - Prevents concurrent modification conflicts
    /// - Ensures gapless version sequence
    pub fn append_event(&mut self, event: Event) -> Result<u64> {
        // Optimistic locking check
        if let Some(expected) = self.expected_version {
            if expected != self.current_version {
                return Err(AllSourceError::ConcurrencyError(format!(
                    "Version conflict: expected {}, got {}",
                    expected, self.current_version
                )));
            }
        }

        // Increment version
        self.current_version += 1;
        let new_version = self.current_version;

        // Store event
        self.events.push(event);

        // Advance watermark (all previous events confirmed)
        self.watermark = new_version;

        self.updated_at = Utc::now();

        Ok(new_version)
    }

    /// Set expected version for next append (optimistic locking)
    pub fn expect_version(&mut self, version: u64) {
        self.expected_version = Some(version);
    }

    /// Clear expected version
    pub fn clear_expected_version(&mut self) {
        self.expected_version = None;
    }

    /// Get events from version (inclusive)
    pub fn events_from(&self, from_version: u64) -> Vec<&Event> {
        if from_version == 0 || from_version > self.current_version {
            return Vec::new();
        }

        let start_idx = (from_version - 1) as usize;
        self.events[start_idx..].iter().collect()
    }

    /// Check if stream has gapless versions up to watermark
    pub fn is_gapless(&self) -> bool {
        if self.watermark > self.current_version {
            return false; // Watermark shouldn't exceed current version
        }

        // Check all versions up to watermark exist
        for version in 1..=self.watermark {
            let idx = (version - 1) as usize;
            if idx >= self.events.len() {
                return false;
            }
        }

        true
    }

    // Getters
    pub fn stream_id(&self) -> &EntityId {
        &self.stream_id
    }

    pub fn partition_key(&self) -> &PartitionKey {
        &self.partition_key
    }

    pub fn current_version(&self) -> u64 {
        self.current_version
    }

    pub fn watermark(&self) -> u64 {
        self.watermark
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_event(entity_id: &str) -> Event {
        Event::from_strings(
            "test.event".to_string(),
            entity_id.to_string(),
            "default".to_string(),
            json!({"data": "test"}),
            None,
        )
        .unwrap()
    }

    #[test]
    fn test_new_stream() {
        let stream_id = EntityId::new("stream-1".to_string()).unwrap();
        let stream = EventStream::new(stream_id.clone());

        assert_eq!(stream.current_version(), 0);
        assert_eq!(stream.watermark(), 0);
        assert_eq!(stream.event_count(), 0);
        assert!(stream.is_gapless());
    }

    #[test]
    fn test_append_event() {
        let stream_id = EntityId::new("stream-1".to_string()).unwrap();
        let mut stream = EventStream::new(stream_id.clone());

        let event = create_test_event("stream-1");
        let version = stream.append_event(event).unwrap();

        assert_eq!(version, 1);
        assert_eq!(stream.current_version(), 1);
        assert_eq!(stream.watermark(), 1);
        assert_eq!(stream.event_count(), 1);
        assert!(stream.is_gapless());
    }

    #[test]
    fn test_multiple_appends() {
        let stream_id = EntityId::new("stream-1".to_string()).unwrap();
        let mut stream = EventStream::new(stream_id.clone());

        for i in 1..=10 {
            let event = create_test_event("stream-1");
            let version = stream.append_event(event).unwrap();
            assert_eq!(version, i);
        }

        assert_eq!(stream.current_version(), 10);
        assert_eq!(stream.watermark(), 10);
        assert_eq!(stream.event_count(), 10);
        assert!(stream.is_gapless());
    }

    #[test]
    fn test_optimistic_locking_success() {
        let stream_id = EntityId::new("stream-1".to_string()).unwrap();
        let mut stream = EventStream::new(stream_id);

        // Set expected version
        stream.expect_version(0);

        let event = create_test_event("stream-1");
        let result = stream.append_event(event);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_optimistic_locking_failure() {
        let stream_id = EntityId::new("stream-1".to_string()).unwrap();
        let mut stream = EventStream::new(stream_id);

        // Append first event
        let event1 = create_test_event("stream-1");
        stream.append_event(event1).unwrap();

        // Set wrong expected version
        stream.expect_version(0);

        let event2 = create_test_event("stream-1");
        let result = stream.append_event(event2);

        assert!(result.is_err());
        assert!(matches!(result, Err(AllSourceError::ConcurrencyError(_))));
    }

    #[test]
    fn test_events_from() {
        let stream_id = EntityId::new("stream-1".to_string()).unwrap();
        let mut stream = EventStream::new(stream_id);

        // Append 5 events
        for _ in 0..5 {
            let event = create_test_event("stream-1");
            stream.append_event(event).unwrap();
        }

        let events = stream.events_from(3);
        assert_eq!(events.len(), 3); // Events 3, 4, 5
    }

    #[test]
    fn test_partition_assignment() {
        let stream_id = EntityId::new("stream-1".to_string()).unwrap();
        let stream = EventStream::new(stream_id);

        let partition_key = stream.partition_key();
        assert!(partition_key.partition_id() < PartitionKey::DEFAULT_PARTITION_COUNT);
    }

    #[test]
    fn test_clear_expected_version() {
        let stream_id = EntityId::new("stream-1".to_string()).unwrap();
        let mut stream = EventStream::new(stream_id);

        stream.expect_version(0);
        stream.clear_expected_version();

        // Should succeed without version check
        let event = create_test_event("stream-1");
        let result = stream.append_event(event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_events_from_edge_cases() {
        let stream_id = EntityId::new("stream-1".to_string()).unwrap();
        let mut stream = EventStream::new(stream_id);

        // Append 3 events
        for _ in 0..3 {
            let event = create_test_event("stream-1");
            stream.append_event(event).unwrap();
        }

        // Test edge cases
        assert_eq!(stream.events_from(0).len(), 0); // Invalid version 0
        assert_eq!(stream.events_from(1).len(), 3); // From beginning
        assert_eq!(stream.events_from(3).len(), 1); // Last event
        assert_eq!(stream.events_from(4).len(), 0); // Beyond current
    }
}
