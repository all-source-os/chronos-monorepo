/// PostgreSQL-backed Event Stream Repository
///
/// Production-grade persistent storage implementing SierraDB patterns:
/// - Fixed partitioning for horizontal scaling
/// - Gapless version guarantees via watermarks
/// - Optimistic locking for concurrency control
/// - Transaction management for ACID guarantees
///
/// # Features
/// - **Persistent storage**: Data survives restarts
/// - **Transaction safety**: ACID properties ensured
/// - **Concurrent access**: PostgreSQL handles multi-writer scenarios
/// - **Partition-aware**: Ready for sharding/clustering
/// - **Integrity checks**: Built-in gapless verification
///
/// # Performance
/// - Batch inserts for high throughput
/// - Indexed queries for low latency
/// - Connection pooling for scalability
/// - Prepared statements for efficiency

#[cfg(feature = "postgres")]
use sqlx::{PgPool, Row, Postgres, Transaction};
#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use crate::domain::entities::{Event, EventStream};
#[cfg(feature = "postgres")]
use crate::domain::value_objects::{EntityId, PartitionKey, EventType, TenantId};
#[cfg(feature = "postgres")]
use crate::domain::repositories::EventStreamRepository;
#[cfg(feature = "postgres")]
use crate::error::{AllSourceError, Result};
#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};

#[cfg(feature = "postgres")]
pub struct PostgresEventStreamRepository {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresEventStreamRepository {
    /// Create new PostgreSQL repository with connection pool
    ///
    /// # Arguments
    /// - `pool`: Pre-configured PostgreSQL connection pool
    ///
    /// # Connection Pool Configuration
    /// ```ignore
    /// let pool = PgPoolOptions::new()
    ///     .max_connections(20)
    ///     .connect("postgresql://user:pass@localhost/allsource")
    ///     .await?;
    ///
    /// let repo = PostgresEventStreamRepository::new(pool);
    /// ```
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run database migrations
    ///
    /// Applies all SQL migrations from the `migrations/` directory.
    /// Should be called during application startup.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| AllSourceError::StorageError(format!("Migration failed: {}", e)))?;
        Ok(())
    }

    /// Helper: Load events for a stream
    async fn load_events(
        tx: &mut Transaction<'_, Postgres>,
        stream_id: &str,
    ) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT tenant_id, event_type, entity_id, payload, metadata, timestamp
             FROM events
             WHERE stream_id = $1
             ORDER BY version ASC"
        )
        .bind(stream_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to load events: {}", e)))?;

        let mut events = Vec::new();
        for row in rows {
            let tenant_id: String = row.try_get("tenant_id")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid tenant_id: {}", e)))?;
            let event_type: String = row.try_get("event_type")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid event_type: {}", e)))?;
            let entity_id: String = row.try_get("entity_id")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid entity_id: {}", e)))?;
            let payload: serde_json::Value = row.try_get("payload")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid payload: {}", e)))?;
            let metadata: Option<serde_json::Value> = row.try_get("metadata")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid metadata: {}", e)))?;

            let event = Event::from_strings(
                event_type,
                entity_id,
                tenant_id,
                payload,
                metadata,
            )?;

            events.push(event);
        }

        Ok(events)
    }

    /// Helper: Reconstruct EventStream from database row
    async fn reconstruct_stream(
        tx: &mut Transaction<'_, Postgres>,
        stream_id: &str,
        partition_id: i32,
        current_version: i64,
        watermark: i64,
        expected_version: Option<i64>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<EventStream> {
        // Load events
        let events = Self::load_events(tx, stream_id).await?;

        // Create EntityId
        let entity_id = EntityId::new(stream_id.to_string())?;

        // Create PartitionKey
        let partition_key = PartitionKey::from_partition_id(partition_id as u32, 32)?;

        // Reconstruct EventStream
        EventStream::reconstruct(
            entity_id,
            partition_key,
            current_version as u64,
            watermark as u64,
            events,
            expected_version.map(|v| v as u64),
            created_at,
            updated_at,
        )
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl EventStreamRepository for PostgresEventStreamRepository {
    async fn get_or_create_stream(&self, stream_id: &EntityId) -> Result<EventStream> {
        let mut tx = self.pool.begin().await
            .map_err(|e| AllSourceError::StorageError(format!("Transaction begin failed: {}", e)))?;

        // Try to load existing stream
        let maybe_row = sqlx::query(
            "SELECT stream_id, partition_id, current_version, watermark,
                    expected_version, created_at, updated_at
             FROM event_streams
             WHERE stream_id = $1
             FOR UPDATE"  // Lock row for update
        )
        .bind(stream_id.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to query stream: {}", e)))?;

        let stream = if let Some(row) = maybe_row {
            // Stream exists, reconstruct it
            let partition_id: i32 = row.try_get("partition_id")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid partition_id: {}", e)))?;
            let current_version: i64 = row.try_get("current_version")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid current_version: {}", e)))?;
            let watermark: i64 = row.try_get("watermark")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid watermark: {}", e)))?;
            let expected_version: Option<i64> = row.try_get("expected_version").ok();
            let created_at: DateTime<Utc> = row.try_get("created_at")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid created_at: {}", e)))?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at")
                .map_err(|e| AllSourceError::StorageError(format!("Invalid updated_at: {}", e)))?;

            Self::reconstruct_stream(
                &mut tx,
                stream_id.as_str(),
                partition_id,
                current_version,
                watermark,
                expected_version,
                created_at,
                updated_at,
            ).await?
        } else {
            // Stream doesn't exist, create new one
            let stream = EventStream::new(stream_id.clone());

            sqlx::query(
                "INSERT INTO event_streams
                 (stream_id, partition_id, current_version, watermark, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(stream_id.as_str())
            .bind(stream.partition_key().partition_id() as i32)
            .bind(stream.current_version() as i64)
            .bind(stream.watermark() as i64)
            .bind(stream.created_at())
            .bind(stream.updated_at())
            .execute(&mut *tx)
            .await
            .map_err(|e| AllSourceError::StorageError(format!("Failed to create stream: {}", e)))?;

            stream
        };

        tx.commit().await
            .map_err(|e| AllSourceError::StorageError(format!("Transaction commit failed: {}", e)))?;

        Ok(stream)
    }

    async fn append_to_stream(&self, stream: &mut EventStream, event: Event) -> Result<u64> {
        let mut tx = self.pool.begin().await
            .map_err(|e| AllSourceError::StorageError(format!("Transaction begin failed: {}", e)))?;

        // Get current version with row lock (pessimistic locking for DB)
        let current_version: i64 = sqlx::query_scalar(
            "SELECT current_version FROM event_streams WHERE stream_id = $1 FOR UPDATE"
        )
        .bind(stream.stream_id().as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Stream not found: {}", e)))?;

        // Optimistic locking check (domain-level)
        if let Some(expected) = stream.expected_version() {
            if expected != current_version as u64 {
                return Err(AllSourceError::ConcurrencyError(format!(
                    "Version conflict: expected {}, got {}",
                    expected, current_version
                )));
            }
        }

        // Append event to domain entity (this also validates)
        let new_version = stream.append_event(event.clone())?;

        // Insert event into database
        sqlx::query(
            "INSERT INTO events
             (stream_id, version, tenant_id, event_type, entity_id, payload, metadata, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(stream.stream_id().as_str())
        .bind(new_version as i64)
        .bind(event.tenant_id().as_str())
        .bind(event.event_type().as_str())
        .bind(event.entity_id().as_str())
        .bind(serde_json::to_value(event.payload())?)
        .bind(event.metadata().map(|m| serde_json::to_value(m)).transpose()?)
        .bind(event.timestamp())
        .execute(&mut *tx)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to insert event: {}", e)))?;

        // Update stream metadata
        sqlx::query(
            "UPDATE event_streams
             SET current_version = $1, watermark = $2, updated_at = $3
             WHERE stream_id = $4"
        )
        .bind(stream.current_version() as i64)
        .bind(stream.watermark() as i64)
        .bind(stream.updated_at())
        .bind(stream.stream_id().as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to update stream: {}", e)))?;

        tx.commit().await
            .map_err(|e| AllSourceError::StorageError(format!("Transaction commit failed: {}", e)))?;

        Ok(new_version)
    }

    async fn save_stream(&self, stream: &EventStream) -> Result<()> {
        // For PostgreSQL, we don't need separate save - it's handled in append_to_stream
        // This method is here for compatibility with in-memory implementation
        let mut tx = self.pool.begin().await
            .map_err(|e| AllSourceError::StorageError(format!("Transaction begin failed: {}", e)))?;

        sqlx::query(
            "UPDATE event_streams
             SET current_version = $1, watermark = $2, updated_at = $3
             WHERE stream_id = $4"
        )
        .bind(stream.current_version() as i64)
        .bind(stream.watermark() as i64)
        .bind(stream.updated_at())
        .bind(stream.stream_id().as_str())
        .execute(&mut *tx)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to save stream: {}", e)))?;

        tx.commit().await
            .map_err(|e| AllSourceError::StorageError(format!("Transaction commit failed: {}", e)))?;

        Ok(())
    }

    async fn load_stream(&self, stream_id: &EntityId) -> Result<Option<EventStream>> {
        let mut tx = self.pool.begin().await
            .map_err(|e| AllSourceError::StorageError(format!("Transaction begin failed: {}", e)))?;

        let maybe_row = sqlx::query(
            "SELECT stream_id, partition_id, current_version, watermark,
                    expected_version, created_at, updated_at
             FROM event_streams
             WHERE stream_id = $1"
        )
        .bind(stream_id.as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to query stream: {}", e)))?;

        let stream = if let Some(row) = maybe_row {
            let partition_id: i32 = row.try_get("partition_id")?;
            let current_version: i64 = row.try_get("current_version")?;
            let watermark: i64 = row.try_get("watermark")?;
            let expected_version: Option<i64> = row.try_get("expected_version").ok();
            let created_at: DateTime<Utc> = row.try_get("created_at")?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at")?;

            Some(Self::reconstruct_stream(
                &mut tx,
                stream_id.as_str(),
                partition_id,
                current_version,
                watermark,
                expected_version,
                created_at,
                updated_at,
            ).await?)
        } else {
            None
        };

        tx.commit().await
            .map_err(|e| AllSourceError::StorageError(format!("Transaction commit failed: {}", e)))?;

        Ok(stream)
    }

    async fn get_streams_by_partition(&self, partition_key: &PartitionKey) -> Result<Vec<EventStream>> {
        let partition_id = partition_key.partition_id() as i32;

        let rows = sqlx::query(
            "SELECT stream_id, partition_id, current_version, watermark,
                    expected_version, created_at, updated_at
             FROM event_streams
             WHERE partition_id = $1
             ORDER BY stream_id"
        )
        .bind(partition_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to query partition: {}", e)))?;

        let mut streams = Vec::new();
        for row in rows {
            let stream_id: String = row.try_get("stream_id")?;
            let partition_id: i32 = row.try_get("partition_id")?;
            let current_version: i64 = row.try_get("current_version")?;
            let watermark: i64 = row.try_get("watermark")?;
            let expected_version: Option<i64> = row.try_get("expected_version").ok();
            let created_at: DateTime<Utc> = row.try_get("created_at")?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at")?;

            let mut tx = self.pool.begin().await?;
            let stream = Self::reconstruct_stream(
                &mut tx,
                &stream_id,
                partition_id,
                current_version,
                watermark,
                expected_version,
                created_at,
                updated_at,
            ).await?;
            tx.commit().await?;

            streams.push(stream);
        }

        Ok(streams)
    }

    async fn get_watermark(&self, stream_id: &EntityId) -> Result<u64> {
        let watermark: i64 = sqlx::query_scalar(
            "SELECT watermark FROM event_streams WHERE stream_id = $1"
        )
        .bind(stream_id.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AllSourceError::EntityNotFound(format!("Stream not found: {}", e)))?;

        Ok(watermark as u64)
    }

    async fn verify_gapless(&self, stream_id: &EntityId) -> Result<bool> {
        let is_gapless: bool = sqlx::query_scalar(
            "SELECT verify_stream_gapless($1)"
        )
        .bind(stream_id.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Gapless check failed: {}", e)))?;

        Ok(is_gapless)
    }

    async fn count_streams(&self) -> Result<usize> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM event_streams")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AllSourceError::StorageError(format!("Count failed: {}", e)))?;

        Ok(count as usize)
    }

    async fn partition_stats(&self) -> Result<Vec<(u32, usize)>> {
        let rows = sqlx::query_as::<_, (i32, i64)>(
            "SELECT partition_id, COUNT(*) FROM event_streams GROUP BY partition_id ORDER BY partition_id"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Stats query failed: {}", e)))?;

        Ok(rows.into_iter().map(|(p, c)| (p as u32, c as usize)).collect())
    }
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/allsource_test".to_string());

        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL running
    async fn test_create_repository() {
        let pool = setup_test_db().await;
        let repo = PostgresEventStreamRepository::new(pool);

        // Run migrations
        repo.migrate().await.expect("Migrations should succeed");
    }

    #[tokio::test]
    #[ignore] // Requires PostgreSQL running
    async fn test_get_or_create_stream() {
        let pool = setup_test_db().await;
        let repo = PostgresEventStreamRepository::new(pool);
        repo.migrate().await.unwrap();

        let stream_id = EntityId::new("test-stream-1".to_string()).unwrap();
        let stream = repo.get_or_create_stream(&stream_id).await.unwrap();

        assert_eq!(stream.current_version(), 0);
        assert_eq!(stream.watermark(), 0);
        assert_eq!(stream.stream_id(), &stream_id);
    }

    // More tests would follow...
}
