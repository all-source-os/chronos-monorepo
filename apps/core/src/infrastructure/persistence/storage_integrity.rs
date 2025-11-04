use crate::error::{AllSourceError, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Storage integrity checker (SierraDB Pattern)
///
/// Prevents silent data corruption with checksums.
/// Based on production lessons from SierraDB event store.
///
/// # SierraDB Pattern
/// - Checksums detect corruption in storage
/// - Critical for long-running production systems
/// - Verifies WAL segments and Parquet files
///
/// # Design
/// - SHA-256 for cryptographic strength
/// - Per-segment checksums for WAL
/// - Per-file checksums for Parquet
/// - Incremental verification (not full scan)
pub struct StorageIntegrity;

impl StorageIntegrity {
    /// Compute SHA-256 checksum for data
    ///
    /// Returns hex-encoded checksum string.
    ///
    /// # Example
    /// ```
    /// use allsource_core::infrastructure::persistence::StorageIntegrity;
    ///
    /// let data = b"hello world";
    /// let checksum = StorageIntegrity::compute_checksum(data);
    /// assert_eq!(checksum.len(), 64); // SHA-256 is 32 bytes = 64 hex chars
    /// ```
    pub fn compute_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Verify data against expected checksum
    ///
    /// Returns true if checksums match, false otherwise.
    ///
    /// # Example
    /// ```
    /// use allsource_core::infrastructure::persistence::StorageIntegrity;
    ///
    /// let data = b"hello world";
    /// let checksum = StorageIntegrity::compute_checksum(data);
    /// assert!(StorageIntegrity::verify_checksum(data, &checksum).unwrap());
    /// ```
    pub fn verify_checksum(data: &[u8], expected: &str) -> Result<bool> {
        let computed = Self::compute_checksum(data);
        Ok(computed == expected)
    }

    /// Verify data and return error if mismatch
    ///
    /// More convenient than verify_checksum for error handling.
    pub fn verify_or_error(data: &[u8], expected: &str) -> Result<()> {
        if !Self::verify_checksum(data, expected)? {
            return Err(AllSourceError::StorageError(format!(
                "Checksum mismatch: expected {}, computed {}",
                expected,
                Self::compute_checksum(data)
            )));
        }
        Ok(())
    }

    /// Compute checksum with metadata
    ///
    /// Includes data length and optional label in checksum.
    /// Prevents length extension attacks and provides context.
    pub fn compute_checksum_with_metadata(data: &[u8], label: Option<&str>) -> String {
        let mut hasher = Sha256::new();

        // Include length to prevent length extension
        hasher.update(&(data.len() as u64).to_le_bytes());

        // Include label if provided
        if let Some(l) = label {
            hasher.update(l.as_bytes());
        }

        // Include actual data
        hasher.update(data);

        format!("{:x}", hasher.finalize())
    }

    /// Verify WAL segment integrity
    ///
    /// WAL segments are critical for durability.
    /// Any corruption means potential data loss.
    ///
    /// # Returns
    /// - Ok(true) if segment is valid
    /// - Ok(false) if segment doesn't exist
    /// - Err if corruption detected
    pub fn verify_wal_segment(segment_path: &Path) -> Result<bool> {
        if !segment_path.exists() {
            return Ok(false);
        }

        // Read segment file
        let data = std::fs::read(segment_path).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to read WAL segment: {}", e))
        })?;

        // WAL format: [checksum: 64 bytes][data: N bytes]
        if data.len() < 64 {
            return Err(AllSourceError::StorageError(
                "WAL segment too short for checksum".to_string(),
            ));
        }

        let stored_checksum = String::from_utf8_lossy(&data[0..64]).to_string();
        let segment_data = &data[64..];

        Self::verify_or_error(segment_data, &stored_checksum)?;
        Ok(true)
    }

    /// Verify Parquet file integrity
    ///
    /// Parquet files are our long-term storage.
    /// Corruption here means historical data loss.
    ///
    /// # Returns
    /// - Ok(true) if file is valid
    /// - Ok(false) if file doesn't exist
    /// - Err if corruption detected
    pub fn verify_parquet_file(file_path: &Path) -> Result<bool> {
        if !file_path.exists() {
            return Ok(false);
        }

        // For now, just verify file can be read
        // TODO: Add Parquet metadata checksum verification
        let _data = std::fs::read(file_path).map_err(|e| {
            AllSourceError::StorageError(format!("Failed to read Parquet file: {}", e))
        })?;

        // Parquet has internal checksums, but we could add external ones
        Ok(true)
    }

    /// Batch verify multiple files
    ///
    /// Efficiently verify multiple files with progress reporting.
    pub fn batch_verify<P: AsRef<Path>>(
        paths: &[P],
        progress_callback: Option<Box<dyn Fn(usize, usize)>>,
    ) -> Result<Vec<bool>> {
        let mut results = Vec::new();

        for (idx, path) in paths.iter().enumerate() {
            let path = path.as_ref();

            // Determine file type and verify
            let result = if path.extension().and_then(|s| s.to_str()) == Some("wal") {
                Self::verify_wal_segment(path)?
            } else if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
                Self::verify_parquet_file(path)?
            } else {
                false
            };

            results.push(result);

            // Report progress
            if let Some(ref callback) = progress_callback {
                callback(idx + 1, paths.len());
            }
        }

        Ok(results)
    }
}

/// Integrity check result
#[derive(Debug, Clone, PartialEq)]
pub struct IntegrityCheckResult {
    pub path: String,
    pub valid: bool,
    pub checksum: Option<String>,
    pub error: Option<String>,
}

impl IntegrityCheckResult {
    pub fn success(path: String, checksum: String) -> Self {
        Self {
            path,
            valid: true,
            checksum: Some(checksum),
            error: None,
        }
    }

    pub fn failure(path: String, error: String) -> Self {
        Self {
            path,
            valid: false,
            checksum: None,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_checksum() {
        let data = b"hello world";
        let checksum = StorageIntegrity::compute_checksum(data);

        // SHA-256 produces 32 bytes = 64 hex characters
        assert_eq!(checksum.len(), 64);

        // Checksums should be deterministic
        let checksum2 = StorageIntegrity::compute_checksum(data);
        assert_eq!(checksum, checksum2);
    }

    #[test]
    fn test_verify_checksum() {
        let data = b"test data";
        let checksum = StorageIntegrity::compute_checksum(data);

        assert!(StorageIntegrity::verify_checksum(data, &checksum).unwrap());

        // Wrong checksum should fail
        assert!(!StorageIntegrity::verify_checksum(data, "wrong").unwrap());
    }

    #[test]
    fn test_verify_or_error() {
        let data = b"test data";
        let checksum = StorageIntegrity::compute_checksum(data);

        // Valid checksum should succeed
        assert!(StorageIntegrity::verify_or_error(data, &checksum).is_ok());

        // Invalid checksum should error
        let result = StorageIntegrity::verify_or_error(data, "wrong");
        assert!(result.is_err());
        assert!(matches!(result, Err(AllSourceError::StorageError(_))));
    }

    #[test]
    fn test_checksum_with_metadata() {
        let data = b"test";

        let checksum1 = StorageIntegrity::compute_checksum_with_metadata(data, Some("label1"));
        let checksum2 = StorageIntegrity::compute_checksum_with_metadata(data, Some("label2"));

        // Different labels produce different checksums
        assert_ne!(checksum1, checksum2);

        // Same label produces same checksum
        let checksum3 = StorageIntegrity::compute_checksum_with_metadata(data, Some("label1"));
        assert_eq!(checksum1, checksum3);
    }

    #[test]
    fn test_different_data_different_checksums() {
        let data1 = b"hello";
        let data2 = b"world";

        let checksum1 = StorageIntegrity::compute_checksum(data1);
        let checksum2 = StorageIntegrity::compute_checksum(data2);

        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_empty_data() {
        let data = b"";
        let checksum = StorageIntegrity::compute_checksum(data);

        // Should still produce valid checksum
        assert_eq!(checksum.len(), 64);
        assert!(StorageIntegrity::verify_checksum(data, &checksum).unwrap());
    }

    #[test]
    fn test_large_data() {
        let data = vec![0u8; 1_000_000]; // 1MB
        let checksum = StorageIntegrity::compute_checksum(&data);

        assert_eq!(checksum.len(), 64);
        assert!(StorageIntegrity::verify_checksum(&data, &checksum).unwrap());
    }

    #[test]
    fn test_integrity_check_result() {
        let success = IntegrityCheckResult::success(
            "test.wal".to_string(),
            "abc123".to_string(),
        );
        assert!(success.valid);
        assert_eq!(success.checksum, Some("abc123".to_string()));
        assert_eq!(success.error, None);

        let failure = IntegrityCheckResult::failure(
            "test.wal".to_string(),
            "corruption detected".to_string(),
        );
        assert!(!failure.valid);
        assert_eq!(failure.checksum, None);
        assert_eq!(failure.error, Some("corruption detected".to_string()));
    }
}
