use crate::error::{AllSourceError, Result};
use crate::event::Event;
use arrow::array::{
    Array, ArrayRef, StringBuilder, TimestampMicrosecondArray, TimestampMicrosecondBuilder,
    UInt64Builder,
};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Parquet-based persistent storage for events
pub struct ParquetStorage {
    /// Base directory for storing parquet files
    storage_dir: PathBuf,

    /// Current batch being accumulated
    current_batch: Vec<Event>,

    /// Batch size before flushing to disk
    batch_size: usize,

    /// Schema for Arrow/Parquet
    schema: Arc<Schema>,
}

impl ParquetStorage {
    pub fn new(storage_dir: impl AsRef<Path>) -> Result<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // Create storage directory if it doesn't exist
        fs::create_dir_all(&storage_dir).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to create storage directory: {}", e))
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
            current_batch: Vec::new(),
            batch_size: 1000, // Flush every 1000 events
            schema,
        })
    }

    /// Add an event to the current batch
    pub fn append_event(&mut self, event: Event) -> Result<()> {
        self.current_batch.push(event);

        // Auto-flush if batch is full
        if self.current_batch.len() >= self.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    /// Flush current batch to a Parquet file
    pub fn flush(&mut self) -> Result<()> {
        if self.current_batch.is_empty() {
            return Ok(());
        }

        let batch_count = self.current_batch.len();
        tracing::info!("Flushing {} events to Parquet storage", batch_count);

        // Create record batch from events
        let record_batch = self.events_to_record_batch(&self.current_batch)?;

        // Generate filename with timestamp
        let filename = format!(
            "events-{}.parquet",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        );
        let file_path = self.storage_dir.join(filename);

        // Write to Parquet file
        let file = File::create(&file_path).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to create parquet file: {}", e))
        })?;

        let props = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build();

        let mut writer = ArrowWriter::try_new(file, self.schema.clone(), Some(props))?;

        writer.write(&record_batch)?;
        writer.close()?;

        tracing::info!(
            "Successfully wrote {} events to {}",
            batch_count,
            file_path.display()
        );

        // Clear current batch
        self.current_batch.clear();

        Ok(())
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
            event_type_builder.append_value(&event.event_type);
            entity_id_builder.append_value(&event.entity_id);
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
            AllSourceError::StorageError(format!("Failed to read storage directory: {}", e))
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
            AllSourceError::StorageError(format!("Failed to open parquet file: {}", e))
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
            .ok_or_else(|| {
                AllSourceError::StorageError("Invalid event_type column".to_string())
            })?;

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
            let event = Event {
                id: uuid::Uuid::parse_str(event_ids.value(i)).map_err(|e| {
                    AllSourceError::StorageError(format!("Invalid UUID: {}", e))
                })?,
                event_type: event_types.value(i).to_string(),
                entity_id: entity_ids.value(i).to_string(),
                payload: serde_json::from_str(payloads.value(i))?,
                timestamp: chrono::DateTime::from_timestamp_micros(timestamps.value(i))
                    .ok_or_else(|| {
                        AllSourceError::StorageError("Invalid timestamp".to_string())
                    })?,
                metadata: if metadatas.is_null(i) {
                    None
                } else {
                    Some(serde_json::from_str(metadatas.value(i))?)
                },
                version: versions.value(i) as i64,
            };

            events.push(event);
        }

        Ok(events)
    }

    /// Get storage statistics
    pub fn stats(&self) -> Result<StorageStats> {
        let entries = fs::read_dir(&self.storage_dir).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to read storage directory: {}", e))
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

        Ok(StorageStats {
            total_files,
            total_size_bytes,
            storage_dir: self.storage_dir.clone(),
            current_batch_size: self.current_batch.len(),
        })
    }
}

impl Drop for ParquetStorage {
    fn drop(&mut self) {
        // Ensure any remaining events are flushed
        if !self.current_batch.is_empty() {
            if let Err(e) = self.flush() {
                tracing::error!("Failed to flush events on drop: {}", e);
            }
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
    use tempfile::TempDir;

    fn create_test_event(entity_id: &str) -> Event {
        Event {
            id: uuid::Uuid::new_v4(),
            event_type: "test.event".to_string(),
            entity_id: entity_id.to_string(),
            payload: json!({
                "test": "data",
                "value": 42
            }),
            timestamp: chrono::Utc::now(),
            metadata: None,
            version: 1,
        }
    }

    #[test]
    fn test_parquet_storage_write_read() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = ParquetStorage::new(temp_dir.path()).unwrap();

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
        let mut storage = ParquetStorage::new(temp_dir.path()).unwrap();

        // Add and flush events
        for i in 0..5 {
            storage.append_event(create_test_event(&format!("entity-{}", i))).unwrap();
        }
        storage.flush().unwrap();

        let stats = storage.stats().unwrap();
        assert_eq!(stats.total_files, 1);
        assert!(stats.total_size_bytes > 0);
    }
}
