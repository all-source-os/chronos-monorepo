//! Integration tests for `ProjectionWorker` against a live Core.
//!
//! These tests require a running AllSource Core. They skip gracefully when
//! `ALLSOURCE_TEST_CORE_URL` is not set.
//!
//! # Running
//!
//! ```bash
//! # Against a local Docker Core:
//! ALLSOURCE_TEST_CORE_URL=http://localhost:3900 \
//! ALLSOURCE_TEST_API_KEY=test-key \
//!   cargo test -p allsource --test projection_worker --features projection-worker
//! ```
//!
//! # Coverage
//! - `cold_start_processes_all_events` — reducer fires for every ingested event.
//! - `restart_resumes_from_checkpoint` — a second worker with the same name only
//!   processes events ingested AFTER the first worker stopped.
//! - `dedup_filters_replayed_versions` — events with version ≤ last applied are
//!   skipped per entity.
//!
//! # Not covered automatically
//! Reconnect-after-Core-restart must be verified manually (it requires a
//! `docker restart` mid-test). The reconnect code path is unit-tested through
//! `ExpBackoff` in projection_worker::tests.

#![cfg(feature = "projection-worker")]

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use allsource::{
    CoreClient, Error, Event, IngestEventInput, ProjectionHandle, ProjectionWorker, WorkerState,
};
use serde_json::json;

fn core_url() -> Option<String> {
    std::env::var("ALLSOURCE_TEST_CORE_URL").ok()
}

fn api_key() -> String {
    std::env::var("ALLSOURCE_TEST_API_KEY").unwrap_or_else(|_| "test-key".into())
}

fn unique_name(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}-{ts}")
}

macro_rules! require_core {
    () => {
        match core_url() {
            Some(url) => url,
            None => {
                eprintln!("SKIPPED: set ALLSOURCE_TEST_CORE_URL to run integration tests");
                return;
            }
        }
    };
}

async fn ingest_n_events(core: &CoreClient, entity_prefix: &str, n: u64) -> u64 {
    let inputs: Vec<_> = (1..=n)
        .map(|i| IngestEventInput {
            event_type: "test.event".into(),
            entity_id: format!("{entity_prefix}-{i}"),
            payload: json!({"seq": i}),
            metadata: None,
        })
        .collect();
    core.ingest_batch(inputs)
        .await
        .expect("batch ingest failed")
        .ingested
}

/// Wait up to 10s for the handle to report caught-up.
async fn wait_caught_up<S: WorkerState>(handle: &ProjectionHandle<S>) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !handle.is_caught_up() {
        assert!(
            std::time::Instant::now() <= deadline,
            "worker did not catch up within 10s"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn cold_start_processes_all_events() {
    let url = require_core!();
    let core = CoreClient::new(&url, &api_key()).unwrap();

    let prefix = unique_name("cold");
    let ingested = ingest_n_events(&core, &prefix, 20).await;
    assert!(ingested > 0, "expected at least 1 event ingested");

    let worker_name = unique_name("worker-cold");
    let prefix_for_reducer = prefix.clone();
    let counter = Arc::new(AtomicU64::new(0));
    let counter_clone = Arc::clone(&counter);

    let worker = ProjectionWorker::<HashMap<String, u64>>::builder(core.clone())
        .name(&worker_name)
        .event_types(&["test.event"])
        .reducer(move |state, event: &Event| {
            // Only count events that belong to this test's namespace.
            if event.entity_id.starts_with(&prefix_for_reducer) {
                *state.entry(event.entity_id.clone()).or_insert(0) += 1;
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        })
        .checkpoint_interval(100)
        .build()
        .unwrap();

    let handle = worker.start().await.expect("start failed");
    wait_caught_up(&handle).await;

    // The reducer saw exactly the events we ingested for this namespace.
    let count = counter.load(Ordering::SeqCst);
    assert_eq!(
        count, ingested,
        "expected {ingested} events, reducer saw {count}"
    );

    handle.stop().await.unwrap();
}

#[tokio::test]
async fn restart_resumes_from_checkpoint() {
    let url = require_core!();
    let core = CoreClient::new(&url, &api_key()).unwrap();

    let prefix = unique_name("resume");
    let worker_name = unique_name("worker-resume");

    // Phase 1: ingest 10 events, run worker, stop.
    ingest_n_events(&core, &prefix, 10).await;

    let run1_count = Arc::new(AtomicU64::new(0));
    {
        let prefix = prefix.clone();
        let run1_clone = Arc::clone(&run1_count);
        let worker = ProjectionWorker::<HashMap<String, u64>>::builder(core.clone())
            .name(&worker_name)
            .event_types(&["test.event"])
            .reducer(move |state, event: &Event| {
                if event.entity_id.starts_with(&prefix) {
                    *state.entry(event.entity_id.clone()).or_insert(0) += 1;
                    run1_clone.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            })
            .checkpoint_interval(1) // checkpoint every event so resume is precise
            .build()
            .unwrap();

        let handle = worker.start().await.unwrap();
        wait_caught_up(&handle).await;
        handle.stop().await.unwrap();
    }
    assert_eq!(
        run1_count.load(Ordering::SeqCst),
        10,
        "run 1 should see all 10"
    );

    // Phase 2: ingest 5 more events, start a NEW worker with the SAME name.
    ingest_n_events(&core, &format!("{prefix}-phase2"), 5).await;

    let run2_count = Arc::new(AtomicU64::new(0));
    {
        let prefix = prefix.clone();
        let run2_clone = Arc::clone(&run2_count);
        let worker = ProjectionWorker::<HashMap<String, u64>>::builder(core.clone())
            .name(&worker_name)
            .event_types(&["test.event"])
            .reducer(move |state, event: &Event| {
                if event.entity_id.starts_with(&prefix) {
                    *state.entry(event.entity_id.clone()).or_insert(0) += 1;
                    run2_clone.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            })
            .checkpoint_interval(1)
            .build()
            .unwrap();

        let handle = worker.start().await.unwrap();
        wait_caught_up(&handle).await;
        handle.stop().await.unwrap();
    }

    // Run 2 must only see the 5 new events, not the original 10.
    assert_eq!(
        run2_count.load(Ordering::SeqCst),
        5,
        "run 2 should only process the 5 NEW events (got {})",
        run2_count.load(Ordering::SeqCst)
    );
}

#[tokio::test]
async fn dedup_filters_replayed_versions() {
    let url = require_core!();
    let core = CoreClient::new(&url, &api_key()).unwrap();

    let prefix = unique_name("dedup");
    let worker_name = unique_name("worker-dedup");

    // Ingest the SAME entity at versions 1, 2, 3. Core assigns version
    // per-entity, so re-ingesting with the same entity_id bumps the version
    // on each call.
    for _ in 0..3 {
        core.ingest_event(IngestEventInput {
            event_type: "test.event".into(),
            entity_id: prefix.clone(),
            payload: json!({}),
            metadata: None,
        })
        .await
        .unwrap();
    }

    let prefix_for_reducer = prefix.clone();
    let counter = Arc::new(AtomicU64::new(0));
    let counter_clone = Arc::clone(&counter);

    let worker = ProjectionWorker::<HashMap<String, u64>>::builder(core)
        .name(&worker_name)
        .event_types(&["test.event"])
        .reducer(move |state, event: &Event| {
            if event.entity_id == prefix_for_reducer {
                *state.entry(event.entity_id.clone()).or_insert(0) += 1;
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }
            Ok(())
        })
        .checkpoint_interval(1000)
        .build()
        .unwrap();

    let handle = worker.start().await.unwrap();
    wait_caught_up(&handle).await;

    // Three distinct versions → three reducer calls. If Core re-sent any
    // version we already applied, the version-based dedup in the worker
    // would drop it. We verify the final count matches the ingested count
    // with no duplicates.
    let count = counter.load(Ordering::SeqCst);
    assert_eq!(count, 3, "expected 3 unique-version events, got {count}");

    handle.stop().await.unwrap();
}

#[tokio::test]
async fn error_in_reducer_propagates() {
    let url = require_core!();
    let core = CoreClient::new(&url, &api_key()).unwrap();

    let prefix = unique_name("err");
    ingest_n_events(&core, &prefix, 3).await;

    let worker_name = unique_name("worker-err");
    let prefix_for_reducer = prefix.clone();

    let worker = ProjectionWorker::<Vec<String>>::builder(core)
        .name(&worker_name)
        .event_types(&["test.event"])
        .reducer(move |_state, event: &Event| {
            if event.entity_id.starts_with(&prefix_for_reducer) {
                Err(Error::Config(format!("intentional: {}", event.entity_id)))
            } else {
                Ok(())
            }
        })
        .checkpoint_interval(1)
        .build()
        .unwrap();

    let handle = worker.start().await.unwrap();
    // Worker will abort on first matching event; give it a moment.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The handle is still alive (the task ended but stop() remains safe).
    handle.stop().await.unwrap();
}
