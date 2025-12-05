//! Performance validation tests for high-throughput event processing
//!
//! These tests validate that the performance optimizations (SIMD JSON parsing,
//! lock-free queues, batch processing, memory pooling) achieve acceptable
//! throughput. Note: Tests run in debug mode with ~10-20x slower performance.
//!
//! For production benchmarks, run with: `cargo test --release`
//!
//! Expected performance in RELEASE mode:
//! - SIMD JSON parsing: 500K+ events/sec
//! - Lock-free queue: 1M+ ops/sec
//! - Batch processor: 200K+ events/sec
//! - Arena allocations: 10M+ allocs/sec

#[cfg(test)]
mod tests {
    use crate::domain::entities::Event;
    use crate::infrastructure::persistence::{
        arena_pool::{get_arena, reset_stats as reset_arena_stats, arena_stats},
        batch_processor::{BatchProcessor, BatchProcessorConfig},
        lock_free::{LockFreeEventQueue, LockFreeMetrics, ShardedEventQueue},
        simd_json::SimdJsonParser,
    };
    use serde_json::json;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn create_test_json(id: u32) -> String {
        json!({
            "event_type": "perf.test",
            "entity_id": format!("entity-{}", id % 1000),
            "stream_id": "test-stream",
            "data": {"value": id, "name": "test", "nested": {"a": 1, "b": 2}}
        })
        .to_string()
    }

    fn create_test_event(id: u32) -> Event {
        Event::from_strings(
            "perf.test".to_string(),
            format!("entity-{}", id % 1000),
            "test-stream".to_string(),
            json!({"value": id}),
            None,
        )
        .unwrap()
    }

    /// Test SIMD JSON parsing throughput
    #[test]
    fn test_simd_json_throughput() {
        let parser = SimdJsonParser::new();

        // Prepare test data
        let json_strings: Vec<String> = (0..100_000).map(create_test_json).collect();
        let total_bytes: usize = json_strings.iter().map(|s| s.len()).sum();

        let start = Instant::now();

        for json_str in &json_strings {
            let mut bytes = json_str.as_bytes().to_vec();
            let _: serde_json::Value = parser.parse(&mut bytes).unwrap();
        }

        let duration = start.elapsed();
        let events_per_sec = 100_000.0 / duration.as_secs_f64();
        let throughput_mbps = (total_bytes as f64 / 1_000_000.0) / duration.as_secs_f64();

        println!("\n=== SIMD JSON Parsing Performance ===");
        println!("Events: 100,000");
        println!("Duration: {:?}", duration);
        println!("Events/sec: {:.0}", events_per_sec);
        println!("Throughput: {:.2} MB/s", throughput_mbps);
        println!("Parser stats: {:?}", parser.stats());

        // Target: At least 50K events/sec in debug mode (500K+ in release)
        assert!(
            events_per_sec > 50_000.0,
            "SIMD JSON parsing too slow: {:.0} events/sec (expected >50K in debug)",
            events_per_sec
        );
    }

    /// Test lock-free queue throughput
    #[test]
    fn test_lock_free_queue_throughput() {
        let queue = LockFreeEventQueue::new(200_000);

        // Single-threaded push performance
        let events: Vec<Event> = (0..100_000).map(create_test_event).collect();

        let start = Instant::now();
        for event in events {
            queue.try_push(event).unwrap();
        }
        let push_duration = start.elapsed();

        // Pop all events
        let start = Instant::now();
        let mut pop_count = 0;
        while queue.try_pop().is_some() {
            pop_count += 1;
        }
        let pop_duration = start.elapsed();

        let push_rate = 100_000.0 / push_duration.as_secs_f64();
        let pop_rate = pop_count as f64 / pop_duration.as_secs_f64();

        println!("\n=== Lock-Free Queue Performance ===");
        println!("Events: 100,000");
        println!("Push duration: {:?}", push_duration);
        println!("Pop duration: {:?}", pop_duration);
        println!("Push rate: {:.0} events/sec", push_rate);
        println!("Pop rate: {:.0} events/sec", pop_rate);

        // Target: At least 1M operations/sec in debug mode (queue ops are mostly lock-free)
        assert!(
            push_rate > 1_000_000.0,
            "Push rate too slow: {:.0} ops/sec (expected >1M in debug)",
            push_rate
        );
        assert!(
            pop_rate > 500_000.0,
            "Pop rate too slow: {:.0} ops/sec (expected >500K in debug)",
            pop_rate
        );
    }

    /// Test sharded queue throughput with multiple threads
    #[test]
    fn test_sharded_queue_concurrent_throughput() {
        let queue = Arc::new(ShardedEventQueue::new(1_000_000));
        let events_per_thread = 25_000;
        let thread_count = 4;
        let total_events = events_per_thread * thread_count;

        let start = Instant::now();

        std::thread::scope(|s| {
            for t in 0..thread_count {
                let queue_ref = queue.clone();
                s.spawn(move || {
                    for i in 0..events_per_thread {
                        let event = create_test_event((t * events_per_thread + i) as u32);
                        let _ = queue_ref.try_push(event);
                    }
                });
            }
        });

        let push_duration = start.elapsed();
        let push_rate = total_events as f64 / push_duration.as_secs_f64();

        // Pop with multiple threads
        let start = Instant::now();

        std::thread::scope(|s| {
            for _ in 0..thread_count {
                let queue_ref = queue.clone();
                s.spawn(move || {
                    let mut count = 0;
                    while count < events_per_thread {
                        if queue_ref.try_pop_any().is_some() {
                            count += 1;
                        }
                    }
                });
            }
        });

        let pop_duration = start.elapsed();
        let pop_rate = total_events as f64 / pop_duration.as_secs_f64();

        println!("\n=== Sharded Queue Concurrent Performance ===");
        println!("Total events: {}", total_events);
        println!("Threads: {}", thread_count);
        println!("Push duration: {:?}", push_duration);
        println!("Pop duration: {:?}", pop_duration);
        println!("Push rate: {:.0} events/sec", push_rate);
        println!("Pop rate: {:.0} events/sec", pop_rate);
        println!("Queue stats: {:?}", queue.stats());

        // Target: At least 400K operations/sec with 4 threads in debug mode (2M+ in release)
        assert!(
            push_rate > 400_000.0,
            "Concurrent push rate too slow: {:.0} ops/sec (expected >400K in debug)",
            push_rate
        );
    }

    /// Test batch processor end-to-end throughput
    #[test]
    fn test_batch_processor_throughput() {
        let config = BatchProcessorConfig {
            max_batch_size: 10_000,
            queue_capacity: 500_000,
            shard_count: 16,
            arena_size: 64 * 1024 * 1024,
            simd_enabled: true,
        };

        let processor = BatchProcessor::with_config(config);

        // Prepare test data
        let batch_size = 10_000;
        let batch_count = 10;
        let total_events = batch_size * batch_count;

        let batches: Vec<Vec<String>> = (0..batch_count)
            .map(|b| {
                (0..batch_size)
                    .map(|i| create_test_json((b * batch_size + i) as u32))
                    .collect()
            })
            .collect();

        let start = Instant::now();

        for batch in batches {
            let result = processor.process_batch(batch);
            assert_eq!(result.failure_count, 0);
        }

        let duration = start.elapsed();
        let events_per_sec = total_events as f64 / duration.as_secs_f64();

        let stats = processor.stats();
        println!("\n=== Batch Processor End-to-End Performance ===");
        println!("Total events: {}", total_events);
        println!("Batch size: {}", batch_size);
        println!("Duration: {:?}", duration);
        println!("Events/sec: {:.0}", events_per_sec);
        println!("Stats: {:?}", stats);

        // Target: At least 50K events/sec in debug mode (200K+ in release)
        // This includes JSON parsing + validation + queuing
        assert!(
            events_per_sec > 50_000.0,
            "Batch processor too slow: {:.0} events/sec (expected >50K in debug)",
            events_per_sec
        );
    }

    /// Test arena pool performance
    #[test]
    fn test_arena_pool_allocation_speed() {
        reset_arena_stats();

        let iterations = 10_000;
        let allocs_per_iter = 100;

        let start = Instant::now();

        for _ in 0..iterations {
            let arena = get_arena();
            for i in 0..allocs_per_iter {
                let _ = arena.alloc_str(&format!("test-string-{}", i));
            }
            // Arena returned to pool on drop
        }

        let duration = start.elapsed();
        let total_allocs = iterations * allocs_per_iter;
        let allocs_per_sec = total_allocs as f64 / duration.as_secs_f64();

        let stats = arena_stats();
        println!("\n=== Arena Pool Allocation Performance ===");
        println!("Total allocations: {}", total_allocs);
        println!("Duration: {:?}", duration);
        println!("Allocations/sec: {:.0}", allocs_per_sec);
        println!("Arenas created: {}", stats.arenas_created);
        println!("Arenas recycled: {}", stats.arenas_recycled);
        println!("Recycle rate: {:.1}%", stats.recycle_rate * 100.0);

        // Target: At least 3M allocations/sec in debug mode (10M+ in release)
        assert!(
            allocs_per_sec > 3_000_000.0,
            "Arena allocation too slow: {:.0} allocs/sec (expected >3M in debug)",
            allocs_per_sec
        );
    }

    /// Integration test: Full pipeline throughput
    #[test]
    fn test_full_pipeline_throughput() {
        let processor = Arc::new(BatchProcessor::with_config(
            BatchProcessorConfig::high_throughput(),
        ));
        let metrics = LockFreeMetrics::new();

        let total_events = 100_000;
        let batch_size = 5_000;
        let thread_count = 4;
        let events_per_thread = total_events / thread_count;

        let start = Instant::now();

        std::thread::scope(|s| {
            // Producer threads
            for t in 0..thread_count {
                let proc = processor.clone();
                s.spawn(move || {
                    let mut remaining = events_per_thread;
                    let mut batch_id = 0;
                    while remaining > 0 {
                        let count = remaining.min(batch_size);
                        let batch: Vec<String> = (0..count)
                            .map(|i| {
                                create_test_json(
                                    (t * events_per_thread + batch_id * batch_size + i) as u32,
                                )
                            })
                            .collect();
                        proc.process_batch(batch);
                        remaining -= count;
                        batch_id += 1;
                    }
                });
            }
        });

        let duration = start.elapsed();
        let events_per_sec = total_events as f64 / duration.as_secs_f64();

        let stats = processor.stats();
        println!("\n=== Full Pipeline Performance (Concurrent) ===");
        println!("Total events: {}", total_events);
        println!("Threads: {}", thread_count);
        println!("Duration: {:?}", duration);
        println!("Events/sec: {:.0}", events_per_sec);
        println!("Avg batch size: {:.1}", stats.avg_batch_size);
        println!("Queue depth: {}", stats.queue_depth);

        // Target: At least 100K events/sec with 4 threads in debug mode (300K+ in release)
        assert!(
            events_per_sec > 100_000.0,
            "Full pipeline too slow: {:.0} events/sec (expected >100K in debug)",
            events_per_sec
        );
    }

    /// Test sustained throughput over time
    #[test]
    fn test_sustained_throughput() {
        let processor = BatchProcessor::new();

        let batch_size = 1_000;
        let duration_target = Duration::from_secs(2);
        let mut total_events = 0;
        let mut total_duration = Duration::ZERO;

        let start = Instant::now();
        while start.elapsed() < duration_target {
            let batch: Vec<String> = (0..batch_size)
                .map(|i| create_test_json((total_events + i) as u32))
                .collect();

            let batch_start = Instant::now();
            let result = processor.process_batch(batch);
            total_duration += batch_start.elapsed();

            total_events += result.success_count;
        }

        let events_per_sec = total_events as f64 / total_duration.as_secs_f64();

        println!("\n=== Sustained Throughput Test ===");
        println!("Test duration: {:?}", start.elapsed());
        println!("Processing time: {:?}", total_duration);
        println!("Total events: {}", total_events);
        println!("Events/sec: {:.0}", events_per_sec);

        // Should maintain consistent throughput over time (50K+ in debug, 200K+ in release)
        assert!(
            events_per_sec > 50_000.0,
            "Sustained throughput too low: {:.0} events/sec (expected >50K in debug)",
            events_per_sec
        );
    }
}
