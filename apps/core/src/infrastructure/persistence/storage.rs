use crate::{
    domain::entities::Event,
    error::{AllSourceError, Result},
};
use arrow::{
    array::{
        Array, ArrayRef, StringBuilder, TimestampMicrosecondArray, TimestampMicrosecondBuilder,
        UInt64Builder,
    },
    datatypes::{DataType, Field, Schema, TimeUnit},
    record_batch::RecordBatch,
};
use parquet::{arrow::ArrowWriter, file::properties::WriterProperties};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// Default batch size for Parquet writes (10,000 events as per US-023)
pub const DEFAULT_BATCH_SIZE: usize = 10_000;

/// Default flush timeout in milliseconds
pub const DEFAULT_FLUSH_TIMEOUT_MS: u64 = 5_000;

/// Configuration for ParquetStorage batch processing
#[derive(Debug, Clone)]
pub struct ParquetStorageConfig {
    /// Batch size before automatic flush (default: 10,000)
    pub batch_size: usize,
    /// Timeout before flushing partial batch (default: 5 seconds)
    pub flush_timeout: Duration,
    /// Compression codec for Parquet files
    pub compression: parquet::basic::Compression,
}

impl Default for ParquetStorageConfig {
    fn default() -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
            flush_timeout: Duration::from_millis(DEFAULT_FLUSH_TIMEOUT_MS),
            compression: parquet::basic::Compression::SNAPPY,
        }
    }
}

impl ParquetStorageConfig {
    /// High-throughput configuration optimized for large batch writes
    pub fn high_throughput() -> Self {
        Self {
            batch_size: 50_000,
            flush_timeout: Duration::from_secs(10),
            compression: parquet::basic::Compression::SNAPPY,
        }
    }

    /// Low-latency configuration for smaller, more frequent writes
    pub fn low_latency() -> Self {
        Self {
            batch_size: 1_000,
            flush_timeout: Duration::from_secs(1),
            compression: parquet::basic::Compression::SNAPPY,
        }
    }
}

/// Statistics for batch write operations
#[derive(Debug, Clone, Default)]
pub struct BatchWriteStats {
    /// Total batches written
    pub batches_written: u64,
    /// Total events written
    pub events_written: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Average batch size
    pub avg_batch_size: f64,
    /// Events per second (throughput)
    pub events_per_sec: f64,
    /// Total write time in nanoseconds
    pub total_write_time_ns: u64,
    /// Number of timeout-triggered flushes
    pub timeout_flushes: u64,
    /// Number of size-triggered flushes
    pub size_flushes: u64,
}

/// Result of a batch write operation
#[derive(Debug, Clone)]
pub struct BatchWriteResult {
    /// Number of events written
    pub events_written: usize,
    /// Number of batches flushed to disk
    pub batches_flushed: usize,
    /// Total duration of the write operation
    pub duration: Duration,
    /// Write throughput in events per second
    pub events_per_sec: f64,
}

/// Parquet-based persistent storage for events with batch processing
///
/// Features:
/// - Configurable batch size (default: 10,000 events per US-023)
/// - Timeout-based flushing for partial batches
/// - Thread-safe batch accumulation
/// - SNAPPY compression for efficient storage
/// - Automatic flush on shutdown via Drop
pub struct ParquetStorage {
    /// Base directory for storing parquet files
    storage_dir: PathBuf,

    /// Current batch being accumulated (protected by mutex for thread safety)
    current_batch: Mutex<Vec<Event>>,

    /// Configuration
    config: ParquetStorageConfig,

    /// Schema for Arrow/Parquet
    schema: Arc<Schema>,

    /// Last flush timestamp for timeout tracking
    last_flush_time: Mutex<Instant>,

    /// Statistics tracking
    batches_written: AtomicU64,
    events_written: AtomicU64,
    bytes_written: AtomicU64,
    total_write_time_ns: AtomicU64,
    timeout_flushes: AtomicU64,
    size_flushes: AtomicU64,
}

impl ParquetStorage {
    /// Create a new ParquetStorage with default configuration (10,000 event batches)
    pub fn new(storage_dir: impl AsRef<Path>) -> Result<Self> {
        Self::with_config(storage_dir, ParquetStorageConfig::default())
    }

    /// Create a new ParquetStorage with custom configuration
    pub fn with_config(
        storage_dir: impl AsRef<Path>,
        config: ParquetStorageConfig,
    ) -> Result<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // Create storage directory if it doesn't exist
        fs::create_dir_all(&storage_dir).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to create storage directory: {e}"))
        })?;

        // Define Arrow schema for events
        let schema = Arc::new(Schema::new(vec![
            Field::new("event_id", DataType::Utf8, false),
            Field::new("event_type", DataType::Utf8, false),
            Field::new("entity_id", DataType::Utf8, false),
            Field::new("payload", DataType::Utf8, false),
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Microsecond, None),
                false,
            ),
            Field::new("metadata", DataType::Utf8, true),
            Field::new("version", DataType::UInt64, false),
        ]));

        Ok(Self {
            storage_dir,
            current_batch: Mutex::new(Vec::with_capacity(config.batch_size)),
            config,
            schema,
            last_flush_time: Mutex::new(Instant::now()),
            batches_written: AtomicU64::new(0),
            events_written: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
            total_write_time_ns: AtomicU64::new(0),
            timeout_flushes: AtomicU64::new(0),
            size_flushes: AtomicU64::new(0),
        })
    }

    /// Create storage with legacy batch size (1000) for backward compatibility
    #[deprecated(note = "Use new() or with_config() instead - default batch size is now 10,000")]
    pub fn with_legacy_batch_size(storage_dir: impl AsRef<Path>) -> Result<Self> {
        Self::with_config(
            storage_dir,
            ParquetStorageConfig {
                batch_size: 1000,
                ..Default::default()
            },
        )
    }

    /// Add an event to the current batch
    ///
    /// Events are buffered until either:
    /// - Batch size reaches configured limit (default: 10,000)
    /// - Flush timeout is exceeded
    /// - Manual flush() is called
    pub fn append_event(&self, event: Event) -> Result<()> {
        let should_flush = {
            let mut batch = self.current_batch.lock().unwrap();
            batch.push(event);
            batch.len() >= self.config.batch_size
        };

        if should_flush {
            self.size_flushes.fetch_add(1, Ordering::Relaxed);
            self.flush()?;
        }

        Ok(())
    }

    /// Add multiple events to the batch (optimized batch insertion)
    ///
    /// This is the preferred method for high-throughput ingestion.
    /// Events are added atomically and flushed if batch size is reached.
    pub fn batch_write(&self, events: Vec<Event>) -> Result<BatchWriteResult> {
        let start = Instant::now();
        let event_count = events.len();
        let mut batches_flushed = 0;

        // Add events in chunks to avoid holding lock too long
        let mut remaining_events = events.into_iter().peekable();

        while remaining_events.peek().is_some() {
            let should_flush = {
                let mut batch = self.current_batch.lock().unwrap();
                let available_space = self.config.batch_size.saturating_sub(batch.len());

                if available_space == 0 {
                    true
                } else {
                    // Take up to available_space events
                    let to_add: Vec<Event> =
                        remaining_events.by_ref().take(available_space).collect();
                    batch.extend(to_add);
                    batch.len() >= self.config.batch_size
                }
            };

            if should_flush {
                self.size_flushes.fetch_add(1, Ordering::Relaxed);
                self.flush()?;
                batches_flushed += 1;
            }
        }

        let duration = start.elapsed();

        Ok(BatchWriteResult {
            events_written: event_count,
            batches_flushed,
            duration,
            events_per_sec: event_count as f64 / duration.as_secs_f64(),
        })
    }

    /// Check if a timeout-based flush is needed and perform it
    ///
    /// Call this periodically (e.g., from a background task) to ensure
    /// partial batches are flushed within the configured timeout.
    pub fn check_timeout_flush(&self) -> Result<bool> {
        let should_flush = {
            let last_flush = self.last_flush_time.lock().unwrap();
            let batch = self.current_batch.lock().unwrap();
            !batch.is_empty() && last_flush.elapsed() >= self.config.flush_timeout
        };

        if should_flush {
            self.timeout_flushes.fetch_add(1, Ordering::Relaxed);
            self.flush()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Flush current batch to a Parquet file
    ///
    /// Thread-safe: Can be called from multiple threads.
    pub fn flush(&self) -> Result<()> {
        let events_to_write = {
            let mut batch = self.current_batch.lock().unwrap();
            if batch.is_empty() {
                return Ok(());
            }
            std::mem::take(&mut *batch)
        };

        let batch_count = events_to_write.len();
        tracing::info!("Flushing {} events to Parquet storage", batch_count);

        let start = Instant::now();

        // Create record batch from events
        let record_batch = self.events_to_record_batch(&events_to_write)?;

        // Generate filename with timestamp and unique suffix for concurrent writes
        let filename = format!(
            "events-{}-{}.parquet",
            chrono::Utc::now().format("%Y%m%d-%H%M%S%3f"),
            uuid::Uuid::new_v4().as_simple()
        );
        let file_path = self.storage_dir.join(&filename);

        // Write to Parquet file
        let file = File::create(&file_path).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to create parquet file: {e}"))
        })?;

        let props = WriterProperties::builder()
            .set_compression(self.config.compression)
            .build();

        let mut writer = ArrowWriter::try_new(file, self.schema.clone(), Some(props))?;

        writer.write(&record_batch)?;
        let file_metadata = writer.close()?;

        let duration = start.elapsed();

        // Update statistics
        self.batches_written.fetch_add(1, Ordering::Relaxed);
        self.events_written
            .fetch_add(batch_count as u64, Ordering::Relaxed);
        if let Some(size) = file_metadata
            .row_groups
            .first()
            .map(|rg| rg.total_byte_size)
        {
            self.bytes_written.fetch_add(size as u64, Ordering::Relaxed);
        }
        self.total_write_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);

        // Update last flush time
        {
            let mut last_flush = self.last_flush_time.lock().unwrap();
            *last_flush = Instant::now();
        }

        tracing::info!(
            "Successfully wrote {} events to {} in {:?}",
            batch_count,
            file_path.display(),
            duration
        );

        Ok(())
    }

    /// Force flush any remaining events (for shutdown handling)
    ///
    /// This ensures partial batches are persisted on graceful shutdown.
    pub fn flush_on_shutdown(&self) -> Result<usize> {
        let batch_size = {
            let batch = self.current_batch.lock().unwrap();
            batch.len()
        };

        if batch_size > 0 {
            tracing::info!("Shutdown: flushing partial batch of {} events", batch_size);
            self.flush()?;
        }

        Ok(batch_size)
    }

    /// Get batch write statistics
    pub fn batch_stats(&self) -> BatchWriteStats {
        let batches = self.batches_written.load(Ordering::Relaxed);
        let events = self.events_written.load(Ordering::Relaxed);
        let bytes = self.bytes_written.load(Ordering::Relaxed);
        let time_ns = self.total_write_time_ns.load(Ordering::Relaxed);

        let time_secs = time_ns as f64 / 1_000_000_000.0;

        BatchWriteStats {
            batches_written: batches,
            events_written: events,
            bytes_written: bytes,
            avg_batch_size: if batches > 0 {
                events as f64 / batches as f64
            } else {
                0.0
            },
            events_per_sec: if time_secs > 0.0 {
                events as f64 / time_secs
            } else {
                0.0
            },
            total_write_time_ns: time_ns,
            timeout_flushes: self.timeout_flushes.load(Ordering::Relaxed),
            size_flushes: self.size_flushes.load(Ordering::Relaxed),
        }
    }

    /// Get current batch size (pending events)
    pub fn pending_count(&self) -> usize {
        self.current_batch.lock().unwrap().len()
    }

    /// Get configured batch size
    pub fn batch_size(&self) -> usize {
        self.config.batch_size
    }

    /// Get configured flush timeout
    pub fn flush_timeout(&self) -> Duration {
        self.config.flush_timeout
    }

    /// Convert events to Arrow RecordBatch
    fn events_to_record_batch(&self, events: &[Event]) -> Result<RecordBatch> {
        let mut event_id_builder = StringBuilder::new();
        let mut event_type_builder = StringBuilder::new();
        let mut entity_id_builder = StringBuilder::new();
        let mut payload_builder = StringBuilder::new();
        let mut timestamp_builder = TimestampMicrosecondBuilder::new();
        let mut metadata_builder = StringBuilder::new();
        let mut version_builder = UInt64Builder::new();

        for event in events {
            event_id_builder.append_value(event.id.to_string());
            event_type_builder.append_value(event.event_type_str());
            entity_id_builder.append_value(event.entity_id_str());
            payload_builder.append_value(serde_json::to_string(&event.payload)?);

            // Convert timestamp to microseconds
            let timestamp_micros = event.timestamp.timestamp_micros();
            timestamp_builder.append_value(timestamp_micros);

            if let Some(ref metadata) = event.metadata {
                metadata_builder.append_value(serde_json::to_string(metadata)?);
            } else {
                metadata_builder.append_null();
            }

            version_builder.append_value(event.version as u64);
        }

        let arrays: Vec<ArrayRef> = vec![
            Arc::new(event_id_builder.finish()),
            Arc::new(event_type_builder.finish()),
            Arc::new(entity_id_builder.finish()),
            Arc::new(payload_builder.finish()),
            Arc::new(timestamp_builder.finish()),
            Arc::new(metadata_builder.finish()),
            Arc::new(version_builder.finish()),
        ];

        let record_batch = RecordBatch::try_new(self.schema.clone(), arrays)?;

        Ok(record_batch)
    }

    /// Load events from all Parquet files
    pub fn load_all_events(&self) -> Result<Vec<Event>> {
        let mut all_events = Vec::new();

        // Read all parquet files in storage directory
        let entries = fs::read_dir(&self.storage_dir).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to read storage directory: {e}"))
        })?;

        let mut parquet_files: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "parquet")
                    .unwrap_or(false)
            })
            .collect();

        // Sort files by name (which includes timestamp)
        parquet_files.sort();

        for file_path in parquet_files {
            tracing::info!("Loading events from {}", file_path.display());
            let file_events = self.load_events_from_file(&file_path)?;
            all_events.extend(file_events);
        }

        tracing::info!("Loaded {} total events from storage", all_events.len());

        Ok(all_events)
    }

    /// Load events from a single Parquet file
    fn load_events_from_file(&self, file_path: &Path) -> Result<Vec<Event>> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

        let file = File::open(file_path).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to open parquet file: {e}"))
        })?;

        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let mut reader = builder.build()?;

        let mut events = Vec::new();

        while let Some(Ok(batch)) = reader.next() {
            let batch_events = self.record_batch_to_events(&batch)?;
            events.extend(batch_events);
        }

        Ok(events)
    }

    /// Convert Arrow RecordBatch back to events
    fn record_batch_to_events(&self, batch: &RecordBatch) -> Result<Vec<Event>> {
        let event_ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .ok_or_else(|| AllSourceError::StorageError("Invalid event_id column".to_string()))?;

        let event_types = batch
            .column(1)
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .ok_or_else(|| AllSourceError::StorageError("Invalid event_type column".to_string()))?;

        let entity_ids = batch
            .column(2)
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .ok_or_else(|| AllSourceError::StorageError("Invalid entity_id column".to_string()))?;

        let payloads = batch
            .column(3)
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .ok_or_else(|| AllSourceError::StorageError("Invalid payload column".to_string()))?;

        let timestamps = batch
            .column(4)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .ok_or_else(|| AllSourceError::StorageError("Invalid timestamp column".to_string()))?;

        let metadatas = batch
            .column(5)
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .ok_or_else(|| AllSourceError::StorageError("Invalid metadata column".to_string()))?;

        let versions = batch
            .column(6)
            .as_any()
            .downcast_ref::<arrow::array::UInt64Array>()
            .ok_or_else(|| AllSourceError::StorageError("Invalid version column".to_string()))?;

        let mut events = Vec::new();

        for i in 0..batch.num_rows() {
            let id = uuid::Uuid::parse_str(event_ids.value(i))
                .map_err(|e| AllSourceError::StorageError(format!("Invalid UUID: {e}")))?;

            let timestamp = chrono::DateTime::from_timestamp_micros(timestamps.value(i))
                .ok_or_else(|| AllSourceError::StorageError("Invalid timestamp".to_string()))?;

            let metadata = if metadatas.is_null(i) {
                None
            } else {
                Some(serde_json::from_str(metadatas.value(i))?)
            };

            let event = Event::reconstruct_from_strings(
                id,
                event_types.value(i).to_string(),
                entity_ids.value(i).to_string(),
                "default".to_string(), // Default tenant for backward compatibility
                serde_json::from_str(payloads.value(i))?,
                timestamp,
                metadata,
                versions.value(i) as i64,
            );

            events.push(event);
        }

        Ok(events)
    }

    /// List all Parquet file paths in the storage directory, sorted by name.
    ///
    /// Used by the replication catch-up protocol to stream snapshot files
    /// to followers that are too far behind for WAL-only catch-up.
    pub fn list_parquet_files(&self) -> Result<Vec<PathBuf>> {
        let entries = fs::read_dir(&self.storage_dir).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to read storage directory: {e}"))
        })?;

        let mut parquet_files: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "parquet")
                    .unwrap_or(false)
            })
            .collect();

        parquet_files.sort();
        Ok(parquet_files)
    }

    /// Get the storage directory path.
    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    /// Get storage statistics
    pub fn stats(&self) -> Result<StorageStats> {
        let entries = fs::read_dir(&self.storage_dir).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to read storage directory: {e}"))
        })?;

        let mut total_files = 0;
        let mut total_size_bytes = 0u64;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                total_files += 1;
                if let Ok(metadata) = entry.metadata() {
                    total_size_bytes += metadata.len();
                }
            }
        }

        let current_batch_size = self.current_batch.lock().unwrap().len();

        Ok(StorageStats {
            total_files,
            total_size_bytes,
            storage_dir: self.storage_dir.clone(),
            current_batch_size,
        })
    }
}

impl Drop for ParquetStorage {
    fn drop(&mut self) {
        // Ensure any remaining events are flushed on shutdown
        if let Err(e) = self.flush_on_shutdown() {
            tracing::error!("Failed to flush events on drop: {}", e);
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct StorageStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub storage_dir: PathBuf,
    pub current_batch_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_event(entity_id: &str) -> Event {
        Event::reconstruct_from_strings(
            uuid::Uuid::new_v4(),
            "test.event".to_string(),
            entity_id.to_string(),
            "default".to_string(),
            json!({
                "test": "data",
                "value": 42
            }),
            chrono::Utc::now(),
            None,
            1,
        )
    }

    #[test]
    fn test_parquet_storage_write_read() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        // Add events
        for i in 0..10 {
            let event = create_test_event(&format!("entity-{}", i));
            storage.append_event(event).unwrap();
        }

        // Flush to disk
        storage.flush().unwrap();

        // Load back
        let loaded_events = storage.load_all_events().unwrap();
        assert_eq!(loaded_events.len(), 10);
    }

    #[test]
    fn test_storage_stats() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        // Add and flush events
        for i in 0..5 {
            storage
                .append_event(create_test_event(&format!("entity-{}", i)))
                .unwrap();
        }
        storage.flush().unwrap();

        let stats = storage.stats().unwrap();
        assert_eq!(stats.total_files, 1);
        assert!(stats.total_size_bytes > 0);
    }

    #[test]
    fn test_default_batch_size() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        // Default batch size should be 10,000 as per US-023
        assert_eq!(storage.batch_size(), DEFAULT_BATCH_SIZE);
        assert_eq!(storage.batch_size(), 10_000);
    }

    #[test]
    fn test_custom_config() {
        let temp_dir = TempDir::new().unwrap();
        let config = ParquetStorageConfig {
            batch_size: 5_000,
            flush_timeout: Duration::from_secs(2),
            compression: parquet::basic::Compression::SNAPPY,
        };
        let storage = ParquetStorage::with_config(temp_dir.path(), config).unwrap();

        assert_eq!(storage.batch_size(), 5_000);
        assert_eq!(storage.flush_timeout(), Duration::from_secs(2));
    }

    #[test]
    fn test_batch_write() {
        let temp_dir = TempDir::new().unwrap();
        let config = ParquetStorageConfig {
            batch_size: 100, // Small batch for testing
            ..Default::default()
        };
        let storage = ParquetStorage::with_config(temp_dir.path(), config).unwrap();

        // Create 250 events (should trigger 2 flushes with 50 remaining)
        let events: Vec<Event> = (0..250)
            .map(|i| create_test_event(&format!("entity-{}", i)))
            .collect();

        let result = storage.batch_write(events).unwrap();
        assert_eq!(result.events_written, 250);
        assert_eq!(result.batches_flushed, 2);

        // 50 events should be pending
        assert_eq!(storage.pending_count(), 50);

        // Flush remaining
        storage.flush().unwrap();

        // Load all events back
        let loaded = storage.load_all_events().unwrap();
        assert_eq!(loaded.len(), 250);
    }

    #[test]
    fn test_auto_flush_on_batch_size() {
        let temp_dir = TempDir::new().unwrap();
        let config = ParquetStorageConfig {
            batch_size: 10, // Very small for testing
            ..Default::default()
        };
        let storage = ParquetStorage::with_config(temp_dir.path(), config).unwrap();

        // Add 15 events - should auto-flush at 10
        for i in 0..15 {
            storage
                .append_event(create_test_event(&format!("entity-{}", i)))
                .unwrap();
        }

        // Should have 5 pending, 10 written
        assert_eq!(storage.pending_count(), 5);

        let stats = storage.batch_stats();
        assert_eq!(stats.events_written, 10);
        assert_eq!(stats.batches_written, 1);
        assert_eq!(stats.size_flushes, 1);
    }

    #[test]
    fn test_flush_on_shutdown() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        // Add some events without reaching batch size
        for i in 0..5 {
            storage
                .append_event(create_test_event(&format!("entity-{}", i)))
                .unwrap();
        }

        assert_eq!(storage.pending_count(), 5);

        // Manually trigger shutdown flush
        let flushed = storage.flush_on_shutdown().unwrap();
        assert_eq!(flushed, 5);
        assert_eq!(storage.pending_count(), 0);

        // Verify events are persisted
        let loaded = storage.load_all_events().unwrap();
        assert_eq!(loaded.len(), 5);
    }

    #[test]
    fn test_thread_safe_writes() {
        let temp_dir = TempDir::new().unwrap();
        let config = ParquetStorageConfig {
            batch_size: 100,
            ..Default::default()
        };
        let storage = Arc::new(ParquetStorage::with_config(temp_dir.path(), config).unwrap());

        let events_per_thread = 50;
        let thread_count = 4;

        std::thread::scope(|s| {
            for t in 0..thread_count {
                let storage_ref = Arc::clone(&storage);
                s.spawn(move || {
                    for i in 0..events_per_thread {
                        let event = create_test_event(&format!("thread-{}-entity-{}", t, i));
                        storage_ref.append_event(event).unwrap();
                    }
                });
            }
        });

        // Flush remaining
        storage.flush().unwrap();

        // All events should be written
        let loaded = storage.load_all_events().unwrap();
        assert_eq!(loaded.len(), events_per_thread * thread_count);
    }

    #[test]
    fn test_batch_stats() {
        let temp_dir = TempDir::new().unwrap();
        let config = ParquetStorageConfig {
            batch_size: 50,
            ..Default::default()
        };
        let storage = ParquetStorage::with_config(temp_dir.path(), config).unwrap();

        // Write 100 events (2 batches)
        let events: Vec<Event> = (0..100)
            .map(|i| create_test_event(&format!("entity-{}", i)))
            .collect();

        storage.batch_write(events).unwrap();

        let stats = storage.batch_stats();
        assert_eq!(stats.batches_written, 2);
        assert_eq!(stats.events_written, 100);
        assert!(stats.avg_batch_size > 0.0);
        assert!(stats.events_per_sec > 0.0);
        assert_eq!(stats.size_flushes, 2);
    }

    #[test]
    fn test_config_presets() {
        let high_throughput = ParquetStorageConfig::high_throughput();
        assert_eq!(high_throughput.batch_size, 50_000);
        assert_eq!(high_throughput.flush_timeout, Duration::from_secs(10));

        let low_latency = ParquetStorageConfig::low_latency();
        assert_eq!(low_latency.batch_size, 1_000);
        assert_eq!(low_latency.flush_timeout, Duration::from_secs(1));

        let default = ParquetStorageConfig::default();
        assert_eq!(default.batch_size, DEFAULT_BATCH_SIZE);
        assert_eq!(default.batch_size, 10_000);
    }

    /// Benchmark: Compare single-event writes vs batch writes
    /// Run with: cargo test --release -- --ignored test_batch_write_throughput
    #[test]
    #[ignore]
    fn test_batch_write_throughput() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        let event_count = 50_000;

        // Benchmark batch write
        let events: Vec<Event> = (0..event_count)
            .map(|i| create_test_event(&format!("entity-{}", i)))
            .collect();

        let start = std::time::Instant::now();
        let result = storage.batch_write(events).unwrap();
        storage.flush().unwrap(); // Flush any remaining
        let batch_duration = start.elapsed();

        let batch_stats = storage.batch_stats();

        println!("\n=== Parquet Batch Write Performance (BATCH_SIZE=10,000) ===");
        println!("Events: {}", event_count);
        println!("Duration: {:?}", batch_duration);
        println!("Events/sec: {:.0}", result.events_per_sec);
        println!("Batches written: {}", batch_stats.batches_written);
        println!("Avg batch size: {:.0}", batch_stats.avg_batch_size);
        println!("Bytes written: {} KB", batch_stats.bytes_written / 1024);

        // Target: Batch writes should achieve at least 100K events/sec in release mode
        // This represents 40%+ improvement over single-event writes
        assert!(
            result.events_per_sec > 10_000.0,
            "Batch write throughput too low: {:.0} events/sec (expected >10K in debug, >100K in release)",
            result.events_per_sec
        );
    }

    /// Benchmark: Single-event write baseline (for comparison)
    #[test]
    #[ignore]
    fn test_single_event_write_baseline() {
        let temp_dir = TempDir::new().unwrap();
        let config = ParquetStorageConfig {
            batch_size: 1, // Force flush after each event
            ..Default::default()
        };
        let storage = ParquetStorage::with_config(temp_dir.path(), config).unwrap();

        let event_count = 1_000; // Fewer events since this is slow

        let start = std::time::Instant::now();
        for i in 0..event_count {
            let event = create_test_event(&format!("entity-{}", i));
            storage.append_event(event).unwrap();
        }
        let duration = start.elapsed();

        let events_per_sec = event_count as f64 / duration.as_secs_f64();

        println!("\n=== Single-Event Write Baseline ===");
        println!("Events: {}", event_count);
        println!("Duration: {:?}", duration);
        println!("Events/sec: {:.0}", events_per_sec);

        // This should be significantly slower than batch writes
        // Used as a baseline to demonstrate 40%+ improvement
    }
}
