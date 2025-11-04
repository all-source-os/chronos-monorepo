use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Lock-free metrics collector for high-performance monitoring
///
/// # Design Pattern
/// Uses atomic operations for lock-free metric updates. This eliminates
/// contention in hot paths where metrics are recorded frequently.
///
/// # Benefits
/// - **Zero Lock Contention**: All updates use atomic operations
/// - **Cache-Friendly**: Minimal memory footprint
/// - **High Throughput**: ~5-10ns per metric update
/// - **Thread-Safe**: Safe for concurrent access without locks
///
/// # Trade-offs
/// - Cannot guarantee atomic snapshots of all metrics together
/// - Slight approximation in aggregate calculations under high concurrency
/// - Memory ordering: Uses `Relaxed` for performance (acceptable for metrics)
///
/// # Example
/// ```ignore
/// let metrics = LockFreeMetrics::new();
///
/// // Record events (multiple threads can do this concurrently)
/// metrics.record_ingest();
/// metrics.record_query(Duration::from_micros(150));
///
/// // Read aggregated metrics
/// println!("Throughput: {}/sec", metrics.throughput_per_sec());
/// println!("Avg latency: {:?}", metrics.avg_query_latency());
/// ```
pub struct LockFreeMetrics {
    /// Total events ingested
    events_ingested: AtomicU64,

    /// Total events queried
    events_queried: AtomicU64,

    /// Sum of all query latencies (nanoseconds)
    total_latency_ns: AtomicU64,

    /// Minimum query latency (nanoseconds)
    min_latency_ns: AtomicU64,

    /// Maximum query latency (nanoseconds)
    max_latency_ns: AtomicU64,

    /// Number of errors encountered
    errors: AtomicU64,

    /// Timestamp when metrics collection started
    started_at: Instant,
}

impl LockFreeMetrics {
    /// Create new lock-free metrics collector
    pub fn new() -> Self {
        Self {
            events_ingested: AtomicU64::new(0),
            events_queried: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            min_latency_ns: AtomicU64::new(u64::MAX),
            max_latency_ns: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            started_at: Instant::now(),
        }
    }

    /// Record an event ingestion
    ///
    /// # Performance
    /// - Lock-free atomic increment (~5-10ns)
    /// - Uses Relaxed ordering (metrics don't need strict ordering)
    #[inline]
    pub fn record_ingest(&self) {
        self.events_ingested.fetch_add(1, Ordering::Relaxed);
    }

    /// Record multiple event ingestions at once
    ///
    /// More efficient than calling `record_ingest()` multiple times.
    #[inline]
    pub fn record_ingest_batch(&self, count: u64) {
        self.events_ingested.fetch_add(count, Ordering::Relaxed);
    }

    /// Record a query with its latency
    ///
    /// # Performance
    /// - Lock-free atomic operations (~10-15ns total)
    /// - Min/max tracking uses compare-and-swap
    pub fn record_query(&self, latency: Duration) {
        let latency_ns = latency.as_nanos() as u64;

        self.events_queried.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency_ns, Ordering::Relaxed);

        // Update minimum latency (compare-and-swap loop)
        let mut current_min = self.min_latency_ns.load(Ordering::Relaxed);
        while latency_ns < current_min {
            match self.min_latency_ns.compare_exchange_weak(
                current_min,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_min = actual,
            }
        }

        // Update maximum latency (compare-and-swap loop)
        let mut current_max = self.max_latency_ns.load(Ordering::Relaxed);
        while latency_ns > current_max {
            match self.max_latency_ns.compare_exchange_weak(
                current_max,
                latency_ns,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    /// Record an error
    #[inline]
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total events ingested
    pub fn events_ingested(&self) -> u64 {
        self.events_ingested.load(Ordering::Relaxed)
    }

    /// Get total events queried
    pub fn events_queried(&self) -> u64 {
        self.events_queried.load(Ordering::Relaxed)
    }

    /// Get total errors
    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// Calculate ingestion throughput (events/second)
    pub fn throughput_per_sec(&self) -> f64 {
        let elapsed = self.started_at.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.events_ingested.load(Ordering::Relaxed) as f64 / elapsed
    }

    /// Calculate average query latency
    pub fn avg_query_latency(&self) -> Option<Duration> {
        let total = self.total_latency_ns.load(Ordering::Relaxed);
        let count = self.events_queried.load(Ordering::Relaxed);

        if count == 0 {
            None
        } else {
            Some(Duration::from_nanos(total / count))
        }
    }

    /// Get minimum query latency
    pub fn min_query_latency(&self) -> Option<Duration> {
        let min = self.min_latency_ns.load(Ordering::Relaxed);
        if min == u64::MAX {
            None
        } else {
            Some(Duration::from_nanos(min))
        }
    }

    /// Get maximum query latency
    pub fn max_query_latency(&self) -> Option<Duration> {
        let max = self.max_latency_ns.load(Ordering::Relaxed);
        if max == 0 {
            None
        } else {
            Some(Duration::from_nanos(max))
        }
    }

    /// Get time since metrics collection started
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Reset all metrics
    ///
    /// Note: Not atomic across all metrics. In concurrent scenarios,
    /// some updates might be recorded during reset.
    pub fn reset(&self) {
        self.events_ingested.store(0, Ordering::Relaxed);
        self.events_queried.store(0, Ordering::Relaxed);
        self.total_latency_ns.store(0, Ordering::Relaxed);
        self.min_latency_ns.store(u64::MAX, Ordering::Relaxed);
        self.max_latency_ns.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
    }

    /// Get snapshot of all metrics
    ///
    /// Note: Not atomic - values may be slightly inconsistent under
    /// high concurrent load. Acceptable for monitoring purposes.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            events_ingested: self.events_ingested(),
            events_queried: self.events_queried(),
            errors: self.errors(),
            avg_query_latency: self.avg_query_latency(),
            min_query_latency: self.min_query_latency(),
            max_query_latency: self.max_query_latency(),
            throughput_per_sec: self.throughput_per_sec(),
            uptime: self.uptime(),
        }
    }
}

impl Default for LockFreeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub events_ingested: u64,
    pub events_queried: u64,
    pub errors: u64,
    pub avg_query_latency: Option<Duration>,
    pub min_query_latency: Option<Duration>,
    pub max_query_latency: Option<Duration>,
    pub throughput_per_sec: f64,
    pub uptime: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_create_metrics() {
        let metrics = LockFreeMetrics::new();
        assert_eq!(metrics.events_ingested(), 0);
        assert_eq!(metrics.events_queried(), 0);
        assert_eq!(metrics.errors(), 0);
        assert_eq!(metrics.avg_query_latency(), None);
    }

    #[test]
    fn test_record_ingest() {
        let metrics = LockFreeMetrics::new();

        metrics.record_ingest();
        metrics.record_ingest();
        metrics.record_ingest();

        assert_eq!(metrics.events_ingested(), 3);
    }

    #[test]
    fn test_record_ingest_batch() {
        let metrics = LockFreeMetrics::new();

        metrics.record_ingest_batch(100);
        metrics.record_ingest_batch(50);

        assert_eq!(metrics.events_ingested(), 150);
    }

    #[test]
    fn test_record_query() {
        let metrics = LockFreeMetrics::new();

        metrics.record_query(Duration::from_micros(100));
        metrics.record_query(Duration::from_micros(200));
        metrics.record_query(Duration::from_micros(150));

        assert_eq!(metrics.events_queried(), 3);
        assert_eq!(
            metrics.avg_query_latency(),
            Some(Duration::from_micros(150))
        );
        assert_eq!(
            metrics.min_query_latency(),
            Some(Duration::from_micros(100))
        );
        assert_eq!(
            metrics.max_query_latency(),
            Some(Duration::from_micros(200))
        );
    }

    #[test]
    fn test_record_error() {
        let metrics = LockFreeMetrics::new();

        metrics.record_error();
        metrics.record_error();

        assert_eq!(metrics.errors(), 2);
    }

    #[test]
    fn test_throughput_calculation() {
        let metrics = LockFreeMetrics::new();

        // Sleep to ensure non-zero elapsed time
        thread::sleep(Duration::from_millis(10));

        metrics.record_ingest_batch(1000);

        let throughput = metrics.throughput_per_sec();
        assert!(throughput > 0.0);
        assert!(throughput < 1_000_000.0); // Sanity check
    }

    #[test]
    fn test_reset() {
        let metrics = LockFreeMetrics::new();

        metrics.record_ingest_batch(100);
        metrics.record_query(Duration::from_micros(100));
        metrics.record_error();

        assert_eq!(metrics.events_ingested(), 100);
        assert_eq!(metrics.events_queried(), 1);
        assert_eq!(metrics.errors(), 1);

        metrics.reset();

        assert_eq!(metrics.events_ingested(), 0);
        assert_eq!(metrics.events_queried(), 0);
        assert_eq!(metrics.errors(), 0);
        assert_eq!(metrics.avg_query_latency(), None);
    }

    #[test]
    fn test_snapshot() {
        let metrics = LockFreeMetrics::new();

        thread::sleep(Duration::from_millis(10));

        metrics.record_ingest_batch(50);
        metrics.record_query(Duration::from_micros(100));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.events_ingested, 50);
        assert_eq!(snapshot.events_queried, 1);
        assert!(snapshot.throughput_per_sec > 0.0);
        assert!(snapshot.uptime.as_millis() >= 10);
    }

    #[test]
    fn test_concurrent_ingests() {
        let metrics = Arc::new(LockFreeMetrics::new());
        let mut handles = vec![];

        // Spawn 10 threads, each ingesting 100 events
        for _ in 0..10 {
            let metrics_clone = metrics.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    metrics_clone.record_ingest();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(metrics.events_ingested(), 1000);
    }

    #[test]
    fn test_concurrent_queries() {
        let metrics = Arc::new(LockFreeMetrics::new());
        let mut handles = vec![];

        // Spawn 8 threads, each recording 50 queries
        for _ in 0..8 {
            let metrics_clone = metrics.clone();
            let handle = thread::spawn(move || {
                for i in 0..50 {
                    metrics_clone.record_query(Duration::from_micros(100 + i));
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(metrics.events_queried(), 400);
        assert!(metrics.avg_query_latency().is_some());
        assert!(metrics.min_query_latency().is_some());
        assert!(metrics.max_query_latency().is_some());
    }

    #[test]
    fn test_mixed_concurrent_operations() {
        let metrics = Arc::new(LockFreeMetrics::new());
        let mut handles = vec![];

        // Ingest thread
        let metrics_clone = metrics.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                metrics_clone.record_ingest();
            }
        }));

        // Query thread
        let metrics_clone = metrics.clone();
        handles.push(thread::spawn(move || {
            for i in 0..500 {
                metrics_clone.record_query(Duration::from_micros(100 + i));
            }
        }));

        // Error thread
        let metrics_clone = metrics.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                metrics_clone.record_error();
            }
        }));

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(metrics.events_ingested(), 1000);
        assert_eq!(metrics.events_queried(), 500);
        assert_eq!(metrics.errors(), 100);
    }
}
