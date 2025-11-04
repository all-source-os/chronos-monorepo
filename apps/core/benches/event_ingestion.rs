use allsource_core::event::Event;
use allsource_core::store::EventStore;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde_json::json;
use uuid::Uuid;

fn generate_test_event(entity_id: usize) -> Event {
    Event {
        id: Uuid::new_v4(),
        event_type: "benchmark.test".to_string(),
        entity_id: format!("entity-{}", entity_id),
        payload: json!({
            "value": entity_id,
            "timestamp": chrono::Utc::now(),
            "data": "benchmark data payload"
        }),
        timestamp: chrono::Utc::now(),
        metadata: None,
        version: 1,
    }
}

fn bench_single_event_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingestion");

    group.bench_function("single_event", |b| {
        let store = EventStore::new();

        b.iter(|| {
            let event = generate_test_event(1);
            store.ingest(black_box(event)).unwrap();
        });
    });

    group.finish();
}

fn bench_batch_event_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_ingestion");

    for batch_size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &size| {
                b.iter(|| {
                    let store = EventStore::new();

                    for i in 0..size {
                        let event = generate_test_event(i);
                        store.ingest(black_box(event)).unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_ingestion");

    group.bench_function("parallel_100_threads", |b| {
        use std::sync::Arc;
        use std::thread;

        b.iter(|| {
            let store = Arc::new(EventStore::new());
            let mut handles = vec![];

            for thread_id in 0..100 {
                let store_clone = Arc::clone(&store);

                let handle = thread::spawn(move || {
                    for i in 0..10 {
                        let event = generate_test_event(thread_id * 10 + i);
                        store_clone.ingest(event).unwrap();
                    }
                });

                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    group.finish();
}

fn bench_query_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("query");

    // Setup: create a store with 10k events across 1k entities
    let store = EventStore::new();

    for i in 0..10000 {
        let entity_id = i % 1000;
        let event = generate_test_event(entity_id);
        store.ingest(event).unwrap();
    }

    group.bench_function("query_by_entity", |b| {
        use allsource_core::event::QueryEventsRequest;

        b.iter(|| {
            let request = QueryEventsRequest {
                entity_id: Some("entity-500".to_string()),
                event_type: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
            };

            store.query(black_box(request)).unwrap();
        });
    });

    group.bench_function("query_by_type", |b| {
        use allsource_core::event::QueryEventsRequest;

        b.iter(|| {
            let request = QueryEventsRequest {
                entity_id: None,
                event_type: Some("benchmark.test".to_string()),
                as_of: None,
                since: None,
                until: None,
                limit: Some(100),
            };

            store.query(black_box(request)).unwrap();
        });
    });

    group.finish();
}

fn bench_state_reconstruction(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_reconstruction");

    // Setup: create a store with 100 events for one entity
    let store = EventStore::new();

    for i in 0..100 {
        let mut event = generate_test_event(1);
        event.payload = json!({
            "step": i,
            "value": i * 10,
            "data": format!("step-{}", i)
        });
        store.ingest(event).unwrap();
    }

    group.bench_function("reconstruct_100_events", |b| {
        b.iter(|| {
            store.reconstruct_state(black_box("entity-1"), None).unwrap();
        });
    });

    group.bench_function("snapshot_retrieval", |b| {
        b.iter(|| {
            store.get_snapshot(black_box("entity-1")).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_event_ingestion,
    bench_batch_event_ingestion,
    bench_concurrent_ingestion,
    bench_query_performance,
    bench_state_reconstruction
);

criterion_main!(benches);
