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
    collections::HashMap,
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

    /// Buffered events keyed by tenant_id. Each tenant accumulates its own
    /// batch and flushes independently into its partition under
    /// `storage_dir/<tenant_id>/<yyyy-mm>/`. Single outer mutex protects the
    /// whole map: lookup is O(1), tenant cardinality is low (single digits
    /// today; bounded by Step 3's cache budget later), so contention is
    /// fine. We keep the mutex held only for the push, not for disk I/O —
    /// flush takes ownership of a tenant's batch via remove() and writes
    /// after the lock is released.
    current_batches: Mutex<HashMap<String, Vec<Event>>>,

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
            current_batches: Mutex::new(HashMap::new()),
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
    /// Events are routed to a per-tenant batch keyed by `event.tenant_id_str()`.
    /// A tenant's batch is buffered until any of:
    /// - That tenant's batch hits the configured `batch_size` (default 10,000)
    ///   — flushes only that tenant, not the whole world
    /// - The flush timeout elapses — flushes every tenant with pending data
    /// - `flush()` is called explicitly
    /// - The process shuts down
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
    pub fn append_event(&self, event: Event) -> Result<()> {
        let tenant = event.tenant_id_str().to_string();
        let should_flush_tenant = {
            let mut batches = self.current_batches.lock().unwrap();
            let entry = batches.entry(tenant.clone()).or_insert_with(Vec::new);
            entry.push(event);
            entry.len() >= self.config.batch_size
        };

        if should_flush_tenant {
            self.size_flushes.fetch_add(1, Ordering::Relaxed);
            self.flush_tenant(&tenant)?;
        }

        Ok(())
    }

    /// Add multiple events to the batch (optimized batch insertion)
    ///
    /// Preferred entry point for high-throughput ingestion. Events are
    /// grouped by tenant under a single mutex acquisition and any tenant
    /// that crosses `batch_size` is flushed on the spot.
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
    pub fn batch_write(&self, events: Vec<Event>) -> Result<BatchWriteResult> {
        let start = Instant::now();
        let event_count = events.len();

        // Pre-group by tenant to keep the lock window short — one acquire,
        // one extend per tenant, decide which tenants are over threshold.
        let mut grouped: HashMap<String, Vec<Event>> = HashMap::new();
        for event in events {
            grouped
                .entry(event.tenant_id_str().to_string())
                .or_default()
                .push(event);
        }

        let mut tenants_to_flush: Vec<String> = Vec::new();
        {
            let mut batches = self.current_batches.lock().unwrap();
            for (tenant, mut new_events) in grouped {
                let entry = batches.entry(tenant.clone()).or_insert_with(Vec::new);
                entry.append(&mut new_events);
                if entry.len() >= self.config.batch_size {
                    tenants_to_flush.push(tenant);
                }
            }
        }

        let mut batches_flushed = 0;
        for tenant in tenants_to_flush {
            self.size_flushes.fetch_add(1, Ordering::Relaxed);
            self.flush_tenant(&tenant)?;
            batches_flushed += 1;
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
    /// partial batches are flushed within the configured timeout. When
    /// triggered, every tenant with pending events flushes — the timer is
    /// global, not per-tenant, so a slow-trickle tenant doesn't get
    /// stranded waiting for its own batch to fill.
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
    pub fn check_timeout_flush(&self) -> Result<bool> {
        let should_flush = {
            let last_flush = self.last_flush_time.lock().unwrap();
            let batches = self.current_batches.lock().unwrap();
            let any_pending = batches.values().any(|v| !v.is_empty());
            any_pending && last_flush.elapsed() >= self.config.flush_timeout
        };

        if should_flush {
            self.timeout_flushes.fetch_add(1, Ordering::Relaxed);
            self.flush()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Flush every tenant's pending batch to its partition.
    ///
    /// Thread-safe: callable from any thread. A snapshot of which tenants
    /// have pending data is taken under a short lock; each tenant is then
    /// flushed individually with its own lock cycle, so disk I/O for one
    /// tenant doesn't block writes against another.
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
    pub fn flush(&self) -> Result<()> {
        let tenants: Vec<String> = {
            let batches = self.current_batches.lock().unwrap();
            batches
                .iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, _)| k.clone())
                .collect()
        };
        if tenants.is_empty() {
            return Ok(());
        }
        for tenant in tenants {
            self.flush_tenant(&tenant)?;
        }
        Ok(())
    }

    /// Flush a single tenant's pending events into its partition file.
    ///
    /// File path: `storage_dir/<sanitized_tenant_id>/<yyyy-mm>/events-<ts>-<uuid>.parquet`.
    /// `<yyyy-mm>` is taken from the wall-clock at flush time (matching the
    /// pre-tenant filename's timestamp semantics) rather than from
    /// individual event timestamps — keeps each flush to a single output
    /// file even when buffered events span months.
    fn flush_tenant(&self, tenant_id: &str) -> Result<()> {
        let events_to_write = {
            let mut batches = self.current_batches.lock().unwrap();
            match batches.get_mut(tenant_id) {
                Some(v) if !v.is_empty() => std::mem::take(v),
                _ => return Ok(()),
            }
        };

        let batch_count = events_to_write.len();
        let start = Instant::now();

        let record_batch = self.events_to_record_batch(&events_to_write)?;

        let now = chrono::Utc::now();
        let partition_dir = partition_path_for_tenant(&self.storage_dir, tenant_id, now)?;
        fs::create_dir_all(&partition_dir).map_err(|e| {
            AllSourceError::StorageError(format!(
                "Failed to create tenant partition {}: {e}",
                partition_dir.display()
            ))
        })?;
        let filename = format!(
            "events-{}-{}.parquet",
            now.format("%Y%m%d-%H%M%S%3f"),
            uuid::Uuid::new_v4().as_simple()
        );
        let file_path = partition_dir.join(&filename);

        tracing::info!(
            "Flushing {} events for tenant={} to {}",
            batch_count,
            tenant_id,
            file_path.display()
        );

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

        self.batches_written.fetch_add(1, Ordering::Relaxed);
        self.events_written
            .fetch_add(batch_count as u64, Ordering::Relaxed);
        if let Some(size) = file_metadata
            .row_groups()
            .first()
            .map(parquet::file::metadata::RowGroupMetaData::total_byte_size)
        {
            self.bytes_written.fetch_add(size as u64, Ordering::Relaxed);
        }
        self.total_write_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);

        {
            let mut last_flush = self.last_flush_time.lock().unwrap();
            *last_flush = Instant::now();
        }

        tracing::info!(
            "Wrote {} events for tenant={} to {} in {:?}",
            batch_count,
            tenant_id,
            file_path.display(),
            duration
        );

        Ok(())
    }

    /// Force flush any remaining events (for shutdown handling).
    ///
    /// Sums pending counts across every tenant's batch so the caller can
    /// log "we flushed N events on shutdown" without caring about
    /// per-tenant breakdown.
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
    pub fn flush_on_shutdown(&self) -> Result<usize> {
        let total_pending: usize = {
            let batches = self.current_batches.lock().unwrap();
            batches.values().map(Vec::len).sum()
        };

        if total_pending > 0 {
            tracing::info!(
                "Shutdown: flushing {} pending events across all tenants",
                total_pending
            );
            self.flush()?;
        }

        Ok(total_pending)
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

    /// Total pending events across all tenant batches.
    pub fn pending_count(&self) -> usize {
        self.current_batches
            .lock()
            .unwrap()
            .values()
            .map(Vec::len)
            .sum()
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
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
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

    /// Load events from all Parquet files under the storage directory.
    ///
    /// Walks the tree recursively so both layouts work: legacy flat
    /// (`storage_dir/events-*.parquet`) and the tenant-partitioned tree
    /// introduced by the data-strategy work (`storage_dir/<tenant>/<yyyy-mm>/
    /// events-*.parquet`). The two coexist on disk during migration.
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
    pub fn load_all_events(&self) -> Result<Vec<Event>> {
        let parquet_files = find_parquet_files_recursive(&self.storage_dir)?;

        let mut all_events = Vec::with_capacity(parquet_files.len() * self.config.batch_size);
        for file_path in parquet_files {
            tracing::info!("Loading events from {}", file_path.display());
            let tenant_id = tenant_id_from_path(&self.storage_dir, &file_path);
            let file_events = self.load_events_from_file(&file_path, &tenant_id)?;
            all_events.extend(file_events);
        }

        tracing::info!("Loaded {} total events from storage", all_events.len());

        Ok(all_events)
    }

    /// Load events from a single Parquet file. `tenant_id` is the value to
    /// stamp onto each loaded event — derived from the file's location in
    /// the tree by `load_all_events`. The Parquet schema doesn't include
    /// tenant_id today (path is the source of truth), so this is how
    /// per-tenant identity survives the round trip.
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
    fn load_events_from_file(&self, file_path: &Path, tenant_id: &str) -> Result<Vec<Event>> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

        let file = File::open(file_path).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to open parquet file: {e}"))
        })?;

        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let mut reader = builder.build()?;

        let mut events = Vec::new();

        while let Some(Ok(batch)) = reader.next() {
            let batch_events = self.record_batch_to_events(&batch, tenant_id)?;
            events.extend(batch_events);
        }

        Ok(events)
    }

    /// Convert Arrow RecordBatch back to events. `tenant_id` is stamped onto
    /// each reconstructed event — the schema doesn't carry it today, so the
    /// caller passes the value derived from the file path.
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
    fn record_batch_to_events(&self, batch: &RecordBatch, tenant_id: &str) -> Result<Vec<Event>> {
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
                tenant_id.to_string(),
                serde_json::from_str(payloads.value(i))?,
                timestamp,
                metadata,
                versions.value(i) as i64,
            );

            events.push(event);
        }

        Ok(events)
    }

    /// List all Parquet file paths under the storage directory, sorted by
    /// the relative path so files in the same partition stay grouped.
    ///
    /// Used by the replication catch-up protocol to stream snapshot files
    /// to followers that are too far behind for WAL-only catch-up.
    pub fn list_parquet_files(&self) -> Result<Vec<PathBuf>> {
        find_parquet_files_recursive(&self.storage_dir)
    }

    /// List Parquet files belonging to a single tenant — i.e. only files
    /// under `<storage_dir>/<tenant>/...`. Legacy flat-layout files at the
    /// root are intentionally excluded; the migration tool moves them under
    /// `default/` so once it has run a `tenant=default` query sees them.
    ///
    /// Returns an empty vec when the tenant subtree doesn't exist (no data
    /// for that tenant yet). Returns an error only if `tenant_id` fails the
    /// path-safety whitelist.
    ///
    /// This is the building block for tenant-scoped reads: the caller knows
    /// which files might contain the tenant's data without opening any of
    /// the others.
    pub fn list_parquet_files_for_tenant(&self, tenant_id: &str) -> Result<Vec<PathBuf>> {
        let safe = sanitize_tenant_id_for_path(tenant_id)?;
        let tenant_root = self.storage_dir.join(safe);
        if !tenant_root.is_dir() {
            return Ok(Vec::new());
        }
        find_parquet_files_recursive(&tenant_root)
    }

    /// Load only the events belonging to `tenant_id`, walking just that
    /// tenant's subtree on disk. The full-storage loader
    /// (`load_all_events`) opens every Parquet file regardless of tenant;
    /// this one only opens files under `<storage_dir>/<tenant>/`.
    ///
    /// Returns an empty vec when the tenant has no on-disk data. Returns
    /// an error if the tenant_id fails the path-safety whitelist or any
    /// individual file fails to load.
    ///
    /// This is the read-side complement to per-tenant flushing. It's the
    /// foundation Step 2 (lazy per-tenant load on demand) needs: a way to
    /// hydrate one tenant without paying the cost of loading every other
    /// tenant's data into memory.
    ///
    /// Tenant identity for loaded events comes from the file path, the
    /// same as `load_all_events` — `record_batch_to_events` stamps the
    /// passed `tenant_id` onto every reconstructed event.
    #[cfg_attr(feature = "hotpath", hotpath::measure)]
    pub fn load_events_for_tenant(&self, tenant_id: &str) -> Result<Vec<Event>> {
        let parquet_files = self.list_parquet_files_for_tenant(tenant_id)?;
        tracing::info!(
            tenant_id = tenant_id,
            file_count = parquet_files.len(),
            "load_events_for_tenant: walking tenant subtree only"
        );

        let mut events = Vec::with_capacity(parquet_files.len() * self.config.batch_size);
        for file_path in parquet_files {
            tracing::debug!(
                tenant_id = tenant_id,
                file = %file_path.display(),
                "load_events_for_tenant: opening file"
            );
            let file_events = self.load_events_from_file(&file_path, tenant_id)?;
            events.extend(file_events);
        }

        tracing::info!(
            tenant_id = tenant_id,
            event_count = events.len(),
            "load_events_for_tenant: complete"
        );
        Ok(events)
    }

    /// Get the storage directory path.
    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    /// One-shot migration of flat-layout files into the tenant-partitioned
    /// tree. Run with Core stopped (no concurrent writes).
    ///
    /// Walks `storage_dir`'s top level (non-recursive) for the legacy
    /// `events-*.parquet` files. For each one it loads the events,
    /// regroups them by (tenant_id, yyyy-mm) — events from a flat file
    /// take the path-derived "default" tenant, since pre-partitioning
    /// data carried no tenant in its on-disk form — writes a fresh
    /// Parquet under the corresponding partition directory, and deletes
    /// the original flat file once the new file is closed.
    ///
    /// `dry_run = true` reports what would happen without touching disk.
    /// Run dry first; production data deserves the rehearsal.
    ///
    /// Crash safety: this writes the new partition file before deleting
    /// the flat file, so a crash between the two leaves both on disk.
    /// The recursive loader (`load_all_events`) will then return both,
    /// duplicating those events on next boot. Mitigation: stop Core
    /// before running, and re-run the migration after any crash so the
    /// flat file gets deleted. A future commit can add atomic rename +
    /// fsync semantics; for the one-time migration the stop-Core
    /// constraint is enough.
    pub fn migrate_flat_layout(&self, dry_run: bool) -> Result<MigrationReport> {
        let flat_files = list_flat_layout_files(&self.storage_dir)?;
        let mut report = MigrationReport {
            dry_run,
            ..Default::default()
        };

        for flat_file in flat_files {
            // Pre-partition events used path-derived tenant. For flat-layout
            // files that's always "default" (`tenant_id_from_path` falls back
            // to "default" for single-component paths).
            let events = self.load_events_from_file(&flat_file, "default")?;
            report.flat_files_seen += 1;

            if events.is_empty() {
                // Stale empty file (zero rows). Just remove it.
                if !dry_run {
                    fs::remove_file(&flat_file).map_err(|e| {
                        AllSourceError::StorageError(format!(
                            "Failed to remove empty flat file {}: {e}",
                            flat_file.display()
                        ))
                    })?;
                }
                report.flat_files_removed += 1;
                continue;
            }

            // Group by (tenant, yyyy-mm-from-event-timestamp). The
            // partition month tracks the event's wall-clock time so that
            // post-migration the layout reflects when data happened, not
            // when migration ran. Step 4 (per-tenant snapshots) and Step
            // 5 (retention) will key on that.
            let mut groups: HashMap<(String, String), Vec<Event>> = HashMap::new();
            for event in events {
                let key = (
                    event.tenant_id_str().to_string(),
                    event.timestamp().format("%Y-%m").to_string(),
                );
                groups.entry(key).or_default().push(event);
            }

            for ((tenant, yyyy_mm), group_events) in groups {
                let count = group_events.len();
                if !dry_run {
                    let safe_tenant = sanitize_tenant_id_for_path(&tenant)?;
                    let target_dir = self.storage_dir.join(safe_tenant).join(&yyyy_mm);
                    fs::create_dir_all(&target_dir).map_err(|e| {
                        AllSourceError::StorageError(format!(
                            "Failed to create partition {}: {e}",
                            target_dir.display()
                        ))
                    })?;
                    let filename = format!(
                        "events-{}-{}.parquet",
                        chrono::Utc::now().format("%Y%m%d-%H%M%S%3f"),
                        uuid::Uuid::new_v4().as_simple()
                    );
                    let target_path = target_dir.join(&filename);
                    let record_batch = self.events_to_record_batch(&group_events)?;
                    let file = File::create(&target_path).map_err(|e| {
                        AllSourceError::StorageError(format!(
                            "Failed to create migration target {}: {e}",
                            target_path.display()
                        ))
                    })?;
                    let props = WriterProperties::builder()
                        .set_compression(self.config.compression)
                        .build();
                    let mut writer =
                        ArrowWriter::try_new(file, self.schema.clone(), Some(props))?;
                    writer.write(&record_batch)?;
                    writer.close()?;
                    report.partitions_written += 1;
                }
                report.events_migrated += count;
            }

            if !dry_run {
                fs::remove_file(&flat_file).map_err(|e| {
                    AllSourceError::StorageError(format!(
                        "Failed to remove flat file {} after migration: {e}",
                        flat_file.display()
                    ))
                })?;
                report.flat_files_removed += 1;
            }
        }

        Ok(report)
    }

    /// Get storage statistics
    pub fn stats(&self) -> Result<StorageStats> {
        let parquet_files = find_parquet_files_recursive(&self.storage_dir)?;
        let mut total_size_bytes = 0u64;
        for path in &parquet_files {
            if let Ok(metadata) = fs::metadata(path) {
                total_size_bytes += metadata.len();
            }
        }

        let current_batch_size: usize = self
            .current_batches
            .lock()
            .unwrap()
            .values()
            .map(Vec::len)
            .sum();

        Ok(StorageStats {
            total_files: parquet_files.len(),
            total_size_bytes,
            storage_dir: self.storage_dir.clone(),
            current_batch_size,
        })
    }
}

/// Validate a tenant ID for use as a filesystem path component.
///
/// Whitelist: ASCII letters, digits, `-`, `_`, `.` — covers UUIDs, the
/// hyphen-and-lowercase tenant strings the onboarding flow produces, and
/// the `system` tenant the heartbeat emitter uses. Rejects empty input,
/// any path separator (`/`, `\`), and any "..". The whitelist is the
/// primary defence against path traversal; the explicit ".." check is
/// belt-and-braces in case the whitelist ever loosens.
///
/// Length capped at 128 bytes — comfortably above the 36-byte UUID and
/// the longest onboarding tenant the system has produced, well below
/// every common filesystem's NAME_MAX (typically 255).
fn sanitize_tenant_id_for_path(tenant_id: &str) -> Result<&str> {
    if tenant_id.is_empty() {
        return Err(AllSourceError::StorageError(
            "tenant_id is empty (cannot derive partition path)".to_string(),
        ));
    }
    if tenant_id.len() > 128 {
        return Err(AllSourceError::StorageError(format!(
            "tenant_id is too long for partition path: {} bytes (max 128)",
            tenant_id.len()
        )));
    }
    if tenant_id == "." || tenant_id == ".." {
        return Err(AllSourceError::StorageError(format!(
            "tenant_id {tenant_id:?} is reserved"
        )));
    }
    for c in tenant_id.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.';
        if !ok {
            return Err(AllSourceError::StorageError(format!(
                "tenant_id {tenant_id:?} contains disallowed character {c:?} for partition path"
            )));
        }
    }
    Ok(tenant_id)
}

/// Resolve the directory a flush should write into for `(tenant, when)`.
///
/// Returns `<root>/<tenant>/<yyyy-mm>/`. Caller is responsible for
/// `create_dir_all`-ing the result before opening files in it.
fn partition_path_for_tenant(
    root: &Path,
    tenant_id: &str,
    when: chrono::DateTime<chrono::Utc>,
) -> Result<PathBuf> {
    let safe = sanitize_tenant_id_for_path(tenant_id)?;
    Ok(root.join(safe).join(when.format("%Y-%m").to_string()))
}

/// Reverse of `partition_path_for_tenant` — given a parquet file's full
/// path and the storage root, return the tenant_id stored in the path.
///
/// Tenant-partitioned shape: `<root>/<tenant>/<yyyy-mm>/events-*.parquet`
/// → first component after root is the tenant.
///
/// Legacy flat shape: `<root>/events-*.parquet` → no tenant in path, fall
/// back to `"default"` so events written before the partitioning change
/// keep loading with their original (and only ever) tenant identity.
fn tenant_id_from_path(root: &Path, file_path: &Path) -> String {
    let Ok(rel) = file_path.strip_prefix(root) else {
        return "default".to_string();
    };
    let mut comps = rel.components();
    let first = comps.next();
    let next = comps.next();
    match (first, next) {
        // Two or more components: <tenant>/<rest>... → tenant
        (Some(std::path::Component::Normal(tenant)), Some(_)) => {
            tenant.to_string_lossy().into_owned()
        }
        // Single component (the parquet file itself): legacy flat layout.
        _ => "default".to_string(),
    }
}

/// List Parquet files at the top level of `root` only — i.e. the legacy
/// flat-layout files. Used by the one-shot migration tool to find data
/// that needs moving into the tenant-partitioned tree. The opposite of
/// `find_parquet_files_recursive`: stops at the first directory level so
/// already-partitioned data isn't included.
fn list_flat_layout_files(root: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(root).map_err(|e| {
        AllSourceError::StorageError(format!("Failed to read storage directory: {e}"))
    })?;
    let mut out: Vec<PathBuf> = entries
        .filter_map(std::result::Result::ok)
        .filter_map(|entry| {
            let ft = entry.file_type().ok()?;
            if !ft.is_file() {
                return None;
            }
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    out.sort();
    Ok(out)
}

/// Recursively collect all `*.parquet` files under `root`, sorted by path so
/// callers see a deterministic, tenant-grouped order.
///
/// Existence rationale: the storage layout is moving from a flat
/// `storage_dir/events-*.parquet` pile to a tenant-partitioned tree of the
/// shape `storage_dir/<tenant>/<yyyy-mm>/events-*.parquet`. During the
/// migration both shapes coexist, so every code path that asks "what
/// parquet files do we have?" needs to walk subdirectories. Symlinks are
/// not followed — the storage tree is mounted from a single volume and
/// chasing symlinks invites cycles.
fn find_parquet_files_recursive(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            // Root must exist (we created it in `new`); subdirectories may
            // race a delete from compaction. Skip vanished subdirs rather
            // than failing the whole load.
            Err(e) if dir == root => {
                return Err(AllSourceError::StorageError(format!(
                    "Failed to read storage directory: {e}"
                )));
            }
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            // Use file_type() rather than metadata() so symlinks don't get
            // followed by accident (metadata() resolves symlinks, file_type()
            // doesn't).
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "parquet")
            {
                out.push(path);
            }
        }
    }

    out.sort();
    Ok(out)
}

impl Drop for ParquetStorage {
    fn drop(&mut self) {
        // Ensure any remaining events are flushed on shutdown
        if let Err(e) = self.flush_on_shutdown() {
            tracing::error!("Failed to flush events on drop: {}", e);
        }
    }
}

/// Outcome of a `migrate_flat_layout` run.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct MigrationReport {
    /// Whether the run was a rehearsal (no disk changes).
    pub dry_run: bool,
    /// Number of legacy flat-layout files discovered.
    pub flat_files_seen: usize,
    /// Number of legacy flat files deleted (always 0 when `dry_run`).
    pub flat_files_removed: usize,
    /// Number of new partition files written under the tenant tree.
    pub partitions_written: usize,
    /// Total events copied into the new tree (counted in dry-run too).
    pub events_migrated: usize,
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
            let event = create_test_event(&format!("entity-{i}"));
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
                .append_event(create_test_event(&format!("entity-{i}")))
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

        // 250 events for a single tenant. With per-tenant flush, when the
        // tenant's pending batch crosses batch_size we drain the whole
        // tenant in one flush — not chunk at exactly batch_size like the
        // old global-batch path did. So 250 events triggers exactly one
        // size-flush (the appender pushes all 250 onto the tenant's batch
        // under one lock, sees length >= 100, schedules a flush which
        // drains everything). 0 left pending.
        let events: Vec<Event> = (0..250)
            .map(|i| create_test_event(&format!("entity-{i}")))
            .collect();

        let result = storage.batch_write(events).unwrap();
        assert_eq!(result.events_written, 250);
        assert_eq!(result.batches_flushed, 1);
        assert_eq!(storage.pending_count(), 0);

        // Manual flush is a no-op since nothing's pending.
        storage.flush().unwrap();

        // All 250 events round-trip through the tenant-partitioned tree.
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
                .append_event(create_test_event(&format!("entity-{i}")))
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
                .append_event(create_test_event(&format!("entity-{i}")))
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
                        let event = create_test_event(&format!("thread-{t}-entity-{i}"));
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

        // 100 events, single tenant, batch_size=50. Per-tenant flush
        // drains the whole tenant on the first size trigger, so this
        // produces exactly one size-flush and one batches_written event
        // (vs. the pre-tenant world's two).
        let events: Vec<Event> = (0..100)
            .map(|i| create_test_event(&format!("entity-{i}")))
            .collect();

        storage.batch_write(events).unwrap();

        let stats = storage.batch_stats();
        assert_eq!(stats.batches_written, 1);
        assert_eq!(stats.events_written, 100);
        assert!(stats.avg_batch_size > 0.0);
        assert!(stats.events_per_sec > 0.0);
        assert_eq!(stats.size_flushes, 1);
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
            .map(|i| create_test_event(&format!("entity-{i}")))
            .collect();

        let start = std::time::Instant::now();
        let result = storage.batch_write(events).unwrap();
        storage.flush().unwrap(); // Flush any remaining
        let batch_duration = start.elapsed();

        let batch_stats = storage.batch_stats();

        println!("\n=== Parquet Batch Write Performance (BATCH_SIZE=10,000) ===");
        println!("Events: {event_count}");
        println!("Duration: {batch_duration:?}");
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
            let event = create_test_event(&format!("entity-{i}"));
            storage.append_event(event).unwrap();
        }
        let duration = start.elapsed();

        let events_per_sec = f64::from(event_count) / duration.as_secs_f64();

        println!("\n=== Single-Event Write Baseline ===");
        println!("Events: {event_count}");
        println!("Duration: {duration:?}");
        println!("Events/sec: {events_per_sec:.0}");

        // This should be significantly slower than batch writes
        // Used as a baseline to demonstrate 40%+ improvement
    }

    // -----------------------------------------------------------------
    // Tests for the recursive parquet walker (Step 1, commit #1: read-side
    // bidirectional layout support — see SUSTAINABLE_DATA_STRATEGY.md).
    // -----------------------------------------------------------------

    /// Helper: write a tiny placeholder parquet file at an arbitrary path so
    /// the walker has something concrete to find. We only care that the
    /// walker discovers the path, not that the file is loadable here — the
    /// load path is exercised by the existing read tests.
    fn touch_parquet(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"").unwrap();
    }

    #[test]
    fn test_walker_finds_files_in_flat_layout() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        touch_parquet(&root.join("events-20260101-120000000-aaaa.parquet"));
        touch_parquet(&root.join("events-20260101-130000000-bbbb.parquet"));

        let mut found = find_parquet_files_recursive(root).unwrap();
        found.sort();
        assert_eq!(found.len(), 2);
        assert!(
            found[0].file_name().unwrap().to_str().unwrap().starts_with("events-"),
            "expected events-* file, got {found:?}"
        );
    }

    #[test]
    fn test_walker_finds_files_in_tenant_partitioned_tree() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        // Tenant-partitioned shape: storage_dir/<tenant>/<yyyy-mm>/events-*.parquet
        touch_parquet(&root.join("tenant-a/2026-01/events-20260101-120000000-aaaa.parquet"));
        touch_parquet(&root.join("tenant-a/2026-02/events-20260201-120000000-bbbb.parquet"));
        touch_parquet(&root.join("tenant-b/2026-01/events-20260103-120000000-cccc.parquet"));

        let found = find_parquet_files_recursive(root).unwrap();
        assert_eq!(found.len(), 3);
        // Sort places tenant-a files before tenant-b — that's the
        // tenant-grouping the docs claim.
        assert!(found[0].to_str().unwrap().contains("tenant-a"));
        assert!(found[1].to_str().unwrap().contains("tenant-a"));
        assert!(found[2].to_str().unwrap().contains("tenant-b"));
    }

    #[test]
    fn test_walker_handles_mixed_legacy_and_partitioned_layouts() {
        // The migration window: some tenants have been moved into the tree,
        // some flat files still sit at the root. The walker must surface
        // both so load_all_events sees every event regardless of where it
        // currently lives.
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        touch_parquet(&root.join("events-legacy-aaaa.parquet"));
        touch_parquet(&root.join("tenant-a/2026-01/events-new-bbbb.parquet"));

        let found = find_parquet_files_recursive(root).unwrap();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_walker_ignores_non_parquet_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        std::fs::write(root.join("README.md"), b"hello").unwrap();
        std::fs::write(root.join("events.json"), b"[]").unwrap();
        touch_parquet(&root.join("events-20260101-120000000-aaaa.parquet"));
        // Files that just happen to have "parquet" in the name but no
        // .parquet extension stay out — extension-only filter, no name match.
        std::fs::write(root.join("not-a-parquet-file.bin"), b"").unwrap();

        let found = find_parquet_files_recursive(root).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].extension().and_then(|s| s.to_str()), Some("parquet"));
    }

    /// Build an event whose tenant_id and entity_id we control, so tests
    /// can verify per-tenant routing without depending on the helper that
    /// hardcodes "default".
    fn event_with_tenant(tenant: &str, entity_id: &str) -> Event {
        Event::reconstruct_from_strings(
            uuid::Uuid::new_v4(),
            "test.event".to_string(),
            entity_id.to_string(),
            tenant.to_string(),
            json!({"k": "v"}),
            chrono::Utc::now(),
            None,
            1,
        )
    }

    #[test]
    fn test_flush_writes_into_per_tenant_partition() {
        // End-to-end check that the new write path produces
        // <root>/<tenant>/<yyyy-mm>/events-*.parquet — no flat file at the
        // root, no cross-tenant mixing.
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        for i in 0..3 {
            storage
                .append_event(event_with_tenant("default", &format!("entity-{i}")))
                .unwrap();
        }
        storage.flush().unwrap();

        let parquet_files = find_parquet_files_recursive(temp_dir.path()).unwrap();
        assert_eq!(parquet_files.len(), 1);

        let rel = parquet_files[0]
            .strip_prefix(temp_dir.path())
            .unwrap()
            .to_string_lossy()
            .into_owned();
        // Path shape: default/<yyyy-mm>/events-*.parquet
        let parts: Vec<&str> = rel.split(std::path::MAIN_SEPARATOR).collect();
        assert_eq!(parts.len(), 3, "expected tenant/yyyy-mm/file, got {rel}");
        assert_eq!(parts[0], "default");
        // yyyy-mm is two digits dash four — loose check, exact month
        // varies with wall-clock at test runtime.
        assert!(parts[1].len() == 7 && parts[1].as_bytes()[4] == b'-', "expected yyyy-mm, got {}", parts[1]);
        assert!(parts[2].starts_with("events-") && parts[2].ends_with(".parquet"));

        let loaded = storage.load_all_events().unwrap();
        assert_eq!(loaded.len(), 3);
    }

    #[test]
    fn test_multiple_tenants_get_isolated_subtrees() {
        // Per-tenant flush must not mix tenants into the same Parquet file
        // and must put each tenant under its own subdirectory.
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        for i in 0..2 {
            storage.append_event(event_with_tenant("alice", &format!("a-{i}"))).unwrap();
        }
        for i in 0..3 {
            storage.append_event(event_with_tenant("bob", &format!("b-{i}"))).unwrap();
        }
        storage.flush().unwrap();

        let alice_subtree = temp_dir.path().join("alice");
        let bob_subtree = temp_dir.path().join("bob");
        assert!(alice_subtree.is_dir(), "alice should have its own subtree");
        assert!(bob_subtree.is_dir(), "bob should have its own subtree");

        let alice_files = find_parquet_files_recursive(&alice_subtree).unwrap();
        let bob_files = find_parquet_files_recursive(&bob_subtree).unwrap();
        assert_eq!(alice_files.len(), 1);
        assert_eq!(bob_files.len(), 1);

        // Loaded events keep their tenant_id — round-trip preserves which
        // tenant each event belonged to.
        let loaded = storage.load_all_events().unwrap();
        let (alice_count, bob_count) = loaded.iter().fold((0, 0), |(a, b), e| {
            match e.tenant_id_str() {
                "alice" => (a + 1, b),
                "bob" => (a, b + 1),
                _ => (a, b),
            }
        });
        assert_eq!(alice_count, 2);
        assert_eq!(bob_count, 3);
    }

    #[test]
    fn test_size_flush_only_drains_full_tenant() {
        // When one tenant exactly hits batch_size, only that tenant
        // flushes; the other tenant keeps its events buffered. Prevents
        // one noisy tenant from causing fragmented writes for everyone.
        let temp_dir = TempDir::new().unwrap();
        let config = ParquetStorageConfig { batch_size: 5, ..Default::default() };
        let storage = ParquetStorage::with_config(temp_dir.path(), config).unwrap();

        // Alice: 5 events → on the 5th, len == batch_size triggers flush
        // which drains all 5. Alice ends empty.
        for i in 0..5 {
            storage.append_event(event_with_tenant("alice", &format!("a-{i}"))).unwrap();
        }
        // Bob: 2 events → still under threshold, stays pending.
        for i in 0..2 {
            storage.append_event(event_with_tenant("bob", &format!("b-{i}"))).unwrap();
        }

        assert_eq!(storage.pending_count(), 2, "only bob's 2 events should be pending");

        let parquet_files = find_parquet_files_recursive(temp_dir.path()).unwrap();
        assert_eq!(parquet_files.len(), 1, "only alice should have flushed");
        assert!(
            parquet_files[0].to_string_lossy().contains(&format!("alice{}", std::path::MAIN_SEPARATOR)),
            "expected alice partition, got {}", parquet_files[0].display()
        );
    }

    #[test]
    fn test_tenant_id_from_path_recovers_tenant_for_partitioned_files() {
        let root = Path::new("/data/storage");
        let f = Path::new("/data/storage/alice/2026-04/events-20260426-120000000-aaaa.parquet");
        assert_eq!(tenant_id_from_path(root, f), "alice");
    }

    #[test]
    fn test_tenant_id_from_path_falls_back_to_default_for_legacy_flat_layout() {
        let root = Path::new("/data/storage");
        let f = Path::new("/data/storage/events-20260426-120000000-aaaa.parquet");
        // Legacy single-component path. Pre-tenant data was always
        // commingled with tenant=default, so default is the right fallback.
        assert_eq!(tenant_id_from_path(root, f), "default");
    }

    #[test]
    fn test_sanitize_tenant_id_for_path_accepts_safe_inputs() {
        for ok in [
            "default",
            "system",
            "1e6b2d1c-2f64-4441-9cf9-42f2e451aa17",
            "onboard-diagnostic-160-at-example-com",
            "tenant_with_underscore",
            "v1.0",
        ] {
            assert!(
                sanitize_tenant_id_for_path(ok).is_ok(),
                "{ok:?} should be accepted"
            );
        }
    }

    #[test]
    fn test_sanitize_tenant_id_for_path_rejects_unsafe_inputs() {
        for bad in [
            "",            // empty
            "..",          // parent traversal
            ".",           // current dir
            "foo/bar",     // path separator
            "foo\\bar",    // windows-style separator
            "foo bar",     // whitespace
            "foo\nbar",    // newline
            "foo\0bar",    // null byte
            "tenant?",     // shell glob char
            "tenant*",     // shell glob char
        ] {
            assert!(
                sanitize_tenant_id_for_path(bad).is_err(),
                "{bad:?} should be rejected"
            );
        }

        // Length cap.
        let too_long = "a".repeat(129);
        assert!(sanitize_tenant_id_for_path(&too_long).is_err());
    }

    #[test]
    fn test_partition_path_for_tenant_shape() {
        let root = Path::new("/data");
        let when = chrono::DateTime::parse_from_rfc3339("2026-04-26T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let path = partition_path_for_tenant(root, "alice", when).unwrap();
        assert_eq!(path, Path::new("/data/alice/2026-04"));
    }

    #[test]
    fn test_append_event_rejects_unsafe_tenant_at_flush() {
        // Defence in depth: even if some upstream forgets to validate, the
        // sanitizer in flush_tenant catches it. Since append doesn't write
        // synchronously, the bad tenant is rejected on the first flush
        // attempt. We test that flush surfaces an error rather than
        // silently writing somewhere weird.
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        // append accepts whatever tenant_id the event carries — domain
        // construction would normally reject this, but if it slipped
        // through, flush should refuse to derive a path from it.
        storage.append_event(event_with_tenant("../escape", "e-0")).unwrap();
        let result = storage.flush();
        assert!(result.is_err(), "flush should reject unsafe tenant_id");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("disallowed character") || msg.contains("reserved"),
            "expected sanitization error message, got: {msg}"
        );
    }

    // -----------------------------------------------------------------
    // Tenant-pruned read tests (Step 1, commit #4).
    // -----------------------------------------------------------------

    #[test]
    fn test_load_events_for_tenant_only_walks_target_subtree() {
        // Seed three tenants with distinct event counts. Loading one
        // tenant must return only that tenant's events — and the file
        // list helper must report only that tenant's files (the strong
        // form of "didn't open the others").
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        for i in 0..2 {
            storage.append_event(event_with_tenant("alice", &format!("a-{i}"))).unwrap();
        }
        for i in 0..3 {
            storage.append_event(event_with_tenant("bob", &format!("b-{i}"))).unwrap();
        }
        for i in 0..1 {
            storage.append_event(event_with_tenant("carol", &format!("c-{i}"))).unwrap();
        }
        storage.flush().unwrap();

        let alice_files = storage.list_parquet_files_for_tenant("alice").unwrap();
        assert_eq!(alice_files.len(), 1);
        assert!(
            alice_files[0]
                .to_string_lossy()
                .contains(&format!("alice{}", std::path::MAIN_SEPARATOR)),
            "expected alice file, got {}", alice_files[0].display()
        );
        // The pruned listing must NOT include any bob/carol files — this
        // is the property Step 2 will rely on to avoid loading every
        // tenant's data on a single-tenant query.
        for f in &alice_files {
            let s = f.to_string_lossy();
            assert!(!s.contains("bob"), "alice listing leaked bob file: {s}");
            assert!(!s.contains("carol"), "alice listing leaked carol file: {s}");
        }

        let alice_events = storage.load_events_for_tenant("alice").unwrap();
        assert_eq!(alice_events.len(), 2);
        for e in &alice_events {
            assert_eq!(e.tenant_id_str(), "alice");
        }

        let bob_events = storage.load_events_for_tenant("bob").unwrap();
        assert_eq!(bob_events.len(), 3);
        for e in &bob_events {
            assert_eq!(e.tenant_id_str(), "bob");
        }

        let carol_events = storage.load_events_for_tenant("carol").unwrap();
        assert_eq!(carol_events.len(), 1);
        assert_eq!(carol_events[0].tenant_id_str(), "carol");
    }

    #[test]
    fn test_load_events_for_tenant_returns_empty_when_subtree_missing() {
        // Querying a tenant that has never written must not error — it's
        // a normal "no data" case, not a misconfiguration. Important for
        // first-query latency on a fresh tenant.
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        // Seed only alice so the storage_dir isn't empty (rule out the
        // empty-dir trivial case).
        storage.append_event(event_with_tenant("alice", "a-0")).unwrap();
        storage.flush().unwrap();

        let files = storage.list_parquet_files_for_tenant("nobody-here").unwrap();
        assert!(files.is_empty());

        let events = storage.load_events_for_tenant("nobody-here").unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_load_events_for_tenant_rejects_unsafe_tenant_id() {
        // Path traversal must fail at the API boundary, not after disk
        // reads. Same whitelist as the write path.
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        for unsafe_tid in ["..", "a/b", "a\\b", "", "a..b/.."] {
            let result = storage.load_events_for_tenant(unsafe_tid);
            assert!(
                result.is_err(),
                "tenant_id {unsafe_tid:?} should have been rejected"
            );
        }
    }

    #[test]
    fn test_load_events_for_tenant_ignores_legacy_flat_layout_files() {
        // Flat-layout files at the storage root predate partitioning. A
        // tenant-scoped load must not pick them up — the migration tool
        // is what relocates them under default/. Until it runs, those
        // files are invisible to per-tenant queries (correct behavior:
        // the system has no way to tell which tenant they belong to
        // beyond "default", and pretending otherwise would mis-attribute
        // them).
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        // Seed a flat-layout file (relocates default/<yyyy-mm>/ → root).
        let _flat = seed_flat_layout_file(&storage, 4);

        // Querying default returns nothing — the flat file at the root
        // isn't under default/.
        let default_events = storage.load_events_for_tenant("default").unwrap();
        assert!(
            default_events.is_empty(),
            "tenant-scoped load must not pick up flat-layout files; got {} events",
            default_events.len()
        );

        // Sanity: the full loader still sees them via the recursive walk.
        let all_events = storage.load_all_events().unwrap();
        assert_eq!(all_events.len(), 4);
    }

    // -----------------------------------------------------------------
    // Migration tests (Step 1, commit #3: flat → tenant-tree migration).
    // -----------------------------------------------------------------

    /// Helper: produce a flat-layout Parquet file at the storage root,
    /// matching what pre-#2 deploys wrote. Uses the existing flush path
    /// briefly and then relocates the resulting file from
    /// default/<yyyy-mm>/ back up to the root, simulating the legacy
    /// state.
    fn seed_flat_layout_file(storage: &ParquetStorage, count: usize) -> PathBuf {
        for i in 0..count {
            storage.append_event(create_test_event(&format!("entity-{i}"))).unwrap();
        }
        storage.flush().unwrap();

        // create_test_event uses tenant="default", so the just-flushed file
        // landed under <root>/default/<yyyy-mm>/. Find the newest file in
        // that subtree to avoid picking up files from other tenants seeded
        // by the test before us.
        let default_subtree = storage.storage_dir().join("default");
        let candidates = find_parquet_files_recursive(&default_subtree).unwrap();
        assert!(
            !candidates.is_empty(),
            "seed expected at least one file under default/"
        );
        let src = candidates.into_iter().max().unwrap();

        let dst = storage.storage_dir().join(src.file_name().unwrap());
        std::fs::rename(&src, &dst).unwrap();
        // Best-effort cleanup of the now-empty intermediate dirs so the
        // migration tool only sees the flat file. (`remove_dir` succeeds
        // only on empty dirs, which is exactly the safety we want here.)
        if let Some(month_dir) = src.parent() {
            let _ = std::fs::remove_dir(month_dir);
            if let Some(tenant_dir) = month_dir.parent() {
                let _ = std::fs::remove_dir(tenant_dir);
            }
        }
        dst
    }

    #[test]
    fn test_migrate_flat_layout_dry_run_touches_nothing() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();
        let flat = seed_flat_layout_file(&storage, 7);
        assert!(flat.is_file(), "test setup: flat file should exist");

        let report = storage.migrate_flat_layout(true).unwrap();
        assert!(report.dry_run);
        assert_eq!(report.flat_files_seen, 1);
        assert_eq!(report.events_migrated, 7);
        assert_eq!(report.flat_files_removed, 0);
        assert_eq!(report.partitions_written, 0);
        assert!(flat.is_file(), "flat file must still be present after dry run");
    }

    #[test]
    fn test_migrate_flat_layout_moves_events_into_default_tree_and_removes_flat() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();
        let flat = seed_flat_layout_file(&storage, 5);

        let report = storage.migrate_flat_layout(false).unwrap();
        assert!(!report.dry_run);
        assert_eq!(report.flat_files_seen, 1);
        assert_eq!(report.flat_files_removed, 1);
        assert_eq!(report.events_migrated, 5);
        assert!(report.partitions_written >= 1);
        assert!(!flat.exists(), "flat file should be deleted after migration");

        let post = find_parquet_files_recursive(temp_dir.path()).unwrap();
        assert!(
            post.iter().all(|p| {
                let rel = p.strip_prefix(temp_dir.path()).unwrap().to_string_lossy().into_owned();
                rel.starts_with(&format!("default{}", std::path::MAIN_SEPARATOR))
            }),
            "all migrated files should be under default/"
        );

        let loaded = storage.load_all_events().unwrap();
        assert_eq!(loaded.len(), 5);
    }

    #[test]
    fn test_migrate_flat_layout_is_idempotent_when_re_run_after_completion() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();
        let _flat = seed_flat_layout_file(&storage, 4);

        let first = storage.migrate_flat_layout(false).unwrap();
        assert_eq!(first.events_migrated, 4);

        // Second run sees no flat files at the root, so it's a no-op —
        // events do not duplicate even if an operator runs the tool twice.
        let second = storage.migrate_flat_layout(false).unwrap();
        assert_eq!(second.flat_files_seen, 0);
        assert_eq!(second.events_migrated, 0);
        assert_eq!(second.flat_files_removed, 0);

        let loaded = storage.load_all_events().unwrap();
        assert_eq!(loaded.len(), 4, "rerun must not duplicate or lose events");
    }

    #[test]
    fn test_migrate_flat_layout_ignores_already_partitioned_data() {
        // Mixed state: a tenant tree already exists alongside one flat file.
        // Migration must touch only the flat file.
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();

        for i in 0..3 {
            storage.append_event(event_with_tenant("alice", &format!("a-{i}"))).unwrap();
        }
        storage.flush().unwrap();

        let _flat = seed_flat_layout_file(&storage, 2);

        let report = storage.migrate_flat_layout(false).unwrap();
        assert_eq!(report.flat_files_seen, 1, "only the flat file is in scope");
        assert_eq!(report.events_migrated, 2);

        let alice_files = find_parquet_files_recursive(&temp_dir.path().join("alice")).unwrap();
        assert_eq!(alice_files.len(), 1, "alice's tree must be untouched");

        let loaded = storage.load_all_events().unwrap();
        assert_eq!(loaded.len(), 5);
        let alice_count = loaded.iter().filter(|e| e.tenant_id_str() == "alice").count();
        let default_count = loaded.iter().filter(|e| e.tenant_id_str() == "default").count();
        assert_eq!(alice_count, 3);
        assert_eq!(default_count, 2);
    }

    #[test]
    fn test_migrate_flat_layout_with_no_flat_files_is_a_clean_noop() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ParquetStorage::new(temp_dir.path()).unwrap();
        let report = storage.migrate_flat_layout(false).unwrap();
        assert_eq!(report.flat_files_seen, 0);
        assert_eq!(report.events_migrated, 0);
    }
}
