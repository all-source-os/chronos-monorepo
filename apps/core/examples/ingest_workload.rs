//! Deterministic in-memory ingest workload for profiling.
//!
//! Mirrors `benches/performance_benchmarks.rs::ingestion_throughput` so
//! samply/dhat flamegraphs point at the same hot paths the criterion
//! baseline measures.
//!
//! Usage:
//!   cargo build --release --example ingest_workload
//!   samply record ./target/release/examples/ingest_workload
//!
//! With dhat-heap (when the `dhat-heap` feature is enabled):
//!   cargo run --release --example ingest_workload --features dhat-heap
//!   # produces dhat-heap.json; open at https://nnethercote.github.io/dh_view/dh_view.html

#[cfg(not(feature = "dhat-heap"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use allsource_core::{domain::entities::Event, store::EventStore};
use serde_json::json;

const ITERATIONS: usize = 20;
const EVENTS_PER_ITER: usize = 10_000;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    for _ in 0..ITERATIONS {
        let store = EventStore::new();
        for i in 0..EVENTS_PER_ITER {
            let event = Event::from_strings(
                "benchmark.event".to_string(),
                format!("entity-{}", i % 100),
                "default".to_string(),
                json!({ "index": i, "data": "payload" }),
                None,
            )
            .unwrap();
            store.ingest(&event).unwrap();
        }
        std::hint::black_box(store);
    }
}
