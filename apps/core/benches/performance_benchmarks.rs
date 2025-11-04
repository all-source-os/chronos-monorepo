use allsource_core::{
    event::Event,
    snapshot::SnapshotConfig,
    store::{EventStore, EventStoreConfig},
};
use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

fn create_event(entity_id: &str, event_type: &str, payload: serde_json::Value) -> Event {
    Event {
        id: Uuid::new_v4(),
        event_type: event_type.to_string(),
        entity_id: entity_id.to_string(),
        payload,
        timestamp: Utc::now(),
        metadata: None,
        version: 1,
    }
}

/// Benchmark 1: Event ingestion throughput
fn bench_ingestion_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingestion_throughput");

    for size in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let store = EventStore::new();
                for i in 0..size {
                    let event = create_event(
                        &format!("entity-{}", i % 100),
                        "benchmark.event",
                        json!({"index": i, "data": "payload"}),
                    );
                    store.ingest(event).unwrap();
                }
            });
        });
    }

    group.finish();
}

/// Benchmark 2: Query performance
fn bench_query_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_performance");

    // Setup: Create store with events
    let store = EventStore::new();
    for i in 0..10_000 {
        let event = create_event(
            &format!("entity-{}", i % 100),
            "query.test",
            json!({"value": i}),
        );
        store.ingest(event).unwrap();
    }

    group.bench_function("query_all_entity_events", |b| {
        b.iter(|| {
            let query = allsource_core::event::QueryEventsRequest {
                entity_id: Some("entity-42".to_string()),
                event_type: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
            };
            black_box(store.query(query).unwrap());
        });
    });

    group.bench_function("query_by_type", |b| {
        b.iter(|| {
            let query = allsource_core::event::QueryEventsRequest {
                entity_id: None,
                event_type: Some("query.test".to_string()),
                as_of: None,
                since: None,
                until: None,
                limit: Some(100),
            };
            black_box(store.query(query).unwrap());
        });
    });

    group.finish();
}

/// Benchmark 3: State reconstruction with and without snapshots
fn bench_state_reconstruction(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_reconstruction");

    // Without snapshots
    {
        let store = EventStore::new();
        for i in 0..1_000 {
            let event = create_event(
                "reconstruction-entity",
                "state.update",
                json!({"value": i, "timestamp": Utc::now()}),
            );
            store.ingest(event).unwrap();
        }

        group.bench_function("without_snapshot_1000_events", |b| {
            b.iter(|| {
                black_box(store.reconstruct_state("reconstruction-entity", None).unwrap());
            });
        });
    }

    // With snapshots
    {
        let snapshot_config = SnapshotConfig {
            event_threshold: 100,
            auto_snapshot: true,
            ..Default::default()
        };

        let config = EventStoreConfig {
            snapshot_config,
            ..Default::default()
        };

        let store = EventStore::with_config(config);

        // Ingest 1000 events (will create snapshots automatically)
        for i in 0..1_000 {
            let event = create_event(
                "reconstruction-entity-snap",
                "state.update",
                json!({"value": i}),
            );
            store.ingest(event).unwrap();
        }

        group.bench_function("with_snapshots_1000_events", |b| {
            b.iter(|| {
                black_box(
                    store
                        .reconstruct_state("reconstruction-entity-snap", None)
                        .unwrap(),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark 4: Concurrent write throughput
fn bench_concurrent_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_writes");

    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            thread_count,
            |b, &threads| {
                b.iter(|| {
                    let store = Arc::new(EventStore::new());
                    let mut handles = vec![];

                    for thread_id in 0..threads {
                        let store_clone = Arc::clone(&store);
                        let handle = std::thread::spawn(move || {
                            for i in 0..250 {
                                let event = create_event(
                                    &format!("thread-{}-entity-{}", thread_id, i),
                                    "concurrent.write",
                                    json!({"thread": thread_id, "index": i}),
                                );
                                store_clone.ingest(event).unwrap();
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark 5: Index lookup performance
fn bench_index_lookups(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_lookups");

    let store = EventStore::new();

    // Create diverse dataset
    for entity_id in 0..100 {
        for i in 0..100 {
            let event = create_event(
                &format!("indexed-entity-{}", entity_id),
                &format!("event.type.{}", i % 10),
                json!({"value": i}),
            );
            store.ingest(event).unwrap();
        }
    }

    group.bench_function("entity_index_lookup", |b| {
        b.iter(|| {
            let query = allsource_core::event::QueryEventsRequest {
                entity_id: Some("indexed-entity-50".to_string()),
                event_type: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
            };
            black_box(store.query(query).unwrap());
        });
    });

    group.bench_function("type_index_lookup", |b| {
        b.iter(|| {
            let query = allsource_core::event::QueryEventsRequest {
                entity_id: None,
                event_type: Some("event.type.5".to_string()),
                as_of: None,
                since: None,
                until: None,
                limit: None,
            };
            black_box(store.query(query).unwrap());
        });
    });

    group.finish();
}

/// Benchmark 6: Parquet write performance
fn bench_parquet_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("parquet_writes");

    group.bench_function("parquet_batch_write_1000", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let config = EventStoreConfig::with_persistence(temp_dir.path());
            let store = EventStore::with_config(config);

            for i in 0..1_000 {
                let event = create_event(
                    &format!("parquet-entity-{}", i % 10),
                    "parquet.write",
                    json!({"index": i}),
                );
                store.ingest(event).unwrap();
            }

            store.flush_storage().unwrap();
        });
    });

    group.finish();
}

/// Benchmark 7: Snapshot creation performance
fn bench_snapshot_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_operations");

    // Setup store with events
    let store = EventStore::new();
    for i in 0..500 {
        let event = create_event(
            "snapshot-entity",
            "data.update",
            json!({"value": i, "data": format!("Event {}", i)}),
        );
        store.ingest(event).unwrap();
    }

    group.bench_function("create_snapshot", |b| {
        b.iter(|| {
            black_box(store.create_snapshot("snapshot-entity").unwrap());
        });
    });

    group.finish();
}

/// Benchmark 8: WAL write performance
fn bench_wal_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("wal_writes");

    group.bench_function("wal_sync_writes_100", |b| {
        b.iter(|| {
            let storage_dir = TempDir::new().unwrap();
            let wal_dir = TempDir::new().unwrap();

            let config = EventStoreConfig {
                storage_dir: Some(storage_dir.path().to_path_buf()),
                wal_dir: Some(wal_dir.path().to_path_buf()),
                wal_config: allsource_core::wal::WALConfig {
                    sync_on_write: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let store = EventStore::with_config(config);

            for i in 0..100 {
                let event = create_event(
                    "wal-entity",
                    "wal.test",
                    json!({"index": i}),
                );
                store.ingest(event).unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark 9: Memory usage scaling
fn bench_memory_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_scaling");

    for event_count in [1_000, 5_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(event_count),
            event_count,
            |b, &count| {
                b.iter(|| {
                    let store = EventStore::new();
                    for i in 0..count {
                        let event = create_event(
                            &format!("mem-entity-{}", i % 100),
                            "memory.test",
                            json!({"value": i}),
                        );
                        store.ingest(event).unwrap();
                    }
                    black_box(store.stats());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark 10: Time-travel query performance
fn bench_time_travel(c: &mut Criterion) {
    let mut group = c.benchmark_group("time_travel");

    let store = EventStore::new();
    let mut timestamps = vec![];

    // Ingest events and capture timestamps
    for i in 0..1_000 {
        let event = create_event(
            "time-travel-entity",
            "history.event",
            json!({"version": i}),
        );
        timestamps.push(event.timestamp);
        store.ingest(event).unwrap();
    }

    group.bench_function("reconstruct_at_halfway", |b| {
        b.iter(|| {
            black_box(
                store
                    .reconstruct_state("time-travel-entity", Some(timestamps[500]))
                    .unwrap(),
            );
        });
    });

    group.bench_function("reconstruct_current", |b| {
        b.iter(|| {
            black_box(store.reconstruct_state("time-travel-entity", None).unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ingestion_throughput,
    bench_query_performance,
    bench_state_reconstruction,
    bench_concurrent_writes,
    bench_index_lookups,
    bench_parquet_writes,
    bench_snapshot_operations,
    bench_wal_writes,
    bench_memory_scaling,
    bench_time_travel,
);

criterion_main!(benches);
