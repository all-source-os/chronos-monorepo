use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::{Event, EventStream};
use crate::domain::repositories::{EventStreamRepository, EventStreamReader, EventStreamWriter};
use crate::domain::value_objects::{EntityId, PartitionKey, TenantId};
use crate::error::{AllSourceError, Result};

/// In-memory implementation of EventStreamRepository
///
/// Uses thread-safe RwLock for concurrent access.
/// Suitable for:
/// - Development and testing
/// - Single-node deployments
/// - High-performance in-memory event store
///
/// # SierraDB Patterns
/// - Partitioning: Streams tracked by partition for distribution
/// - Gapless Versions: EventStream enforces sequential versions
/// - Optimistic Locking: Version conflicts detected in append
///
/// # Thread Safety
/// - Uses parking_lot::RwLock for better performance
/// - Multiple readers, single writer
/// - No poisoning on panic (parking_lot feature)
#[derive(Clone)]
pub struct InMemoryEventStreamRepository {
    /// Streams indexed by entity ID
    streams: Arc<RwLock<HashMap<String, EventStream>>>,
}

impl InMemoryEventStreamRepository {
    /// Create a new in-memory repository
    pub fn new() -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Clear all streams (for testing)
    #[cfg(test)]
    pub fn clear(&self) {
        self.streams.write().clear();
    }
}

impl Default for InMemoryEventStreamRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventStreamRepository for InMemoryEventStreamRepository {
    async fn get_or_create_stream(&self, stream_id: &EntityId) -> Result<EventStream> {
        let key = stream_id.as_str().to_string();

        // Try to get existing stream
        {
            let streams = self.streams.read();
            if let Some(stream) = streams.get(&key) {
                return Ok(stream.clone());
            }
        }

        // Create new stream if not exists
        let new_stream = EventStream::new(stream_id.clone());

        {
            let mut streams = self.streams.write();
            // Double-check after acquiring write lock (race condition)
            if let Some(stream) = streams.get(&key) {
                return Ok(stream.clone());
            }
            streams.insert(key, new_stream.clone());
        }

        Ok(new_stream)
    }

    async fn append_to_stream(&self, stream: &mut EventStream, event: Event) -> Result<u64> {
        // Append to the stream (handles optimistic locking)
        let version = stream.append_event(event)?;

        // Persist the updated stream
        let key = stream.stream_id().as_str().to_string();
        let mut streams = self.streams.write();
        streams.insert(key, stream.clone());

        Ok(version)
    }

    async fn save_stream(&self, stream: &EventStream) -> Result<()> {
        let key = stream.stream_id().as_str().to_string();
        let mut streams = self.streams.write();
        streams.insert(key, stream.clone());
        Ok(())
    }

    async fn load_stream(&self, stream_id: &EntityId) -> Result<Option<EventStream>> {
        let key = stream_id.as_str().to_string();
        let streams = self.streams.read();
        Ok(streams.get(&key).cloned())
    }

    async fn get_streams_by_partition(&self, partition_key: &PartitionKey) -> Result<Vec<EventStream>> {
        let streams = self.streams.read();
        let target_partition = partition_key.partition_id();

        let matching_streams: Vec<EventStream> = streams
            .values()
            .filter(|stream| stream.partition_key().partition_id() == target_partition)
            .cloned()
            .collect();

        Ok(matching_streams)
    }

    async fn get_watermark(&self, stream_id: &EntityId) -> Result<u64> {
        let key = stream_id.as_str().to_string();
        let streams = self.streams.read();
        match streams.get(&key) {
            Some(s) => Ok(s.watermark()),
            None => Ok(0),
        }
    }

    async fn verify_gapless(&self, stream_id: &EntityId) -> Result<bool> {
        let key = stream_id.as_str().to_string();
        let streams = self.streams.read();
        match streams.get(&key) {
            Some(s) => Ok(s.is_gapless()),
            None => Ok(true), // Empty stream is gapless
        }
    }

    async fn count_streams(&self) -> Result<usize> {
        let streams = self.streams.read();
        Ok(streams.len())
    }

    async fn partition_stats(&self) -> Result<Vec<(u32, usize)>> {
        let streams = self.streams.read();
        let mut stats: HashMap<u32, usize> = HashMap::new();

        for stream in streams.values() {
            let partition_id = stream.partition_key().partition_id();
            *stats.entry(partition_id).or_insert(0) += 1;
        }

        let mut result: Vec<(u32, usize)> = stats.into_iter().collect();
        result.sort_by_key(|(partition_id, _)| *partition_id);

        Ok(result)
    }

    async fn get_streams_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<EventStream>> {
        let streams = self.streams.read();

        let mut result = Vec::new();
        for stream in streams.values() {
            // Check if stream belongs to this tenant
            if let Some(stream_tenant) = stream.tenant_id() {
                if stream_tenant == tenant_id {
                    result.push(stream.clone());
                }
            }
        }

        Ok(result)
    }

    async fn count_streams_by_tenant(&self, tenant_id: &TenantId) -> Result<usize> {
        let streams = self.streams.read();

        let count = streams.values()
            .filter(|stream| {
                stream.tenant_id().map(|t| t == tenant_id).unwrap_or(false)
            })
            .count();

        Ok(count)
    }
}

#[async_trait]
impl EventStreamReader for InMemoryEventStreamRepository {
    async fn load_stream(&self, stream_id: &EntityId) -> Result<Option<EventStream>> {
        EventStreamRepository::load_stream(self, stream_id).await
    }

    async fn get_watermark(&self, stream_id: &EntityId) -> Result<u64> {
        EventStreamRepository::get_watermark(self, stream_id).await
    }

    async fn get_streams_by_partition(&self, partition_key: &PartitionKey) -> Result<Vec<EventStream>> {
        EventStreamRepository::get_streams_by_partition(self, partition_key).await
    }

    async fn verify_gapless(&self, stream_id: &EntityId) -> Result<bool> {
        EventStreamRepository::verify_gapless(self, stream_id).await
    }

    async fn get_streams_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<EventStream>> {
        EventStreamRepository::get_streams_by_tenant(self, tenant_id).await
    }

    async fn count_streams_by_tenant(&self, tenant_id: &TenantId) -> Result<usize> {
        EventStreamRepository::count_streams_by_tenant(self, tenant_id).await
    }
}

#[async_trait]
impl EventStreamWriter for InMemoryEventStreamRepository {
    async fn get_or_create_stream(&self, stream_id: &EntityId) -> Result<EventStream> {
        EventStreamRepository::get_or_create_stream(self, stream_id).await
    }

    async fn append_to_stream(&self, stream: &mut EventStream, event: Event) -> Result<u64> {
        EventStreamRepository::append_to_stream(self, stream, event).await
    }

    async fn save_stream(&self, stream: &EventStream) -> Result<()> {
        EventStreamRepository::save_stream(self, stream).await
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

    #[tokio::test]
    async fn test_get_or_create_stream() {
        let repo = InMemoryEventStreamRepository::new();
        let entity_id = EntityId::new("entity-1".to_string()).unwrap();

        let stream = EventStreamRepository::get_or_create_stream(&repo, &entity_id).await.unwrap();
        assert_eq!(stream.current_version(), 0);
        assert_eq!(stream.watermark(), 0);

        // Get same stream again
        let stream2 = EventStreamRepository::get_or_create_stream(&repo, &entity_id).await.unwrap();
        assert_eq!(stream2.stream_id(), stream.stream_id());
    }

    #[tokio::test]
    async fn test_append_to_stream() {
        let repo = InMemoryEventStreamRepository::new();
        let entity_id = EntityId::new("entity-1".to_string()).unwrap();

        let mut stream = EventStreamRepository::get_or_create_stream(&repo, &entity_id).await.unwrap();
        let event = create_test_event("entity-1");

        let version = EventStreamRepository::append_to_stream(&repo, &mut stream, event).await.unwrap();
        assert_eq!(version, 1);
        assert_eq!(stream.current_version(), 1);
        assert_eq!(stream.watermark(), 1);
    }

    #[tokio::test]
    async fn test_optimistic_locking() {
        let repo = InMemoryEventStreamRepository::new();
        let entity_id = EntityId::new("entity-1".to_string()).unwrap();

        let mut stream = EventStreamRepository::get_or_create_stream(&repo, &entity_id).await.unwrap();

        // Append first event
        let event1 = create_test_event("entity-1");
        EventStreamRepository::append_to_stream(&repo, &mut stream, event1).await.unwrap();

        // Set wrong expected version
        stream.expect_version(0);
        let event2 = create_test_event("entity-1");
        let result = EventStreamRepository::append_to_stream(&repo, &mut stream, event2).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(AllSourceError::ConcurrencyError(_))));
    }

    #[tokio::test]
    async fn test_get_streams_by_partition() {
        let repo = InMemoryEventStreamRepository::new();

        // Create multiple streams
        for i in 0..10 {
            let entity_id = EntityId::new(format!("entity-{}", i)).unwrap();
            EventStreamRepository::get_or_create_stream(&repo, &entity_id).await.unwrap();
        }

        // Get streams for partition 0
        let partition_key = PartitionKey::from_partition_id(0, 32).unwrap();
        let streams = EventStreamRepository::get_streams_by_partition(&repo, &partition_key).await.unwrap();

        // All returned streams should be in partition 0
        for stream in &streams {
            assert_eq!(stream.partition_key().partition_id(), 0);
        }
    }

    #[tokio::test]
    async fn test_watermark() {
        let repo = InMemoryEventStreamRepository::new();
        let entity_id = EntityId::new("entity-1".to_string()).unwrap();

        let mut stream = EventStreamRepository::get_or_create_stream(&repo, &entity_id).await.unwrap();

        // Append 3 events
        for _ in 0..3 {
            let event = create_test_event("entity-1");
            EventStreamRepository::append_to_stream(&repo, &mut stream, event).await.unwrap();
        }

        let watermark = EventStreamRepository::get_watermark(&repo, &entity_id).await.unwrap();
        assert_eq!(watermark, 3);
    }

    #[tokio::test]
    async fn test_verify_gapless() {
        let repo = InMemoryEventStreamRepository::new();
        let entity_id = EntityId::new("entity-1".to_string()).unwrap();

        // Empty stream is gapless
        assert!(EventStreamRepository::verify_gapless(&repo, &entity_id).await.unwrap());

        let mut stream = EventStreamRepository::get_or_create_stream(&repo, &entity_id).await.unwrap();

        // Append events
        for _ in 0..5 {
            let event = create_test_event("entity-1");
            EventStreamRepository::append_to_stream(&repo, &mut stream, event).await.unwrap();
        }

        // Should still be gapless
        assert!(EventStreamRepository::verify_gapless(&repo, &entity_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_partition_stats() {
        let repo = InMemoryEventStreamRepository::new();

        // Create 100 streams
        for i in 0..100 {
            let entity_id = EntityId::new(format!("entity-{}", i)).unwrap();
            EventStreamRepository::get_or_create_stream(&repo, &entity_id).await.unwrap();
        }

        let stats = EventStreamRepository::partition_stats(&repo).await.unwrap();

        // Should have multiple partitions
        assert!(!stats.is_empty());

        // Total streams should equal 100
        let total: usize = stats.iter().map(|(_, count)| count).sum();
        assert_eq!(total, 100);
    }

    #[tokio::test]
    async fn test_count_streams() {
        let repo = InMemoryEventStreamRepository::new();
        assert_eq!(EventStreamRepository::count_streams(&repo).await.unwrap(), 0);

        for i in 0..10 {
            let entity_id = EntityId::new(format!("entity-{}", i)).unwrap();
            EventStreamRepository::get_or_create_stream(&repo, &entity_id).await.unwrap();
        }

        assert_eq!(EventStreamRepository::count_streams(&repo).await.unwrap(), 10);
    }
}
