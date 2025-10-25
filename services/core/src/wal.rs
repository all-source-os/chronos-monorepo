use crate::error::{AllSourceError, Result};
use crate::domain::entities::Event;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Write-Ahead Log for durability and crash recovery
pub struct WriteAheadLog {
    /// Directory where WAL files are stored
    wal_dir: PathBuf,

    /// Current active WAL file
    current_file: Arc<RwLock<WALFile>>,

    /// Configuration
    config: WALConfig,

    /// Statistics
    stats: Arc<RwLock<WALStats>>,

    /// Current sequence number
    sequence: Arc<RwLock<u64>>,
}

#[derive(Debug, Clone)]
pub struct WALConfig {
    /// Maximum size of a single WAL file before rotation (in bytes)
    pub max_file_size: usize,

    /// Whether to sync to disk after each write (fsync)
    pub sync_on_write: bool,

    /// Maximum number of WAL files to keep
    pub max_wal_files: usize,

    /// Enable WAL compression
    pub compress: bool,
}

impl Default for WALConfig {
    fn default() -> Self {
        Self {
            max_file_size: 64 * 1024 * 1024, // 64 MB
            sync_on_write: true,
            max_wal_files: 10,
            compress: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct WALStats {
    pub total_entries: u64,
    pub total_bytes_written: u64,
    pub current_file_size: usize,
    pub files_rotated: u64,
    pub files_cleaned: u64,
    pub recovery_count: u64,
}

/// WAL entry wrapping an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WALEntry {
    /// Sequence number for ordering
    pub sequence: u64,

    /// Timestamp when written to WAL
    pub wal_timestamp: DateTime<Utc>,

    /// The event being logged
    pub event: Event,

    /// Checksum for integrity verification
    pub checksum: u32,
}

impl WALEntry {
    pub fn new(sequence: u64, event: Event) -> Self {
        let mut entry = Self {
            sequence,
            wal_timestamp: Utc::now(),
            event,
            checksum: 0,
        };
        entry.checksum = entry.calculate_checksum();
        entry
    }

    fn calculate_checksum(&self) -> u32 {
        // Simple CRC32 checksum
        let data = format!("{}{}{}", self.sequence, self.wal_timestamp, self.event.id);
        crc32fast::hash(data.as_bytes())
    }

    pub fn verify(&self) -> bool {
        self.checksum == self.calculate_checksum()
    }
}

/// Represents an active WAL file
struct WALFile {
    path: PathBuf,
    writer: BufWriter<File>,
    size: usize,
    created_at: DateTime<Utc>,
}

impl WALFile {
    fn new(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| AllSourceError::StorageError(format!("Failed to open WAL file: {}", e)))?;

        let size = file
            .metadata()
            .map(|m| m.len() as usize)
            .unwrap_or(0);

        Ok(Self {
            path,
            writer: BufWriter::new(file),
            size,
            created_at: Utc::now(),
        })
    }

    fn write_entry(&mut self, entry: &WALEntry, sync: bool) -> Result<usize> {
        // Serialize entry as JSON line
        let json = serde_json::to_string(entry)?;

        let line = format!("{}\n", json);
        let bytes_written = line.len();

        self.writer
            .write_all(line.as_bytes())
            .map_err(|e| AllSourceError::StorageError(format!("Failed to write to WAL: {}", e)))?;

        if sync {
            self.writer
                .flush()
                .map_err(|e| AllSourceError::StorageError(format!("Failed to flush WAL: {}", e)))?;

            self.writer
                .get_ref()
                .sync_all()
                .map_err(|e| AllSourceError::StorageError(format!("Failed to sync WAL: {}", e)))?;
        }

        self.size += bytes_written;
        Ok(bytes_written)
    }

    fn flush(&mut self) -> Result<()> {
        self.writer
            .flush()
            .map_err(|e| AllSourceError::StorageError(format!("Failed to flush WAL: {}", e)))?;
        Ok(())
    }
}

impl WriteAheadLog {
    /// Create a new WAL
    pub fn new(wal_dir: impl Into<PathBuf>, config: WALConfig) -> Result<Self> {
        let wal_dir = wal_dir.into();

        // Create WAL directory if it doesn't exist
        fs::create_dir_all(&wal_dir)
            .map_err(|e| AllSourceError::StorageError(format!("Failed to create WAL directory: {}", e)))?;

        // Create initial WAL file
        let initial_file_path = Self::generate_wal_filename(&wal_dir, 0);
        let current_file = WALFile::new(initial_file_path)?;

        tracing::info!("✅ WAL initialized at: {}", wal_dir.display());

        Ok(Self {
            wal_dir,
            current_file: Arc::new(RwLock::new(current_file)),
            config,
            stats: Arc::new(RwLock::new(WALStats::default())),
            sequence: Arc::new(RwLock::new(0)),
        })
    }

    /// Generate a WAL filename based on sequence
    fn generate_wal_filename(dir: &Path, sequence: u64) -> PathBuf {
        dir.join(format!("wal-{:016x}.log", sequence))
    }

    /// Write an event to the WAL
    pub fn append(&self, event: Event) -> Result<u64> {
        // Get next sequence number
        let mut seq = self.sequence.write();
        *seq += 1;
        let sequence = *seq;
        drop(seq);

        // Create WAL entry
        let entry = WALEntry::new(sequence, event);

        // Write to current file
        let mut current = self.current_file.write();
        let bytes_written = current.write_entry(&entry, self.config.sync_on_write)?;

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_entries += 1;
        stats.total_bytes_written += bytes_written as u64;
        stats.current_file_size = current.size;
        drop(stats);

        // Check if we need to rotate
        let should_rotate = current.size >= self.config.max_file_size;
        drop(current);

        if should_rotate {
            self.rotate()?;
        }

        tracing::trace!("WAL entry written: sequence={}", sequence);

        Ok(sequence)
    }

    /// Rotate to a new WAL file
    fn rotate(&self) -> Result<()> {
        let seq = *self.sequence.read();
        let new_file_path = Self::generate_wal_filename(&self.wal_dir, seq);

        tracing::info!("🔄 Rotating WAL to new file: {:?}", new_file_path);

        let new_file = WALFile::new(new_file_path)?;

        let mut current = self.current_file.write();
        current.flush()?;
        *current = new_file;

        let mut stats = self.stats.write();
        stats.files_rotated += 1;
        stats.current_file_size = 0;
        drop(stats);

        // Clean up old WAL files
        self.cleanup_old_files()?;

        Ok(())
    }

    /// Clean up old WAL files beyond the retention limit
    fn cleanup_old_files(&self) -> Result<()> {
        let mut wal_files = self.list_wal_files()?;
        wal_files.sort();

        if wal_files.len() > self.config.max_wal_files {
            let to_remove = wal_files.len() - self.config.max_wal_files;
            let files_to_delete = &wal_files[..to_remove];

            for file_path in files_to_delete {
                if let Err(e) = fs::remove_file(file_path) {
                    tracing::warn!("Failed to remove old WAL file {:?}: {}", file_path, e);
                } else {
                    tracing::debug!("🗑️ Removed old WAL file: {:?}", file_path);
                    let mut stats = self.stats.write();
                    stats.files_cleaned += 1;
                }
            }
        }

        Ok(())
    }

    /// List all WAL files in the directory
    fn list_wal_files(&self) -> Result<Vec<PathBuf>> {
        let entries = fs::read_dir(&self.wal_dir)
            .map_err(|e| AllSourceError::StorageError(format!("Failed to read WAL directory: {}", e)))?;

        let mut wal_files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                AllSourceError::StorageError(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().starts_with("wal-") && name.to_string_lossy().ends_with(".log") {
                    wal_files.push(path);
                }
            }
        }

        Ok(wal_files)
    }

    /// Recover events from WAL files
    pub fn recover(&self) -> Result<Vec<Event>> {
        tracing::info!("🔄 Starting WAL recovery...");

        let mut wal_files = self.list_wal_files()?;
        wal_files.sort();

        let mut recovered_events = Vec::new();
        let mut max_sequence = 0u64;
        let mut corrupted_entries = 0;

        for wal_file_path in &wal_files {
            tracing::debug!("Reading WAL file: {:?}", wal_file_path);

            let file = File::open(wal_file_path).map_err(|e| {
                AllSourceError::StorageError(format!("Failed to open WAL file for recovery: {}", e))
            })?;

            let reader = BufReader::new(file);

            for (line_num, line) in reader.lines().enumerate() {
                let line = line.map_err(|e| {
                    AllSourceError::StorageError(format!("Failed to read WAL line: {}", e))
                })?;

                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<WALEntry>(&line) {
                    Ok(entry) => {
                        // Verify checksum
                        if !entry.verify() {
                            tracing::warn!(
                                "Corrupted WAL entry at {:?}:{} (checksum mismatch)",
                                wal_file_path,
                                line_num + 1
                            );
                            corrupted_entries += 1;
                            continue;
                        }

                        max_sequence = max_sequence.max(entry.sequence);
                        recovered_events.push(entry.event);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse WAL entry at {:?}:{}: {}",
                            wal_file_path,
                            line_num + 1,
                            e
                        );
                        corrupted_entries += 1;
                    }
                }
            }
        }

        // Update sequence counter
        let mut seq = self.sequence.write();
        *seq = max_sequence;
        drop(seq);

        // Update stats
        let mut stats = self.stats.write();
        stats.recovery_count += 1;
        drop(stats);

        tracing::info!(
            "✅ WAL recovery complete: {} events recovered, {} corrupted entries",
            recovered_events.len(),
            corrupted_entries
        );

        Ok(recovered_events)
    }

    /// Manually flush the current WAL file
    pub fn flush(&self) -> Result<()> {
        let mut current = self.current_file.write();
        current.flush()?;
        Ok(())
    }

    /// Truncate WAL after successful checkpoint
    pub fn truncate(&self) -> Result<()> {
        tracing::info!("🧹 Truncating WAL after checkpoint");

        // Close current file
        let mut current = self.current_file.write();
        current.flush()?;

        // Remove all WAL files
        let wal_files = self.list_wal_files()?;
        for file_path in wal_files {
            fs::remove_file(&file_path).map_err(|e| {
                AllSourceError::StorageError(format!("Failed to remove WAL file: {}", e))
            })?;
            tracing::debug!("Removed WAL file: {:?}", file_path);
        }

        // Create new WAL file
        let new_file_path = Self::generate_wal_filename(&self.wal_dir, 0);
        *current = WALFile::new(new_file_path)?;

        // Reset sequence
        let mut seq = self.sequence.write();
        *seq = 0;

        tracing::info!("✅ WAL truncated successfully");

        Ok(())
    }

    /// Get WAL statistics
    pub fn stats(&self) -> WALStats {
        (*self.stats.read()).clone()
    }

    /// Get current sequence number
    pub fn current_sequence(&self) -> u64 {
        *self.sequence.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_event() -> Event {
        Event::reconstruct_from_strings(
            Uuid::new_v4(),
            "test.event".to_string(),
            "test-entity".to_string(),
            "default".to_string(),
            json!({"test": "data"}),
            Utc::now(),
            None,
            1,
        )
    }

    #[test]
    fn test_wal_creation() {
        let temp_dir = TempDir::new().unwrap();
        let wal = WriteAheadLog::new(temp_dir.path(), WALConfig::default());
        assert!(wal.is_ok());
    }

    #[test]
    fn test_wal_append() {
        let temp_dir = TempDir::new().unwrap();
        let wal = WriteAheadLog::new(temp_dir.path(), WALConfig::default()).unwrap();

        let event = create_test_event();
        let seq = wal.append(event);
        assert!(seq.is_ok());
        assert_eq!(seq.unwrap(), 1);

        let stats = wal.stats();
        assert_eq!(stats.total_entries, 1);
    }

    #[test]
    fn test_wal_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let wal = WriteAheadLog::new(temp_dir.path(), WALConfig::default()).unwrap();

        // Write some events
        for _ in 0..5 {
            wal.append(create_test_event()).unwrap();
        }

        wal.flush().unwrap();

        // Create new WAL instance (simulating restart)
        let wal2 = WriteAheadLog::new(temp_dir.path(), WALConfig::default()).unwrap();
        let recovered = wal2.recover().unwrap();

        assert_eq!(recovered.len(), 5);
    }

    #[test]
    fn test_wal_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let config = WALConfig {
            max_file_size: 1024, // Small size to trigger rotation
            ..Default::default()
        };

        let wal = WriteAheadLog::new(temp_dir.path(), config).unwrap();

        // Write enough events to trigger rotation
        for _ in 0..50 {
            wal.append(create_test_event()).unwrap();
        }

        let stats = wal.stats();
        assert!(stats.files_rotated > 0);
    }

    #[test]
    fn test_wal_entry_checksum() {
        let event = create_test_event();
        let entry = WALEntry::new(1, event);

        assert!(entry.verify());

        // Modify and verify it fails
        let mut corrupted = entry.clone();
        corrupted.checksum = 0;
        assert!(!corrupted.verify());
    }

    #[test]
    fn test_wal_truncate() {
        let temp_dir = TempDir::new().unwrap();
        let wal = WriteAheadLog::new(temp_dir.path(), WALConfig::default()).unwrap();

        // Write events
        for _ in 0..5 {
            wal.append(create_test_event()).unwrap();
        }

        // Truncate
        wal.truncate().unwrap();

        // Verify sequence is reset
        assert_eq!(wal.current_sequence(), 0);

        // Verify recovery returns empty
        let recovered = wal.recover().unwrap();
        assert_eq!(recovered.len(), 0);
    }
}
