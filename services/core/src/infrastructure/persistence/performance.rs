use std::time::{Duration, Instant};

/// Performance optimization utilities
///
/// Inspired by SierraDB's focus on high-throughput operations.
///
/// # Design Principles
/// - Batch operations for reduced overhead
/// - Minimal allocations
/// - Zero-copy where possible
/// - Lock-free when feasible

/// Batch writer for high-throughput ingestion
///
/// Accumulates items in a buffer and flushes when:
/// - Buffer reaches capacity
/// - Time threshold exceeded
/// - Explicit flush called
///
/// # Example
/// ```ignore
/// let mut writer = BatchWriter::new(100, Duration::from_millis(100));
/// writer.add(item)?;
/// if writer.should_flush() {
///     writer.flush()?;
/// }
/// ```
pub struct BatchWriter<T> {
    buffer: Vec<T>,
    capacity: usize,
    last_flush: Instant,
    flush_interval: Duration,
}

impl<T> BatchWriter<T> {
    /// Create new batch writer
    ///
    /// # Arguments
    /// - `capacity`: Maximum items before auto-flush
    /// - `flush_interval`: Maximum time between flushes
    pub fn new(capacity: usize, flush_interval: Duration) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            last_flush: Instant::now(),
            flush_interval,
        }
    }

    /// Add item to buffer
    pub fn add(&mut self, item: T) {
        self.buffer.push(item);
    }

    /// Check if buffer should be flushed
    pub fn should_flush(&self) -> bool {
        self.buffer.len() >= self.capacity || self.last_flush.elapsed() >= self.flush_interval
    }

    /// Get current buffer size
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Flush buffer and return items
    pub fn flush(&mut self) -> Vec<T> {
        self.last_flush = Instant::now();
        std::mem::take(&mut self.buffer)
    }

    /// Get time since last flush
    pub fn time_since_flush(&self) -> Duration {
        self.last_flush.elapsed()
    }
}

/// Performance metrics tracker
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operations: u64,
    pub total_duration: Duration,
    pub min_duration: Option<Duration>,
    pub max_duration: Option<Duration>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            operations: 0,
            total_duration: Duration::ZERO,
            min_duration: None,
            max_duration: None,
        }
    }

    pub fn record(&mut self, duration: Duration) {
        self.operations += 1;
        self.total_duration += duration;

        self.min_duration = Some(match self.min_duration {
            Some(min) => min.min(duration),
            None => duration,
        });

        self.max_duration = Some(match self.max_duration {
            Some(max) => max.max(duration),
            None => duration,
        });
    }

    pub fn avg_duration(&self) -> Option<Duration> {
        if self.operations == 0 {
            None
        } else {
            Some(self.total_duration / self.operations as u32)
        }
    }

    pub fn throughput_per_sec(&self) -> f64 {
        if self.total_duration.as_secs_f64() == 0.0 {
            0.0
        } else {
            self.operations as f64 / self.total_duration.as_secs_f64()
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory pool for reducing allocations
///
/// Pre-allocates buffers and reuses them to avoid allocation overhead.
///
/// # SierraDB Pattern
/// - Reduces GC pressure
/// - Improves throughput for high-frequency operations
/// - Thread-local pools avoid contention
pub struct MemoryPool<T> {
    pool: Vec<Vec<T>>,
    capacity: usize,
    max_pool_size: usize,
}

impl<T> MemoryPool<T> {
    pub fn new(capacity: usize, max_pool_size: usize) -> Self {
        Self {
            pool: Vec::new(),
            capacity,
            max_pool_size,
        }
    }

    /// Get a buffer from the pool or create new one
    pub fn get(&mut self) -> Vec<T> {
        self.pool.pop().unwrap_or_else(|| Vec::with_capacity(self.capacity))
    }

    /// Return buffer to pool for reuse
    pub fn put(&mut self, mut buffer: Vec<T>) {
        if self.pool.len() < self.max_pool_size {
            buffer.clear();
            self.pool.push(buffer);
        }
    }

    /// Get current pool size
    pub fn pool_size(&self) -> usize {
        self.pool.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_writer_capacity() {
        let mut writer = BatchWriter::new(3, Duration::from_secs(10));

        writer.add(1);
        writer.add(2);
        assert!(!writer.should_flush());

        writer.add(3);
        assert!(writer.should_flush());

        let items = writer.flush();
        assert_eq!(items, vec![1, 2, 3]);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_batch_writer_time_threshold() {
        let mut writer = BatchWriter::new(100, Duration::from_millis(1));

        writer.add(1);
        std::thread::sleep(Duration::from_millis(2));

        assert!(writer.should_flush());
    }

    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new();

        metrics.record(Duration::from_millis(10));
        metrics.record(Duration::from_millis(20));
        metrics.record(Duration::from_millis(30));

        assert_eq!(metrics.operations, 3);
        assert_eq!(metrics.avg_duration(), Some(Duration::from_millis(20)));
        assert_eq!(metrics.min_duration, Some(Duration::from_millis(10)));
        assert_eq!(metrics.max_duration, Some(Duration::from_millis(30)));
    }

    #[test]
    fn test_performance_metrics_throughput() {
        let mut metrics = PerformanceMetrics::new();

        for _ in 0..100 {
            metrics.record(Duration::from_millis(10));
        }

        let throughput = metrics.throughput_per_sec();
        assert!(throughput > 90.0 && throughput < 110.0); // ~100 ops/sec
    }

    #[test]
    fn test_memory_pool() {
        let mut pool: MemoryPool<i32> = MemoryPool::new(10, 5);

        let buf1 = pool.get();
        assert_eq!(buf1.capacity(), 10);

        let mut buf2 = pool.get();
        buf2.push(1);
        buf2.push(2);

        pool.put(buf2);
        assert_eq!(pool.pool_size(), 1);

        let buf3 = pool.get();
        assert_eq!(buf3.len(), 0); // Should be cleared
        assert_eq!(buf3.capacity(), 10);
    }

    #[test]
    fn test_memory_pool_max_size() {
        let mut pool: MemoryPool<i32> = MemoryPool::new(10, 2);

        pool.put(vec![]);
        pool.put(vec![]);
        pool.put(vec![]); // Should be dropped

        assert_eq!(pool.pool_size(), 2);
    }

    #[test]
    fn test_batch_writer_len() {
        let mut writer = BatchWriter::new(10, Duration::from_secs(10));

        assert_eq!(writer.len(), 0);
        writer.add(1);
        assert_eq!(writer.len(), 1);
        writer.add(2);
        assert_eq!(writer.len(), 2);

        writer.flush();
        assert_eq!(writer.len(), 0);
    }
}
