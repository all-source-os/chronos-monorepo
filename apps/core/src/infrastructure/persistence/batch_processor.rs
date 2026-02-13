//! High-throughput batch processing pipeline
//!
//! This module provides an optimized batch processing system that combines:
//! - SIMD-accelerated JSON parsing for fast deserialization
//! - Lock-free sharded queues for parallel ingestion
//! - Memory pooling for zero-allocation hot paths
//!
//! # Performance Target
//! - 1M+ events/sec sustained throughput
//! - Sub-millisecond p99 latency
//! - Linear scalability with CPU cores

use crate::{
    domain::entities::Event,
    error::Result,
    infrastructure::persistence::{
        lock_free::{LockFreeMetrics, ShardedEventQueue},
        simd_json::{SimdJsonParser, SimdJsonStats},
    },
};
use bumpalo::Bump;
use serde::Deserialize;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

/// Configuration for batch processor
#[derive(Debug, Clone)]
pub struct BatchProcessorConfig {
    /// Maximum batch size for processing
    pub max_batch_size: usize,
    /// Queue capacity for pending events
    pub queue_capacity: usize,
    /// Number of shards for the event queue
    pub shard_count: usize,
    /// Arena allocation pool size (bytes)
    pub arena_size: usize,
    /// Enable SIMD JSON parsing
    pub simd_enabled: bool,
}

impl Default for BatchProcessorConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 10_000,
            queue_capacity: 1_000_000,
            shard_count: 16,
            arena_size: 64 * 1024 * 1024, // 64MB
            simd_enabled: true,
        }
    }
}

impl BatchProcessorConfig {
    /// High-throughput configuration for production use
    pub fn high_throughput() -> Self {
        Self {
            max_batch_size: 50_000,
            queue_capacity: 10_000_000,
            shard_count: 32,
            arena_size: 256 * 1024 * 1024, // 256MB
            simd_enabled: true,
        }
    }

    /// Low-latency configuration optimized for quick responses
    pub fn low_latency() -> Self {
        Self {
            max_batch_size: 1_000,
            queue_capacity: 100_000,
            shard_count: 8,
            arena_size: 16 * 1024 * 1024, // 16MB
            simd_enabled: true,
        }
    }
}

/// Raw event data for batch parsing
#[derive(Debug, Clone, Deserialize)]
pub struct RawEventData {
    pub event_type: String,
    pub entity_id: String,
    #[serde(default = "default_stream")]
    pub stream_id: String,
    pub data: serde_json::Value,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn default_stream() -> String {
    "default".to_string()
}

/// Statistics for batch processing
#[derive(Debug, Clone)]
pub struct BatchProcessorStats {
    /// Total batches processed
    pub batches_processed: u64,
    /// Total events processed
    pub events_processed: u64,
    /// Total bytes parsed
    pub bytes_parsed: u64,
    /// Average batch size
    pub avg_batch_size: f64,
    /// Events per second
    pub events_per_sec: f64,
    /// Parse throughput in MB/s
    pub parse_throughput_mbps: f64,
    /// Current queue depth
    pub queue_depth: usize,
    /// Total processing time in nanoseconds
    pub total_time_ns: u64,
}

/// High-throughput batch event processor
///
/// Combines SIMD JSON parsing with lock-free queues for maximum throughput.
/// Designed to achieve 1M+ events/sec sustained ingestion.
pub struct BatchProcessor {
    config: BatchProcessorConfig,
    /// SIMD JSON parser
    json_parser: SimdJsonParser,
    /// Lock-free sharded queue for processed events
    event_queue: ShardedEventQueue,
    /// Lock-free metrics
    metrics: Arc<LockFreeMetrics>,
    /// Batch processing statistics
    batches_processed: AtomicU64,
    events_processed: AtomicU64,
    bytes_parsed: AtomicU64,
    total_time_ns: AtomicU64,
    /// Current arena pool index for round-robin
    arena_index: AtomicUsize,
}

impl BatchProcessor {
    /// Create a new batch processor with default configuration
    pub fn new() -> Self {
        Self::with_config(BatchProcessorConfig::default())
    }

    /// Create a new batch processor with custom configuration
    pub fn with_config(config: BatchProcessorConfig) -> Self {
        let event_queue = ShardedEventQueue::with_shards(config.queue_capacity, config.shard_count);

        Self {
            config,
            json_parser: SimdJsonParser::new(),
            event_queue,
            metrics: Arc::new(LockFreeMetrics::new()),
            batches_processed: AtomicU64::new(0),
            events_processed: AtomicU64::new(0),
            bytes_parsed: AtomicU64::new(0),
            total_time_ns: AtomicU64::new(0),
            arena_index: AtomicUsize::new(0),
        }
    }

    /// Process a batch of raw JSON event strings
    ///
    /// This is the main entry point for high-throughput ingestion.
    /// Uses SIMD-accelerated parsing and lock-free queuing.
    ///
    /// # Arguments
    /// * `json_events` - Vector of JSON strings to parse and process
    ///
    /// # Returns
    /// BatchResult with success/failure counts
    pub fn process_batch(&self, json_events: Vec<String>) -> BatchResult {
        let start = Instant::now();
        let batch_size = json_events.len();

        let mut success_count = 0;
        let mut failure_count = 0;
        let mut bytes_parsed = 0usize;

        for json_str in json_events {
            bytes_parsed += json_str.len();

            match self.parse_and_queue_event(json_str) {
                Ok(()) => {
                    success_count += 1;
                    self.metrics.record_ingest();
                }
                Err(_) => {
                    failure_count += 1;
                    self.metrics.record_error();
                }
            }
        }

        let duration = start.elapsed();
        self.record_batch_stats(batch_size, bytes_parsed, duration);

        BatchResult {
            success_count,
            failure_count,
            duration,
            events_per_sec: success_count as f64 / duration.as_secs_f64(),
        }
    }

    /// Process a batch of raw JSON bytes (more efficient - avoids string conversion)
    ///
    /// # Arguments
    /// * `json_bytes` - Vector of JSON byte slices to parse
    ///
    /// # Returns
    /// BatchResult with success/failure counts
    pub fn process_batch_bytes(&self, mut json_bytes: Vec<Vec<u8>>) -> BatchResult {
        let start = Instant::now();
        let batch_size = json_bytes.len();

        let mut success_count = 0;
        let mut failure_count = 0;
        let mut bytes_parsed = 0usize;

        for bytes in &mut json_bytes {
            bytes_parsed += bytes.len();

            match self.parse_and_queue_bytes(bytes) {
                Ok(()) => {
                    success_count += 1;
                    self.metrics.record_ingest();
                }
                Err(_) => {
                    failure_count += 1;
                    self.metrics.record_error();
                }
            }
        }

        let duration = start.elapsed();
        self.record_batch_stats(batch_size, bytes_parsed, duration);

        BatchResult {
            success_count,
            failure_count,
            duration,
            events_per_sec: success_count as f64 / duration.as_secs_f64(),
        }
    }

    /// Process pre-parsed events directly (fastest path)
    ///
    /// Use this when events are already parsed (e.g., from Arrow/Parquet).
    pub fn process_events(&self, events: Vec<Event>) -> BatchResult {
        let start = Instant::now();
        let batch_size = events.len();

        let success_count = self.event_queue.try_push_batch(events);
        let failure_count = batch_size - success_count;

        self.metrics.record_ingest_batch(success_count as u64);
        if failure_count > 0 {
            for _ in 0..failure_count {
                self.metrics.record_error();
            }
        }

        let duration = start.elapsed();
        self.batches_processed.fetch_add(1, Ordering::Relaxed);
        self.events_processed
            .fetch_add(success_count as u64, Ordering::Relaxed);
        self.total_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);

        BatchResult {
            success_count,
            failure_count,
            duration,
            events_per_sec: success_count as f64 / duration.as_secs_f64(),
        }
    }

    /// Parse and queue a single JSON event string
    fn parse_and_queue_event(&self, json_str: String) -> Result<()> {
        let raw: RawEventData = self.json_parser.parse_str(&json_str)?;

        let event = Event::from_strings(
            raw.event_type,
            raw.entity_id,
            raw.stream_id,
            raw.data,
            raw.metadata,
        )?;

        self.event_queue.try_push(event)
    }

    /// Parse and queue from bytes (SIMD path)
    fn parse_and_queue_bytes(&self, bytes: &mut [u8]) -> Result<()> {
        let raw: RawEventData = self.json_parser.parse(bytes)?;

        let event = Event::from_strings(
            raw.event_type,
            raw.entity_id,
            raw.stream_id,
            raw.data,
            raw.metadata,
        )?;

        self.event_queue.try_push(event)
    }

    /// Record batch statistics
    fn record_batch_stats(&self, batch_size: usize, bytes: usize, duration: Duration) {
        self.batches_processed.fetch_add(1, Ordering::Relaxed);
        self.events_processed
            .fetch_add(batch_size as u64, Ordering::Relaxed);
        self.bytes_parsed.fetch_add(bytes as u64, Ordering::Relaxed);
        self.total_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Get a batch of processed events from the queue
    ///
    /// # Arguments
    /// * `max_count` - Maximum number of events to retrieve
    pub fn get_batch(&self, max_count: usize) -> Vec<Event> {
        self.event_queue.try_pop_batch(max_count)
    }

    /// Get a single event from the queue
    pub fn get_event(&self) -> Option<Event> {
        self.event_queue.try_pop_any()
    }

    /// Get current queue depth
    pub fn queue_depth(&self) -> usize {
        self.event_queue.len()
    }

    /// Check if queue is empty
    pub fn is_queue_empty(&self) -> bool {
        self.event_queue.is_empty()
    }

    /// Get processing statistics
    pub fn stats(&self) -> BatchProcessorStats {
        let batches = self.batches_processed.load(Ordering::Relaxed);
        let events = self.events_processed.load(Ordering::Relaxed);
        let bytes = self.bytes_parsed.load(Ordering::Relaxed);
        let time_ns = self.total_time_ns.load(Ordering::Relaxed);

        let time_secs = time_ns as f64 / 1_000_000_000.0;
        let events_per_sec = if time_secs > 0.0 {
            events as f64 / time_secs
        } else {
            0.0
        };
        let throughput_mbps = if time_secs > 0.0 {
            (bytes as f64 / 1_000_000.0) / time_secs
        } else {
            0.0
        };

        BatchProcessorStats {
            batches_processed: batches,
            events_processed: events,
            bytes_parsed: bytes,
            avg_batch_size: if batches > 0 {
                events as f64 / batches as f64
            } else {
                0.0
            },
            events_per_sec,
            parse_throughput_mbps: throughput_mbps,
            queue_depth: self.event_queue.len(),
            total_time_ns: time_ns,
        }
    }

    /// Get JSON parser statistics
    pub fn parser_stats(&self) -> &SimdJsonStats {
        self.json_parser.stats()
    }

    /// Get metrics collector
    pub fn metrics(&self) -> Arc<LockFreeMetrics> {
        self.metrics.clone()
    }

    /// Get the event queue for direct access
    pub fn event_queue(&self) -> &ShardedEventQueue {
        &self.event_queue
    }

    /// Reset all statistics
    pub fn reset_stats(&self) {
        self.batches_processed.store(0, Ordering::Relaxed);
        self.events_processed.store(0, Ordering::Relaxed);
        self.bytes_parsed.store(0, Ordering::Relaxed);
        self.total_time_ns.store(0, Ordering::Relaxed);
        self.json_parser.reset_stats();
        self.metrics.reset();
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of batch processing operation
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Number of successfully processed events
    pub success_count: usize,
    /// Number of failed events
    pub failure_count: usize,
    /// Total processing duration
    pub duration: Duration,
    /// Events processed per second
    pub events_per_sec: f64,
}

impl BatchResult {
    /// Get total events (success + failure)
    pub fn total(&self) -> usize {
        self.success_count + self.failure_count
    }

    /// Get success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total();
        if total > 0 {
            self.success_count as f64 / total as f64
        } else {
            1.0
        }
    }
}

/// Arena-backed batch buffer for zero-allocation processing
///
/// Uses bumpalo for fast arena allocation during batch processing.
/// All allocations are freed together when the arena is reset.
pub struct ArenaBatchBuffer {
    arena: Bump,
    capacity: usize,
}

impl ArenaBatchBuffer {
    /// Create a new arena buffer with specified capacity
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            arena: Bump::with_capacity(capacity_bytes),
            capacity: capacity_bytes,
        }
    }

    /// Allocate a byte slice in the arena
    pub fn alloc_bytes(&self, data: &[u8]) -> &[u8] {
        self.arena.alloc_slice_copy(data)
    }

    /// Allocate a string in the arena
    pub fn alloc_str(&self, s: &str) -> &str {
        self.arena.alloc_str(s)
    }

    /// Get current arena allocation
    pub fn allocated(&self) -> usize {
        self.arena.allocated_bytes()
    }

    /// Reset the arena (frees all allocations)
    pub fn reset(&mut self) {
        self.arena.reset();
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_json(id: u32) -> String {
        json!({
            "event_type": "test.event",
            "entity_id": format!("entity-{}", id),
            "stream_id": "test-stream",
            "data": {"value": id}
        })
        .to_string()
    }

    #[test]
    fn test_create_batch_processor() {
        let processor = BatchProcessor::new();
        assert!(processor.is_queue_empty());
        assert_eq!(processor.queue_depth(), 0);
    }

    #[test]
    fn test_process_single_batch() {
        let processor = BatchProcessor::new();

        let events: Vec<String> = (0..100).map(create_test_json).collect();
        let result = processor.process_batch(events);

        assert_eq!(result.success_count, 100);
        assert_eq!(result.failure_count, 0);
        assert_eq!(processor.queue_depth(), 100);
    }

    #[test]
    fn test_process_batch_bytes() {
        let processor = BatchProcessor::new();

        let events: Vec<Vec<u8>> = (0..50).map(|i| create_test_json(i).into_bytes()).collect();
        let result = processor.process_batch_bytes(events);

        assert_eq!(result.success_count, 50);
        assert_eq!(result.failure_count, 0);
    }

    #[test]
    fn test_get_batch() {
        let processor = BatchProcessor::new();

        let events: Vec<String> = (0..100).map(create_test_json).collect();
        processor.process_batch(events);

        let batch = processor.get_batch(30);
        assert_eq!(batch.len(), 30);
        assert_eq!(processor.queue_depth(), 70);
    }

    #[test]
    fn test_stats() {
        let processor = BatchProcessor::new();

        let events: Vec<String> = (0..100).map(create_test_json).collect();
        processor.process_batch(events);

        let stats = processor.stats();
        assert_eq!(stats.batches_processed, 1);
        assert_eq!(stats.events_processed, 100);
        assert!(stats.events_per_sec > 0.0);
    }

    #[test]
    fn test_invalid_json() {
        let processor = BatchProcessor::new();

        let events = vec![
            create_test_json(1),
            "invalid json".to_string(),
            create_test_json(3),
        ];
        let result = processor.process_batch(events);

        assert_eq!(result.success_count, 2);
        assert_eq!(result.failure_count, 1);
    }

    #[test]
    fn test_batch_result_metrics() {
        let result = BatchResult {
            success_count: 90,
            failure_count: 10,
            duration: Duration::from_millis(100),
            events_per_sec: 900.0,
        };

        assert_eq!(result.total(), 100);
        assert!((result.success_rate() - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_arena_batch_buffer() {
        let mut buffer = ArenaBatchBuffer::new(1024);

        let s1 = buffer.alloc_str("hello");
        let s2 = buffer.alloc_str("world");

        assert_eq!(s1, "hello");
        assert_eq!(s2, "world");
        let allocated_before = buffer.allocated();
        assert!(allocated_before > 0);

        // Reset makes memory available for reuse (but allocated_bytes may not change)
        buffer.reset();

        // After reset, new allocations should reuse the memory
        let s3 = buffer.alloc_str("test");
        assert_eq!(s3, "test");

        // Verify the buffer is functional after reset
        assert!(buffer.capacity() >= 1024);
    }

    #[test]
    fn test_config_presets() {
        let default = BatchProcessorConfig::default();
        let high_throughput = BatchProcessorConfig::high_throughput();
        let low_latency = BatchProcessorConfig::low_latency();

        assert!(high_throughput.queue_capacity > default.queue_capacity);
        assert!(low_latency.max_batch_size < default.max_batch_size);
    }

    #[test]
    fn test_concurrent_processing() {
        let processor = Arc::new(BatchProcessor::new());

        std::thread::scope(|s| {
            // Multiple producer threads
            for t in 0..4 {
                let proc = processor.clone();
                s.spawn(move || {
                    let events: Vec<String> =
                        (0..100).map(|i| create_test_json(t * 100 + i)).collect();
                    proc.process_batch(events);
                });
            }
        });

        // All 400 events should be in the queue
        assert_eq!(processor.queue_depth(), 400);
    }

    #[test]
    fn test_process_events_direct() {
        let processor = BatchProcessor::new();

        let events: Vec<Event> = (0..50)
            .map(|i| {
                Event::from_strings(
                    "test.event".to_string(),
                    format!("entity-{}", i),
                    "test-stream".to_string(),
                    json!({"value": i}),
                    None,
                )
                .unwrap()
            })
            .collect();

        let result = processor.process_events(events);
        assert_eq!(result.success_count, 50);
        assert_eq!(processor.queue_depth(), 50);
    }
}
