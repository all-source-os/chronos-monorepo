//! TDD RED phase tests for EmbeddedCore Phase 2: API completeness.
//!
//! These tests define the target API for:
//! - Serde on EventView (serialize/deserialize)
//! - Multi-tenant support in embedded mode
//! - Batch ingestion (atomic)
//! - Exact event_type query
//! - Crate root re-export of EmbeddedCore
//!
//! Run with: cargo test --features embedded-phase2 --test embedded_api_phase2

#[cfg(feature = "embedded-phase2")]
mod tests {
    use allsource_core::embedded::{Config, EmbeddedCore, EventView, IngestEvent, Query};
    use serde_json::json;

    // =========================================================================
    // Serde on EventView
    // =========================================================================

    #[tokio::test]
    async fn event_view_serializes_to_json() {
        let core = open_in_memory_core().await;

        core.ingest(IngestEvent {
            entity_id: "order-1",
            event_type: "order.placed",
            payload: json!({"total": 99.99}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("order-1"))
            .await
            .unwrap();

        let json_str = serde_json::to_string(&events[0]).unwrap();
        assert!(json_str.contains("order.placed"));
        assert!(json_str.contains("order-1"));
        assert!(json_str.contains("99.99"));
    }

    #[tokio::test]
    async fn event_view_round_trips_through_json() {
        let core = open_in_memory_core().await;

        core.ingest(IngestEvent {
            entity_id: "item-1",
            event_type: "item.created",
            payload: json!({"name": "Widget", "qty": 5}),
            metadata: Some(json!({"source": "test"})),
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("item-1"))
            .await
            .unwrap();

        let json_str = serde_json::to_string(&events[0]).unwrap();
        let deserialized: EventView = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.entity_id, "item-1");
        assert_eq!(deserialized.event_type, "item.created");
        assert_eq!(deserialized.payload["name"], "Widget");
        assert_eq!(deserialized.metadata.unwrap()["source"], "test");
    }

    // =========================================================================
    // Multi-tenant embedded mode
    // =========================================================================

    #[tokio::test]
    async fn multi_tenant_ingest_with_explicit_tenant_id() {
        let core = EmbeddedCore::open(Config::builder().single_tenant(false).build().unwrap())
            .await
            .unwrap();

        core.ingest(IngestEvent {
            entity_id: "e1",
            event_type: "test.event",
            tenant_id: Some("tenant-a"),
            payload: json!({}),
            metadata: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("e1").tenant_id("tenant-a"))
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tenant_id, "tenant-a");
    }

    #[tokio::test]
    async fn multi_tenant_isolates_queries() {
        let core = EmbeddedCore::open(Config::builder().single_tenant(false).build().unwrap())
            .await
            .unwrap();

        core.ingest(IngestEvent {
            entity_id: "e1",
            event_type: "test.event",
            tenant_id: Some("tenant-a"),
            payload: json!({"from": "a"}),
            metadata: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "e1",
            event_type: "test.event",
            tenant_id: Some("tenant-b"),
            payload: json!({"from": "b"}),
            metadata: None,
        })
        .await
        .unwrap();

        let a_events = core
            .query(Query::new().entity_id("e1").tenant_id("tenant-a"))
            .await
            .unwrap();
        let b_events = core
            .query(Query::new().entity_id("e1").tenant_id("tenant-b"))
            .await
            .unwrap();

        assert_eq!(a_events.len(), 1);
        assert_eq!(b_events.len(), 1);
        assert_eq!(a_events[0].payload["from"], "a");
        assert_eq!(b_events[0].payload["from"], "b");
    }

    #[tokio::test]
    async fn multi_tenant_defaults_tenant_when_not_specified() {
        let core = EmbeddedCore::open(Config::builder().single_tenant(false).build().unwrap())
            .await
            .unwrap();

        // No tenant_id specified in multi-tenant mode should use "default"
        core.ingest(IngestEvent {
            entity_id: "e1",
            event_type: "test.event",
            tenant_id: None,
            payload: json!({}),
            metadata: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("e1").tenant_id("default"))
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
    }

    // =========================================================================
    // Batch ingestion
    // =========================================================================

    #[tokio::test]
    async fn ingest_batch_multiple_events() {
        let core = open_in_memory_core().await;

        let events = vec![
            IngestEvent {
                entity_id: "e1",
                event_type: "a.created",
                payload: json!({"i": 1}),
                metadata: None,
                tenant_id: None,
            },
            IngestEvent {
                entity_id: "e2",
                event_type: "b.created",
                payload: json!({"i": 2}),
                metadata: None,
                tenant_id: None,
            },
            IngestEvent {
                entity_id: "e3",
                event_type: "c.created",
                payload: json!({"i": 3}),
                metadata: None,
                tenant_id: None,
            },
        ];

        core.ingest_batch(events).await.unwrap();
        assert_eq!(core.stats().total_events, 3);
    }

    #[tokio::test]
    async fn ingest_batch_empty_is_noop() {
        let core = open_in_memory_core().await;
        core.ingest_batch(vec![]).await.unwrap();
        assert_eq!(core.stats().total_events, 0);
    }

    #[tokio::test]
    async fn ingest_batch_atomic_rejects_all_on_invalid() {
        let core = open_in_memory_core().await;

        let events = vec![
            IngestEvent {
                entity_id: "e1",
                event_type: "good.event",
                payload: json!({}),
                metadata: None,
                tenant_id: None,
            },
            IngestEvent {
                entity_id: "",
                event_type: "bad.event",
                payload: json!({}),
                metadata: None,
                tenant_id: None,
            },
        ];

        let result = core.ingest_batch(events).await;
        assert!(result.is_err());
        // Atomic — nothing ingested on failure
        assert_eq!(core.stats().total_events, 0);
    }

    #[tokio::test]
    async fn ingest_batch_preserves_order() {
        let core = open_in_memory_core().await;

        let events = vec![
            IngestEvent {
                entity_id: "e1",
                event_type: "step.one",
                payload: json!({"order": 1}),
                metadata: None,
                tenant_id: None,
            },
            IngestEvent {
                entity_id: "e1",
                event_type: "step.two",
                payload: json!({"order": 2}),
                metadata: None,
                tenant_id: None,
            },
            IngestEvent {
                entity_id: "e1",
                event_type: "step.three",
                payload: json!({"order": 3}),
                metadata: None,
                tenant_id: None,
            },
        ];

        core.ingest_batch(events).await.unwrap();

        let result = core
            .query(Query::new().entity_id("e1"))
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].event_type, "step.one");
        assert_eq!(result[1].event_type, "step.two");
        assert_eq!(result[2].event_type, "step.three");
    }

    // =========================================================================
    // Query by exact event_type
    // =========================================================================

    #[tokio::test]
    async fn query_by_exact_event_type() {
        let core = open_in_memory_core().await;

        core.ingest(IngestEvent {
            entity_id: "e1",
            event_type: "order.placed",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "e2",
            event_type: "order.shipped",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().event_type("order.placed"))
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "order.placed");
    }

    // =========================================================================
    // Crate root re-export
    // =========================================================================

    #[tokio::test]
    async fn embedded_core_importable_from_crate_root() {
        // Verifies `pub use embedded::EmbeddedCore` exists in lib.rs
        use allsource_core::EmbeddedCore;

        let core = EmbeddedCore::open(
            allsource_core::embedded::Config::builder().build().unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(core.stats().total_events, 0);
    }

    // =========================================================================
    // Helper
    // =========================================================================

    async fn open_in_memory_core() -> EmbeddedCore {
        EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap()
    }
}
