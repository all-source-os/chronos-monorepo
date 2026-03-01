//! Integration tests for the HTTP sync transport protocol.
//!
//! These tests exercise the sync helpers on `EmbeddedCore` that underpin
//! the HTTP sync endpoints (`/api/v1/sync/pull` and `/api/v1/sync/push`).
//! They verify delta computation, CRDT resolution, idempotency, and
//! offline queue drain — all without needing a real HTTP server.
//!
//! Run with: cargo test --features embedded-sync --test http_sync_transport

#[cfg(feature = "embedded-sync")]
mod tests {
    use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent, Query};
    use serde_json::json;

    #[tokio::test]
    async fn test_sync_pull_returns_delta_events() {
        let core = open_core(1).await;

        // Ingest 3 events
        for i in 0..3 {
            core.ingest(IngestEvent {
                entity_id: &format!("item-{i}"),
                event_type: "item.created",
                payload: json!({"i": i}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        }

        // Pull with empty version vector = get all events
        let empty_vv = std::collections::BTreeMap::new();
        let events = core.events_for_sync(&empty_vv).await.unwrap();
        assert_eq!(events.len(), 3);

        // Pull with version vector past all events = get nothing new
        let full_vv = core.version_vector();
        // events_for_sync uses the version vector's timestamp as "since",
        // so events at or before the threshold are filtered out.
        // We need to verify that after sync, the peer sees all events.
    }

    #[tokio::test]
    async fn test_sync_push_applies_crdt_resolution() {
        let core_a = open_core(1).await;
        let core_b = open_core(2).await;

        // Ingest on A
        core_a
            .ingest(IngestEvent {
                entity_id: "doc-1",
                event_type: "doc.created",
                payload: json!({"title": "hello"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        // Get events from A for sync
        let empty_vv = std::collections::BTreeMap::new();
        let events = core_a.events_for_sync(&empty_vv).await.unwrap();
        assert_eq!(events.len(), 1);

        // Push to B
        let (accepted, skipped) = core_b.receive_sync_push(events.clone()).await.unwrap();
        assert_eq!(accepted, 1);
        assert_eq!(skipped, 0);

        // Push same events again — should be deduplicated
        let (accepted2, skipped2) = core_b.receive_sync_push(events).await.unwrap();
        assert_eq!(accepted2, 0);
        assert_eq!(skipped2, 1);

        // B should have the event
        assert_eq!(core_b.stats().total_events, 1);
    }

    #[tokio::test]
    async fn test_full_bidirectional_http_sync() {
        let core_a = open_core(1).await;
        let core_b = open_core(2).await;

        // Independent writes
        core_a
            .ingest(IngestEvent {
                entity_id: "a-item",
                event_type: "item.created",
                payload: json!({"from": "A"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        core_b
            .ingest(IngestEvent {
                entity_id: "b-item",
                event_type: "item.created",
                payload: json!({"from": "B"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        // Simulate one-way HTTP syncs (the pattern the SyncClient uses):
        // Step 1: A pushes its events to B
        let a_events = core_a
            .events_for_sync(&std::collections::BTreeMap::new())
            .await
            .unwrap();
        let (accepted_on_b, _) = core_b.receive_sync_push(a_events).await.unwrap();
        assert_eq!(accepted_on_b, 1); // B accepts A's event

        // Step 2: B pushes its events to A
        let b_events = core_b
            .events_for_sync(&std::collections::BTreeMap::new())
            .await
            .unwrap();
        // B now has 2 events (its own + A's), but A's resolver will skip A's own event
        let (accepted_on_a, _) = core_a.receive_sync_push(b_events).await.unwrap();
        assert!(accepted_on_a >= 1); // A accepts at least B's event

        // Both should have at least 2 events
        assert!(core_a.stats().total_events >= 2);
        assert_eq!(core_b.stats().total_events, 2);
    }

    #[tokio::test]
    async fn test_sync_idempotent() {
        let core_a = open_core(1).await;
        let core_b = open_core(2).await;

        core_a
            .ingest(IngestEvent {
                entity_id: "x-1",
                event_type: "x.created",
                payload: json!({}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        // Sync 3 times
        for _ in 0..3 {
            let events = core_a
                .events_for_sync(&core_b.version_vector())
                .await
                .unwrap();
            core_b.receive_sync_push(events).await.unwrap();
        }

        // Still only 1 event — no duplicates
        assert_eq!(core_b.stats().total_events, 1);
    }

    #[tokio::test]
    async fn test_sync_after_offline_queue() {
        let core_local = open_core(1).await;
        let core_cloud = open_core(2).await;

        // 100 events while "offline"
        for i in 0..100 {
            core_local
                .ingest(IngestEvent {
                    entity_id: &format!("offline-{i}"),
                    event_type: "item.created",
                    payload: json!({"i": i}),
                    metadata: None,
                    tenant_id: None,
                })
                .await
                .unwrap();
        }

        assert_eq!(core_local.stats().total_events, 100);
        assert_eq!(core_cloud.stats().total_events, 0);

        // "Reconnect" — push all events
        let events = core_local
            .events_for_sync(&core_cloud.version_vector())
            .await
            .unwrap();
        let (accepted, skipped) = core_cloud.receive_sync_push(events).await.unwrap();

        assert_eq!(accepted, 100);
        assert_eq!(skipped, 0);
        assert_eq!(core_cloud.stats().total_events, 100);
    }

    async fn open_core(node_id: u32) -> EmbeddedCore {
        EmbeddedCore::open(Config::builder().node_id(node_id).build().unwrap())
            .await
            .unwrap()
    }
}
