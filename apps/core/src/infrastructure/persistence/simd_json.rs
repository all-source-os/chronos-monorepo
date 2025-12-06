//! SIMD-accelerated JSON parsing for high-throughput event ingestion
//!
//! This module provides zero-copy JSON parsing using simd-json, which leverages
//! SIMD instructions (AVX2, SSE4.2, NEON) for ~2-3x faster parsing compared to
//! serde_json.
//!
//! # Performance Characteristics
//! - Zero-copy parsing where possible
//! - SIMD-accelerated structural parsing
//! - Batch processing support
//! - Memory-efficient arena allocation

use bumpalo::Bump;
use serde::de::DeserializeOwned;
use simd_json::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Statistics for SIMD JSON parsing performance
#[derive(Debug, Default)]
pub struct SimdJsonStats {
    /// Total bytes parsed
    pub bytes_parsed: AtomicU64,
    /// Total documents parsed
    pub documents_parsed: AtomicU64,
    /// Total parsing time in nanoseconds
    pub parse_time_ns: AtomicU64,
    /// Parse errors encountered
    pub parse_errors: AtomicU64,
}

impl SimdJsonStats {
    /// Create new stats tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Get throughput in MB/s
    pub fn throughput_mbps(&self) -> f64 {
        let bytes = self.bytes_parsed.load(Ordering::Relaxed) as f64;
        let time_ns = self.parse_time_ns.load(Ordering::Relaxed) as f64;
        if time_ns > 0.0 {
            (bytes / 1_000_000.0) / (time_ns / 1_000_000_000.0)
        } else {
            0.0
        }
    }

    /// Get documents per second
    pub fn docs_per_second(&self) -> f64 {
        let docs = self.documents_parsed.load(Ordering::Relaxed) as f64;
        let time_ns = self.parse_time_ns.load(Ordering::Relaxed) as f64;
        if time_ns > 0.0 {
            docs / (time_ns / 1_000_000_000.0)
        } else {
            0.0
        }
    }

    /// Record a successful parse
    fn record_parse(&self, bytes: usize, duration_ns: u64) {
        self.bytes_parsed.fetch_add(bytes as u64, Ordering::Relaxed);
        self.documents_parsed.fetch_add(1, Ordering::Relaxed);
        self.parse_time_ns.fetch_add(duration_ns, Ordering::Relaxed);
    }

    /// Record a parse error
    fn record_error(&self) {
        self.parse_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.bytes_parsed.store(0, Ordering::Relaxed);
        self.documents_parsed.store(0, Ordering::Relaxed);
        self.parse_time_ns.store(0, Ordering::Relaxed);
        self.parse_errors.store(0, Ordering::Relaxed);
    }
}

/// High-performance JSON parser using SIMD instructions
pub struct SimdJsonParser {
    stats: SimdJsonStats,
}

impl Default for SimdJsonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SimdJsonParser {
    /// Create a new SIMD JSON parser
    pub fn new() -> Self {
        Self {
            stats: SimdJsonStats::new(),
        }
    }

    /// Parse JSON bytes into a typed value using SIMD acceleration
    ///
    /// # Arguments
    /// * `data` - Mutable JSON bytes (simd-json requires mutable access for in-place parsing)
    ///
    /// # Returns
    /// Parsed value or error
    pub fn parse<T: DeserializeOwned>(&self, data: &mut [u8]) -> Result<T, SimdJsonError> {
        let start = Instant::now();
        let len = data.len();

        match simd_json::from_slice(data) {
            Ok(value) => {
                let duration = start.elapsed().as_nanos() as u64;
                self.stats.record_parse(len, duration);
                Ok(value)
            }
            Err(e) => {
                self.stats.record_error();
                Err(SimdJsonError::ParseError(e.to_string()))
            }
        }
    }

    /// Parse JSON string into a typed value
    ///
    /// Note: This creates a copy of the string for mutation. For zero-copy,
    /// use `parse` with a mutable byte slice.
    pub fn parse_str<T: DeserializeOwned>(&self, data: &str) -> Result<T, SimdJsonError> {
        let mut bytes = data.as_bytes().to_vec();
        self.parse(&mut bytes)
    }

    /// Parse multiple JSON documents in a batch
    ///
    /// Optimized for processing many small documents efficiently.
    pub fn parse_batch<T: DeserializeOwned>(
        &self,
        documents: &mut [Vec<u8>],
    ) -> Vec<Result<T, SimdJsonError>> {
        documents.iter_mut().map(|doc| self.parse(doc)).collect()
    }

    /// Parse JSON with arena allocation for zero-copy string handling
    ///
    /// The arena provides fast bump allocation and all allocations are freed
    /// together when the arena is dropped.
    pub fn parse_with_arena<'a, T: DeserializeOwned>(
        &self,
        data: &mut [u8],
        _arena: &'a Bump,
    ) -> Result<T, SimdJsonError> {
        // Currently just uses standard parsing, but arena can be used for
        // custom string interning or buffer management in the future
        self.parse(data)
    }

    /// Get parsing statistics
    pub fn stats(&self) -> &SimdJsonStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.stats.reset();
    }
}

/// Errors from SIMD JSON parsing
#[derive(Debug, thiserror::Error)]
pub enum SimdJsonError {
    #[error("JSON parse error: {0}")]
    ParseError(String),

    #[error("Invalid UTF-8 in JSON: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Buffer too small for parsing")]
    BufferTooSmall,
}

/// Batch event parser optimized for high-throughput ingestion
pub struct BatchEventParser {
    parser: SimdJsonParser,
    /// Pre-allocated buffer for batch processing
    buffer_pool: Vec<Vec<u8>>,
    /// Maximum batch size
    max_batch_size: usize,
}

impl BatchEventParser {
    /// Create a new batch parser with specified capacity
    pub fn new(max_batch_size: usize) -> Self {
        Self {
            parser: SimdJsonParser::new(),
            buffer_pool: Vec::with_capacity(max_batch_size),
            max_batch_size,
        }
    }

    /// Parse a batch of JSON event strings
    pub fn parse_events<T: DeserializeOwned>(
        &mut self,
        events: &[String],
    ) -> Vec<Result<T, SimdJsonError>> {
        // Reuse buffers from pool
        self.buffer_pool.clear();
        self.buffer_pool
            .extend(events.iter().map(|e| e.as_bytes().to_vec()));

        self.parser.parse_batch(&mut self.buffer_pool)
    }

    /// Parse events from byte slices (more efficient - avoids string conversion)
    pub fn parse_events_bytes<T: DeserializeOwned>(
        &self,
        events: &mut [Vec<u8>],
    ) -> Vec<Result<T, SimdJsonError>> {
        self.parser.parse_batch(events)
    }

    /// Get the underlying parser's statistics
    pub fn stats(&self) -> &SimdJsonStats {
        self.parser.stats()
    }

    /// Maximum batch size
    pub fn max_batch_size(&self) -> usize {
        self.max_batch_size
    }
}

/// Zero-copy JSON value for read-only access
///
/// This wraps simd-json's tape-based representation for efficient
/// field access without full deserialization.
pub struct ZeroCopyJson<'a> {
    tape: simd_json::BorrowedValue<'a>,
}

impl<'a> ZeroCopyJson<'a> {
    /// Parse JSON into zero-copy representation
    pub fn parse(data: &'a mut [u8]) -> Result<Self, SimdJsonError> {
        let tape = simd_json::to_borrowed_value(data)
            .map_err(|e| SimdJsonError::ParseError(e.to_string()))?;
        Ok(Self { tape })
    }

    /// Get a string field without copying
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.tape.get(key).and_then(|v| v.as_str())
    }

    /// Get a numeric field
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.tape.get(key).and_then(|v| v.as_i64())
    }

    /// Get a float field
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.tape.get(key).and_then(|v| v.as_f64())
    }

    /// Get a boolean field
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.tape.get(key).and_then(|v| v.as_bool())
    }

    /// Check if a field exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.tape.get(key).is_some()
    }

    /// Get the underlying borrowed value
    pub fn as_value(&self) -> &simd_json::BorrowedValue<'a> {
        &self.tape
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestEvent {
        id: String,
        #[serde(rename = "type")]
        event_type: String,
        value: i64,
    }

    #[test]
    fn test_simd_json_parse() {
        let parser = SimdJsonParser::new();
        let mut json = r#"{"id":"123","type":"test","value":42}"#.as_bytes().to_vec();

        let result: TestEvent = parser.parse(&mut json).unwrap();
        assert_eq!(result.id, "123");
        assert_eq!(result.event_type, "test");
        assert_eq!(result.value, 42);
    }

    #[test]
    fn test_simd_json_parse_str() {
        let parser = SimdJsonParser::new();
        let json = r#"{"id":"456","type":"event","value":100}"#;

        let result: TestEvent = parser.parse_str(json).unwrap();
        assert_eq!(result.id, "456");
        assert_eq!(result.event_type, "event");
        assert_eq!(result.value, 100);
    }

    #[test]
    fn test_batch_parsing() {
        let parser = SimdJsonParser::new();
        let mut docs: Vec<Vec<u8>> = vec![
            r#"{"id":"1","type":"a","value":1}"#.as_bytes().to_vec(),
            r#"{"id":"2","type":"b","value":2}"#.as_bytes().to_vec(),
            r#"{"id":"3","type":"c","value":3}"#.as_bytes().to_vec(),
        ];

        let results: Vec<Result<TestEvent, _>> = parser.parse_batch(&mut docs);
        assert_eq!(results.len(), 3);

        for (i, result) in results.into_iter().enumerate() {
            let event = result.unwrap();
            assert_eq!(event.id, (i + 1).to_string());
            assert_eq!(event.value, (i + 1) as i64);
        }
    }

    #[test]
    fn test_stats_tracking() {
        let parser = SimdJsonParser::new();
        let mut json = r#"{"id":"test","type":"event","value":0}"#.as_bytes().to_vec();

        let _: TestEvent = parser.parse(&mut json).unwrap();

        let stats = parser.stats();
        assert!(stats.bytes_parsed.load(Ordering::Relaxed) > 0);
        assert_eq!(stats.documents_parsed.load(Ordering::Relaxed), 1);
        assert_eq!(stats.parse_errors.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_parse_error_tracking() {
        let parser = SimdJsonParser::new();
        let mut invalid = b"not valid json".to_vec();

        let result: Result<TestEvent, _> = parser.parse(&mut invalid);
        assert!(result.is_err());
        assert_eq!(parser.stats().parse_errors.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_zero_copy_json() {
        let mut json = r#"{"name":"test","count":42,"active":true}"#.as_bytes().to_vec();

        let zc = ZeroCopyJson::parse(&mut json).unwrap();
        assert_eq!(zc.get_str("name"), Some("test"));
        assert_eq!(zc.get_i64("count"), Some(42));
        assert_eq!(zc.get_bool("active"), Some(true));
        assert!(zc.contains_key("name"));
        assert!(!zc.contains_key("missing"));
    }

    #[test]
    fn test_batch_event_parser() {
        let mut parser = BatchEventParser::new(100);
        let events = vec![
            r#"{"id":"a","type":"x","value":10}"#.to_string(),
            r#"{"id":"b","type":"y","value":20}"#.to_string(),
        ];

        let results: Vec<Result<TestEvent, _>> = parser.parse_events(&events);
        assert_eq!(results.len(), 2);

        let first = results[0].as_ref().unwrap();
        assert_eq!(first.id, "a");
        assert_eq!(first.value, 10);
    }

    #[test]
    fn test_throughput_calculation() {
        let stats = SimdJsonStats::new();
        stats.bytes_parsed.store(1_000_000, Ordering::Relaxed);
        stats.parse_time_ns.store(1_000_000_000, Ordering::Relaxed); // 1 second
        stats.documents_parsed.store(10_000, Ordering::Relaxed);

        assert!((stats.throughput_mbps() - 1.0).abs() < 0.001);
        assert!((stats.docs_per_second() - 10_000.0).abs() < 1.0);
    }
}
