use async_trait::async_trait;
use crate::domain::entities::{Event, EventStream};
use crate::domain::value_objects::{EntityId, PartitionKey};
use crate::error::Result;

/// Event Stream Repository Trait (SierraDB Pattern)
///
/// This repository manages EventStreams with:
/// - Fixed partitioning for horizontal scaling
/// - Gapless version guarantees via watermarks
/// - Optimistic locking for concurrency control
///
/// Based on production-tested patterns from SierraDB event store.
///
/// # Design Principles
/// - Partition-aware operations for scalability
/// - Watermark tracking prevents gaps in version sequences
/// - Optimistic locking detects concurrent modifications
/// - Domain layer interface, infrastructure implements
#[async_trait]
pub trait EventStreamRepository: Send + Sync {
    /// Get or create an event stream for the given entity
    ///
    /// If the stream doesn't exist, creates a new one with version 0.
    /// The partition is automatically assigned based on entity ID.
    ///
    /// # SierraDB Pattern
    /// - Consistent hashing ensures same entity → same partition
    /// - New streams start at version 0, watermark 0
    async fn get_or_create_stream(&self, stream_id: &EntityId) -> Result<EventStream>;

    /// Append an event to a stream with optimistic locking
    ///
    /// Returns the new version number on success.
    /// Fails with ConcurrencyError if expected_version doesn't match.
    ///
    /// # SierraDB Pattern
    /// - Checks optimistic lock (expected_version)
    /// - Increments version atomically
    /// - Advances watermark (gapless guarantee)
    ///
    /// # Example
    /// ```ignore
    /// let mut stream = repo.get_or_create_stream(&entity_id).await?;
    /// stream.expect_version(5); // Optimistic lock
    /// let version = repo.append_to_stream(&mut stream, event).await?;
    /// assert_eq!(version, 6);
    /// ```
    async fn append_to_stream(&self, stream: &mut EventStream, event: Event) -> Result<u64>;

    /// Save stream state (for persistence)
    ///
    /// Persists the entire stream including all events, version, watermark.
    async fn save_stream(&self, stream: &EventStream) -> Result<()>;

    /// Load stream by entity ID
    ///
    /// Returns None if stream doesn't exist.
    async fn load_stream(&self, stream_id: &EntityId) -> Result<Option<EventStream>>;

    /// Get all streams in a specific partition
    ///
    /// Used for:
    /// - Load balancing across partitions
    /// - Partition-level operations
    /// - Compaction and maintenance
    ///
    /// # SierraDB Pattern
    /// - Fixed partitions (32 for single-node)
    /// - Enables sequential writes within partition
    /// - Ready for horizontal scaling
    async fn get_streams_by_partition(&self, partition_key: &PartitionKey) -> Result<Vec<EventStream>>;

    /// Get watermark for a stream
    ///
    /// The watermark represents the "highest continuously confirmed version".
    /// All versions <= watermark are guaranteed gapless.
    ///
    /// # SierraDB Pattern
    /// - Prevents gaps in version sequences
    /// - Critical for proper event replay
    /// - Watermark <= current_version always
    async fn get_watermark(&self, stream_id: &EntityId) -> Result<u64>;

    /// Verify stream integrity (gapless check)
    ///
    /// Returns true if all versions from 1..=watermark exist.
    /// Used for debugging and validation.
    async fn verify_gapless(&self, stream_id: &EntityId) -> Result<bool>;

    /// Get total number of streams
    async fn count_streams(&self) -> Result<usize>;

    /// Get partition statistics
    ///
    /// Returns (partition_id, stream_count) for each partition.
    /// Used for monitoring partition distribution.
    async fn partition_stats(&self) -> Result<Vec<(u32, usize)>>;

    /// Get all streams for a specific tenant
    ///
    /// Returns all event streams that contain events belonging to the specified tenant.
    /// Used for tenant-scoped queries and operations.
    ///
    /// # Tenant Isolation
    /// This method is critical for multi-tenancy support. It ensures that
    /// operations can be scoped to a single tenant for security and compliance.
    async fn get_streams_by_tenant(&self, tenant_id: &crate::domain::value_objects::TenantId) -> Result<Vec<EventStream>>;

    /// Count streams for a specific tenant
    ///
    /// Returns the total number of streams belonging to the specified tenant.
    /// Used for quota enforcement and tenant monitoring.
    async fn count_streams_by_tenant(&self, tenant_id: &crate::domain::value_objects::TenantId) -> Result<usize>;
}

/// Read-only stream repository (query optimization)
///
/// Following Interface Segregation Principle.
#[async_trait]
pub trait EventStreamReader: Send + Sync {
    async fn load_stream(&self, stream_id: &EntityId) -> Result<Option<EventStream>>;
    async fn get_watermark(&self, stream_id: &EntityId) -> Result<u64>;
    async fn get_streams_by_partition(&self, partition_key: &PartitionKey) -> Result<Vec<EventStream>>;
    async fn verify_gapless(&self, stream_id: &EntityId) -> Result<bool>;
    async fn get_streams_by_tenant(&self, tenant_id: &crate::domain::value_objects::TenantId) -> Result<Vec<EventStream>>;
    async fn count_streams_by_tenant(&self, tenant_id: &crate::domain::value_objects::TenantId) -> Result<usize>;
}

/// Write-only stream repository (ingestion optimization)
#[async_trait]
pub trait EventStreamWriter: Send + Sync {
    async fn get_or_create_stream(&self, stream_id: &EntityId) -> Result<EventStream>;
    async fn append_to_stream(&self, stream: &mut EventStream, event: Event) -> Result<u64>;
    async fn save_stream(&self, stream: &EventStream) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // These are trait definition tests - implementation tests will be in infrastructure layer

    #[test]
    fn test_trait_bounds() {
        // Verify traits have correct bounds
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Box<dyn EventStreamRepository>>();
        assert_send_sync::<Box<dyn EventStreamReader>>();
        assert_send_sync::<Box<dyn EventStreamWriter>>();
    }
}
