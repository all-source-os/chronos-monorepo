//! Performance benchmarks for AllSource event store
//!
//! Run with: cargo run --release -p allsource-performance
//!
//! These benchmarks validate that the performance optimizations achieve
//! acceptable throughput. Must be run in release mode for accurate results.

use allsource_core::domain::entities::Event;
use allsource_core::infrastructure::persistence::{
    arena_pool::{arena_stats, get_arena, reset_stats as reset_arena_stats},
    batch_processor::{BatchProcessor, BatchProcessorConfig},
    lock_free::{LockFreeEventQueue, ShardedEventQueue},
    simd_filter::{FilterPredicate, SimdEventFilter},
    simd_json::SimdJsonParser,
};
use chrono::Utc;
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

fn bench_simd_json() {
    println!("\n=== SIMD JSON Parsing Performance ===");
    let parser = SimdJsonParser::new();

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

    println!("Events: 100,000");
    println!("Duration: {:?}", duration);
    println!("Events/sec: {:.0}", events_per_sec);
    println!("Throughput: {:.2} MB/s", throughput_mbps);
    println!("Parser stats: {:?}", parser.stats());

    let target = 500_000.0;
    if events_per_sec > target {
        println!("PASS: {:.0} > {:.0}", events_per_sec, target);
    } else {
        println!("WARN: {:.0} < {:.0} (target)", events_per_sec, target);
    }
}

fn bench_lock_free_queue() {
    println!("\n=== Lock-Free Queue Performance ===");
    let queue = LockFreeEventQueue::new(200_000);

    let events: Vec<Event> = (0..100_000).map(create_test_event).collect();

    let start = Instant::now();
    for event in events {
        queue.try_push(event).unwrap();
    }
    let push_duration = start.elapsed();

    let start = Instant::now();
    let mut pop_count = 0;
    while queue.try_pop().is_some() {
        pop_count += 1;
    }
    let pop_duration = start.elapsed();

    let push_rate = 100_000.0 / push_duration.as_secs_f64();
    let pop_rate = pop_count as f64 / pop_duration.as_secs_f64();

    println!("Events: 100,000");
    println!("Push duration: {:?}", push_duration);
    println!("Pop duration: {:?}", pop_duration);
    println!("Push rate: {:.0} events/sec", push_rate);
    println!("Pop rate: {:.0} events/sec", pop_rate);

    let push_target = 1_000_000.0;
    if push_rate > push_target {
        println!("PASS push: {:.0} > {:.0}", push_rate, push_target);
    } else {
        println!("WARN push: {:.0} < {:.0} (target)", push_rate, push_target);
    }
}

fn bench_sharded_queue() {
    println!("\n=== Sharded Queue Concurrent Performance ===");
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

    println!("Total events: {}", total_events);
    println!("Threads: {}", thread_count);
    println!("Push duration: {:?}", push_duration);
    println!("Push rate: {:.0} events/sec", push_rate);
    println!("Queue stats: {:?}", queue.stats());

    let target = 2_000_000.0;
    if push_rate > target {
        println!("PASS: {:.0} > {:.0}", push_rate, target);
    } else {
        println!("WARN: {:.0} < {:.0} (target)", push_rate, target);
    }
}

fn bench_batch_processor() {
    println!("\n=== Batch Processor Performance ===");
    let config = BatchProcessorConfig {
        max_batch_size: 10_000,
        queue_capacity: 500_000,
        shard_count: 16,
        arena_size: 64 * 1024 * 1024,
        simd_enabled: true,
    };

    let processor = BatchProcessor::with_config(config);

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

    println!("Total events: {}", total_events);
    println!("Batch size: {}", batch_size);
    println!("Duration: {:?}", duration);
    println!("Events/sec: {:.0}", events_per_sec);
    println!("Stats: {:?}", stats);

    let target = 200_000.0;
    if events_per_sec > target {
        println!("PASS: {:.0} > {:.0}", events_per_sec, target);
    } else {
        println!("WARN: {:.0} < {:.0} (target)", events_per_sec, target);
    }
}

fn bench_arena_pool() {
    println!("\n=== Arena Pool Allocation Performance ===");
    reset_arena_stats();

    let iterations = 10_000;
    let allocs_per_iter = 100;

    let start = Instant::now();
    for _ in 0..iterations {
        let arena = get_arena();
        for i in 0..allocs_per_iter {
            let _ = arena.alloc_str(&format!("test-string-{}", i));
        }
    }
    let duration = start.elapsed();

    let total_allocs = iterations * allocs_per_iter;
    let allocs_per_sec = total_allocs as f64 / duration.as_secs_f64();
    let stats = arena_stats();

    println!("Total allocations: {}", total_allocs);
    println!("Duration: {:?}", duration);
    println!("Allocations/sec: {:.0}", allocs_per_sec);
    println!("Arenas created: {}", stats.arenas_created);
    println!("Arenas recycled: {}", stats.arenas_recycled);
    println!("Recycle rate: {:.1}%", stats.recycle_rate * 100.0);

    let target = 10_000_000.0;
    if allocs_per_sec > target {
        println!("PASS: {:.0} > {:.0}", allocs_per_sec, target);
    } else {
        println!("WARN: {:.0} < {:.0} (target)", allocs_per_sec, target);
    }
}

fn bench_simd_filter() {
    println!("\n=== SIMD Event Filtering Performance ===");
    let event_count = 50_000;

    let events: Vec<Event> = (0..event_count)
        .map(|i| {
            Event::reconstruct_from_strings(
                uuid::Uuid::new_v4(),
                format!("event.type.{}", i % 10),
                format!("entity-{}", i % 1000),
                format!("tenant-{}", i % 5),
                json!({"index": i}),
                Utc::now() - chrono::Duration::hours(event_count as i64 - i as i64),
                None,
                1,
            )
        })
        .collect();

    let filter = SimdEventFilter::new();
    let threshold = Utc::now() - chrono::Duration::hours((event_count / 2) as i64);

    // Warm up
    for _ in 0..10 {
        let _ = filter.filter_events(&events, &FilterPredicate::TimestampAfter(threshold));
    }
    filter.reset_stats();

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = filter.filter_events(&events, &FilterPredicate::TimestampAfter(threshold));
    }
    let duration = start.elapsed();

    let events_per_sec = (event_count * iterations) as f64 / duration.as_secs_f64();

    println!("Events: {}", event_count);
    println!("Iterations: {}", iterations);
    println!("Duration: {:?}", duration);
    println!("Events/sec: {:.0}", events_per_sec);
    println!("SIMD available: {}", filter.is_simd_available());

    let target = 1_000_000.0;
    if events_per_sec > target {
        println!("PASS: {:.0} > {:.0}", events_per_sec, target);
    } else {
        println!("WARN: {:.0} < {:.0} (target)", events_per_sec, target);
    }
}

fn bench_full_pipeline() {
    println!("\n=== Full Pipeline Performance (Concurrent) ===");
    let processor = Arc::new(BatchProcessor::with_config(
        BatchProcessorConfig::high_throughput(),
    ));

    let total_events = 100_000;
    let batch_size = 5_000;
    let thread_count = 4;
    let events_per_thread = total_events / thread_count;

    let start = Instant::now();
    std::thread::scope(|s| {
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

    println!("Total events: {}", total_events);
    println!("Threads: {}", thread_count);
    println!("Duration: {:?}", duration);
    println!("Events/sec: {:.0}", events_per_sec);
    println!("Avg batch size: {:.1}", stats.avg_batch_size);

    let target = 300_000.0;
    if events_per_sec > target {
        println!("PASS: {:.0} > {:.0}", events_per_sec, target);
    } else {
        println!("WARN: {:.0} < {:.0} (target)", events_per_sec, target);
    }
}

fn bench_sustained() {
    println!("\n=== Sustained Throughput Test ===");
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

    println!("Test duration: {:?}", start.elapsed());
    println!("Processing time: {:?}", total_duration);
    println!("Total events: {}", total_events);
    println!("Events/sec: {:.0}", events_per_sec);

    let target = 200_000.0;
    if events_per_sec > target {
        println!("PASS: {:.0} > {:.0}", events_per_sec, target);
    } else {
        println!("WARN: {:.0} < {:.0} (target)", events_per_sec, target);
    }
}

fn main() {
    println!("AllSource Performance Benchmarks");
    println!("==============================");
    println!("Run with: cargo run --release -p allsource-performance");
    println!();

    #[cfg(debug_assertions)]
    println!("WARNING: Running in debug mode. Results will be 10-20x slower than release.");

    bench_simd_json();
    bench_lock_free_queue();
    bench_sharded_queue();
    bench_batch_processor();
    bench_arena_pool();
    bench_simd_filter();
    bench_full_pipeline();
    bench_sustained();

    println!("\n==============================");
    println!("Benchmarks complete");
}
