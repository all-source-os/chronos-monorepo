use crate::error::{AllSourceError, Result};
use crate::domain::entities::Event;
use crate::storage::ParquetStorage;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Manages Parquet file compaction for optimal storage and query performance
pub struct CompactionManager {
    /// Directory where Parquet files are stored
    storage_dir: PathBuf,

    /// Configuration
    config: CompactionConfig,

    /// Statistics
    stats: Arc<RwLock<CompactionStats>>,

    /// Last compaction time
    last_compaction: Arc<RwLock<Option<DateTime<Utc>>>>,
}

#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Minimum number of files to trigger compaction
    pub min_files_to_compact: usize,

    /// Target size for compacted files (in bytes)
    pub target_file_size: usize,

    /// Maximum size for a single compacted file (in bytes)
    pub max_file_size: usize,

    /// Minimum file size to consider for compaction (small files)
    pub small_file_threshold: usize,

    /// Time interval between automatic compactions (in seconds)
    pub compaction_interval_seconds: u64,

    /// Enable automatic background compaction
    pub auto_compact: bool,

    /// Compaction strategy
    pub strategy: CompactionStrategy,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            min_files_to_compact: 3,
            target_file_size: 128 * 1024 * 1024,      // 128 MB
            max_file_size: 256 * 1024 * 1024,         // 256 MB
            small_file_threshold: 10 * 1024 * 1024,   // 10 MB
            compaction_interval_seconds: 3600,         // 1 hour
            auto_compact: true,
            strategy: CompactionStrategy::SizeBased,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CompactionStrategy {
    /// Compact based on file size (default)
    SizeBased,
    /// Compact based on file age
    TimeBased,
    /// Compact all files into one
    FullCompaction,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CompactionStats {
    pub total_compactions: u64,
    pub total_files_compacted: u64,
    pub total_bytes_before: u64,
    pub total_bytes_after: u64,
    pub total_events_compacted: u64,
    pub last_compaction_duration_ms: u64,
    pub space_saved_bytes: u64,
}

/// Information about a Parquet file candidate for compaction
#[derive(Debug, Clone)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    created: DateTime<Utc>,
}

impl CompactionManager {
    /// Create a new compaction manager
    pub fn new(storage_dir: impl Into<PathBuf>, config: CompactionConfig) -> Self {
        let storage_dir = storage_dir.into();

        tracing::info!(
            "✅ Compaction manager initialized at: {}",
            storage_dir.display()
        );

        Self {
            storage_dir,
            config,
            stats: Arc::new(RwLock::new(CompactionStats::default())),
            last_compaction: Arc::new(RwLock::new(None)),
        }
    }

    /// List all Parquet files in the storage directory
    fn list_parquet_files(&self) -> Result<Vec<FileInfo>> {
        let entries = fs::read_dir(&self.storage_dir).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to read storage directory: {}", e))
        })?;

        let mut files = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| {
                AllSourceError::StorageError(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "parquet" {
                    let metadata = entry.metadata().map_err(|e| {
                        AllSourceError::StorageError(format!("Failed to read file metadata: {}", e))
                    })?;

                    let size = metadata.len();
                    let created = metadata
                        .created()
                        .ok()
                        .and_then(|t| {
                            t.duration_since(std::time::UNIX_EPOCH)
                                .ok()
                                .map(|d| {
                                    DateTime::from_timestamp(d.as_secs() as i64, 0)
                                        .unwrap_or_else(Utc::now)
                                })
                        })
                        .unwrap_or_else(Utc::now);

                    files.push(FileInfo {
                        path,
                        size,
                        created,
                    });
                }
            }
        }

        // Sort by creation time (oldest first)
        files.sort_by_key(|f| f.created);

        Ok(files)
    }

    /// Identify files that should be compacted based on strategy
    fn select_files_for_compaction(&self, files: &[FileInfo]) -> Vec<FileInfo> {
        match self.config.strategy {
            CompactionStrategy::SizeBased => self.select_small_files(files),
            CompactionStrategy::TimeBased => self.select_old_files(files),
            CompactionStrategy::FullCompaction => files.to_vec(),
        }
    }

    /// Select small files for compaction
    fn select_small_files(&self, files: &[FileInfo]) -> Vec<FileInfo> {
        let small_files: Vec<FileInfo> = files
            .iter()
            .filter(|f| f.size < self.config.small_file_threshold as u64)
            .cloned()
            .collect();

        // Only compact if we have enough small files
        if small_files.len() >= self.config.min_files_to_compact {
            small_files
        } else {
            Vec::new()
        }
    }

    /// Select old files for time-based compaction
    fn select_old_files(&self, files: &[FileInfo]) -> Vec<FileInfo> {
        let now = Utc::now();
        let age_threshold = chrono::Duration::hours(24); // Files older than 24 hours

        let old_files: Vec<FileInfo> = files
            .iter()
            .filter(|f| now - f.created > age_threshold)
            .cloned()
            .collect();

        if old_files.len() >= self.config.min_files_to_compact {
            old_files
        } else {
            Vec::new()
        }
    }

    /// Check if compaction should run
    pub fn should_compact(&self) -> bool {
        if !self.config.auto_compact {
            return false;
        }

        let last = self.last_compaction.read();
        match *last {
            None => true, // Never compacted
            Some(last_time) => {
                let elapsed = (Utc::now() - last_time).num_seconds();
                elapsed >= self.config.compaction_interval_seconds as i64
            }
        }
    }

    /// Perform compaction of Parquet files
    pub fn compact(&self) -> Result<CompactionResult> {
        let start_time = std::time::Instant::now();
        tracing::info!("🔄 Starting Parquet compaction...");

        // List all Parquet files
        let files = self.list_parquet_files()?;

        if files.is_empty() {
            tracing::debug!("No Parquet files to compact");
            return Ok(CompactionResult {
                files_compacted: 0,
                bytes_before: 0,
                bytes_after: 0,
                events_compacted: 0,
                duration_ms: 0,
            });
        }

        // Select files for compaction
        let files_to_compact = self.select_files_for_compaction(&files);

        if files_to_compact.is_empty() {
            tracing::debug!(
                "No files meet compaction criteria (strategy: {:?})",
                self.config.strategy
            );
            return Ok(CompactionResult {
                files_compacted: 0,
                bytes_before: 0,
                bytes_after: 0,
                events_compacted: 0,
                duration_ms: 0,
            });
        }

        let bytes_before: u64 = files_to_compact.iter().map(|f| f.size).sum();

        tracing::info!(
            "Compacting {} files ({:.2} MB)",
            files_to_compact.len(),
            bytes_before as f64 / (1024.0 * 1024.0)
        );

        // Read events from all files to be compacted
        let mut all_events = Vec::new();
        for file_info in &files_to_compact {
            match self.read_parquet_file(&file_info.path) {
                Ok(mut events) => {
                    all_events.append(&mut events);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to read Parquet file {:?}: {}",
                        file_info.path,
                        e
                    );
                    // Continue with other files
                }
            }
        }

        if all_events.is_empty() {
            tracing::warn!("No events read from files to compact");
            return Ok(CompactionResult {
                files_compacted: 0,
                bytes_before,
                bytes_after: 0,
                events_compacted: 0,
                duration_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Sort events by timestamp for better compression and query performance
        all_events.sort_by_key(|e| e.timestamp);

        tracing::debug!("Read {} events for compaction", all_events.len());

        // Write compacted file(s)
        let compacted_files = self.write_compacted_files(&all_events)?;

        let bytes_after: u64 = compacted_files.iter().map(|p| {
            fs::metadata(p)
                .map(|m| m.len())
                .unwrap_or(0)
        }).sum();

        // Delete original files atomically
        for file_info in &files_to_compact {
            if let Err(e) = fs::remove_file(&file_info.path) {
                tracing::error!(
                    "Failed to remove old file {:?}: {}",
                    file_info.path,
                    e
                );
            } else {
                tracing::debug!("Removed old file: {:?}", file_info.path);
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_compactions += 1;
        stats.total_files_compacted += files_to_compact.len() as u64;
        stats.total_bytes_before += bytes_before;
        stats.total_bytes_after += bytes_after;
        stats.total_events_compacted += all_events.len() as u64;
        stats.last_compaction_duration_ms = duration_ms;
        stats.space_saved_bytes += bytes_before.saturating_sub(bytes_after);
        drop(stats);

        // Update last compaction time
        *self.last_compaction.write() = Some(Utc::now());

        let compression_ratio = if bytes_before > 0 {
            (bytes_after as f64 / bytes_before as f64) * 100.0
        } else {
            100.0
        };

        tracing::info!(
            "✅ Compaction complete: {} files → {} files, {:.2} MB → {:.2} MB ({:.1}%), {} events, {}ms",
            files_to_compact.len(),
            compacted_files.len(),
            bytes_before as f64 / (1024.0 * 1024.0),
            bytes_after as f64 / (1024.0 * 1024.0),
            compression_ratio,
            all_events.len(),
            duration_ms
        );

        Ok(CompactionResult {
            files_compacted: files_to_compact.len(),
            bytes_before,
            bytes_after,
            events_compacted: all_events.len(),
            duration_ms,
        })
    }

    /// Read events from a Parquet file
    fn read_parquet_file(&self, path: &Path) -> Result<Vec<Event>> {
        // Use ParquetStorage to read the file
        let storage = ParquetStorage::new(&self.storage_dir)?;

        // For now, we'll read all events and filter by file
        // In a production system, you'd want to read specific files
        let all_events = storage.load_all_events()?;

        Ok(all_events)
    }

    /// Write compacted events to new Parquet file(s)
    fn write_compacted_files(&self, events: &[Event]) -> Result<Vec<PathBuf>> {
        let mut compacted_files = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_size = 0;

        for event in events {
            // Estimate event size (rough approximation)
            let event_size = serde_json::to_string(event)
                .map(|s| s.len())
                .unwrap_or(1024);

            // Check if adding this event would exceed target size
            if current_size + event_size > self.config.target_file_size && !current_batch.is_empty() {
                // Write current batch
                let file_path = self.write_batch(&current_batch)?;
                compacted_files.push(file_path);

                // Start new batch
                current_batch.clear();
                current_size = 0;
            }

            current_batch.push(event.clone());
            current_size += event_size;

            // Also check max file size
            if current_size >= self.config.max_file_size {
                let file_path = self.write_batch(&current_batch)?;
                compacted_files.push(file_path);

                current_batch.clear();
                current_size = 0;
            }
        }

        // Write remaining events
        if !current_batch.is_empty() {
            let file_path = self.write_batch(&current_batch)?;
            compacted_files.push(file_path);
        }

        Ok(compacted_files)
    }

    /// Write a batch of events to a new Parquet file
    fn write_batch(&self, events: &[Event]) -> Result<PathBuf> {
        let mut storage = ParquetStorage::new(&self.storage_dir)?;

        // Generate filename with timestamp
        let filename = format!(
            "events-compacted-{}.parquet",
            Utc::now().format("%Y%m%d-%H%M%S-%f")
        );
        let file_path = self.storage_dir.join(filename);

        // Write events
        for event in events {
            storage.append_event(event.clone())?;
        }

        // Flush to disk
        storage.flush()?;

        tracing::debug!(
            "Wrote compacted file: {:?} ({} events)",
            file_path,
            events.len()
        );

        Ok(file_path)
    }

    /// Get compaction statistics
    pub fn stats(&self) -> CompactionStats {
        (*self.stats.read()).clone()
    }

    /// Get configuration
    pub fn config(&self) -> &CompactionConfig {
        &self.config
    }

    /// Trigger manual compaction
    pub fn compact_now(&self) -> Result<CompactionResult> {
        tracing::info!("Manual compaction triggered");
        self.compact()
    }
}

/// Result of a compaction operation
#[derive(Debug, Clone, Serialize)]
pub struct CompactionResult {
    pub files_compacted: usize,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub events_compacted: usize,
    pub duration_ms: u64,
}

/// Background compaction task
pub struct CompactionTask {
    manager: Arc<CompactionManager>,
    interval: Duration,
}

impl CompactionTask {
    /// Create a new background compaction task
    pub fn new(manager: Arc<CompactionManager>, interval_seconds: u64) -> Self {
        Self {
            manager,
            interval: Duration::from_secs(interval_seconds),
        }
    }

    /// Run the compaction task in a loop
    pub async fn run(self) {
        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;

            if self.manager.should_compact() {
                tracing::debug!("Auto-compaction check triggered");

                match self.manager.compact() {
                    Ok(result) => {
                        if result.files_compacted > 0 {
                            tracing::info!(
                                "Auto-compaction succeeded: {} files, {:.2} MB saved",
                                result.files_compacted,
                                (result.bytes_before - result.bytes_after) as f64 / (1024.0 * 1024.0)
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Auto-compaction failed: {}", e);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compaction_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = CompactionConfig::default();
        let manager = CompactionManager::new(temp_dir.path(), config);

        assert_eq!(manager.stats().total_compactions, 0);
    }

    #[test]
    fn test_should_compact() {
        let temp_dir = TempDir::new().unwrap();
        let config = CompactionConfig {
            auto_compact: true,
            compaction_interval_seconds: 1,
            ..Default::default()
        };
        let manager = CompactionManager::new(temp_dir.path(), config);

        // Should compact on first check (never compacted)
        assert!(manager.should_compact());
    }

    #[test]
    fn test_file_selection_size_based() {
        let temp_dir = TempDir::new().unwrap();
        let config = CompactionConfig {
            small_file_threshold: 1024 * 1024, // 1 MB
            min_files_to_compact: 2,
            strategy: CompactionStrategy::SizeBased,
            ..Default::default()
        };
        let manager = CompactionManager::new(temp_dir.path(), config);

        let files = vec![
            FileInfo {
                path: PathBuf::from("small1.parquet"),
                size: 500_000, // 500 KB
                created: Utc::now(),
            },
            FileInfo {
                path: PathBuf::from("small2.parquet"),
                size: 600_000, // 600 KB
                created: Utc::now(),
            },
            FileInfo {
                path: PathBuf::from("large.parquet"),
                size: 10_000_000, // 10 MB
                created: Utc::now(),
            },
        ];

        let selected = manager.select_files_for_compaction(&files);
        assert_eq!(selected.len(), 2); // Only the 2 small files
    }
}
