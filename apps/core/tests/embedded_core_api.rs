//! TDD tests for EmbeddedCore facade API.
//!
//! Run with: cargo test --features embedded --test embedded_core_api

#[cfg(feature = "embedded")]
mod tests {
    use allsource_core::embedded::{Config, EmbeddedCore, EventView, IngestEvent, Query};
    use serde_json::json;
    use tempfile::TempDir;

    // =========================================================================
    // Config builder tests
    // =========================================================================

    #[test]
    fn config_builder_defaults_to_in_memory_single_tenant() {
        let config = Config::builder().build().unwrap();
        // No data_dir means in-memory only
        assert!(config.data_dir().is_none());
        // Single-tenant by default
        assert!(config.single_tenant());
    }

    #[test]
    fn config_builder_with_data_dir() {
        let tmp = TempDir::new().unwrap();
        let config = Config::builder().data_dir(tmp.path()).build().unwrap();
        assert!(config.data_dir().is_some());
    }

    #[test]
    fn config_builder_wal_sync_option() {
        let config = Config::builder().wal_sync_on_write(false).build().unwrap();
        assert!(!config.wal_sync_on_write());
    }

    #[test]
    fn config_builder_multi_tenant() {
        let config = Config::builder().single_tenant(false).build().unwrap();
        assert!(!config.single_tenant());
    }

    // =========================================================================
    // EmbeddedCore::open tests
    // =========================================================================

    #[tokio::test]
    async fn open_in_memory() {
        let core = EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .expect("open in-memory should succeed");
        assert_eq!(core.stats().total_events, 0);
    }

    #[tokio::test]
    async fn open_with_persistence() {
        let tmp = TempDir::new().unwrap();
        let core = EmbeddedCore::open(
            Config::builder().data_dir(tmp.path()).build().unwrap(),
        )
        .await
        .expect("open with persistence should succeed");
        assert_eq!(core.stats().total_events, 0);
    }

    // =========================================================================
    // Ingest tests
    // =========================================================================

    #[tokio::test]
    async fn ingest_simple_event() {
        let core = open_in_memory_core().await;

        core.ingest(IngestEvent {
            entity_id: "order-001",
            event_type: "order.placed",
            payload: json!({"total": 99.99}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .expect("ingest should succeed");

        assert_eq!(core.stats().total_events, 1);
    }

    #[tokio::test]
    async fn ingest_with_metadata() {
        let core = open_in_memory_core().await;

        core.ingest(IngestEvent {
            entity_id: "user-001",
            event_type: "user.registered",
            payload: json!({"email": "a@b.com"}),
            metadata: Some(json!({"source": "signup"})),
            tenant_id: None,
        })
        .await
        .unwrap();

        assert_eq!(core.stats().total_events, 1);
    }

    #[tokio::test]
    async fn ingest_rejects_invalid_event_type() {
        let core = open_in_memory_core().await;

        let result = core
            .ingest(IngestEvent {
                entity_id: "e1",
                event_type: "Invalid.Type", // uppercase — should fail EventType validation
                payload: json!({}),
                metadata: None,
                tenant_id: None,
            })
            .await;

        assert!(result.is_err());
        assert_eq!(core.stats().total_events, 0);
    }

    // =========================================================================
    // Query tests
    // =========================================================================

    #[tokio::test]
    async fn query_by_entity_id() {
        let core = open_in_memory_core().await;

        for i in 0..3 {
            core.ingest(IngestEvent {
                entity_id: "order-99",
                event_type: "order.updated",
                payload: json!({"step": i}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        }

        let events = core
            .query(Query::new().entity_id("order-99"))
            .await
            .unwrap();

        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn query_by_event_type_prefix() {
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
        core.ingest(IngestEvent {
            entity_id: "e3",
            event_type: "user.created",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().event_type_prefix("order."))
            .await
            .unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn query_with_limit() {
        let core = open_in_memory_core().await;

        for i in 0..10 {
            core.ingest(IngestEvent {
                entity_id: &format!("e-{i}"),
                event_type: "item.created",
                payload: json!({"i": i}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        }

        let events = core.query(Query::new().limit(3)).await.unwrap();
        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn query_returns_event_view_with_plain_strings() {
        let core = open_in_memory_core().await;

        core.ingest(IngestEvent {
            entity_id: "ent-1",
            event_type: "item.added",
            payload: json!({"qty": 5}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("ent-1"))
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        let ev: &EventView = &events[0];
        assert_eq!(ev.event_type, "item.added");
        assert_eq!(ev.entity_id, "ent-1");
        assert_eq!(ev.payload["qty"], 5);
        assert!(!ev.id.is_nil());
    }

    #[tokio::test]
    async fn query_empty_result() {
        let core = open_in_memory_core().await;

        let events = core
            .query(Query::new().entity_id("nonexistent"))
            .await
            .unwrap();

        assert!(events.is_empty());
    }

    // =========================================================================
    // Projection tests
    // =========================================================================

    #[tokio::test]
    async fn projection_returns_none_for_unknown_entity() {
        let core = open_in_memory_core().await;

        // Built-in projections exist but return None for unknown entities
        let state = core.projection("entity_snapshots", "nonexistent");
        assert!(state.is_none());
    }

    // =========================================================================
    // Single-tenant mode
    // =========================================================================

    #[tokio::test]
    async fn single_tenant_uses_default_tenant_id() {
        let core = open_in_memory_core().await;

        core.ingest(IngestEvent {
            entity_id: "e-st",
            event_type: "event.created",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("e-st"))
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tenant_id, "default");
    }

    // =========================================================================
    // Isolation — no global state
    // =========================================================================

    #[tokio::test]
    async fn multiple_instances_are_isolated() {
        let core_a = open_in_memory_core().await;
        let core_b = open_in_memory_core().await;

        core_a
            .ingest(IngestEvent {
                entity_id: "a1",
                event_type: "a.created",
                payload: json!({}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        assert_eq!(core_a.stats().total_events, 1);
        assert_eq!(core_b.stats().total_events, 0); // isolated
    }

    // =========================================================================
    // Shutdown
    // =========================================================================

    #[tokio::test]
    async fn shutdown_is_graceful() {
        let tmp = TempDir::new().unwrap();
        let core = EmbeddedCore::open(
            Config::builder().data_dir(tmp.path()).build().unwrap(),
        )
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "e1",
            event_type: "data.saved",
            payload: json!({"important": true}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.shutdown().await.expect("shutdown should succeed");
    }

    // =========================================================================
    // inner() escape hatch
    // =========================================================================

    #[tokio::test]
    async fn inner_provides_raw_event_store() {
        let core = open_in_memory_core().await;
        let store = core.inner();
        // Can call raw EventStore methods
        assert_eq!(store.stats().total_events, 0);
    }

    // =========================================================================
    // Phase 2: Serde support
    // =========================================================================

    #[tokio::test]
    async fn event_view_serializes_to_json() {
        let core = open_in_memory_core().await;

        core.ingest(IngestEvent {
            entity_id: "ser-1",
            event_type: "item.created",
            payload: json!({"name": "widget"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("ser-1"))
            .await
            .unwrap();

        let json_str = serde_json::to_string(&events[0]).unwrap();
        assert!(json_str.contains("item.created"));
        assert!(json_str.contains("widget"));
    }

    #[tokio::test]
    async fn event_view_deserializes_from_json() {
        let core = open_in_memory_core().await;

        core.ingest(IngestEvent {
            entity_id: "deser-1",
            event_type: "item.created",
            payload: json!({"x": 1}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("deser-1"))
            .await
            .unwrap();

        let json_str = serde_json::to_string(&events[0]).unwrap();
        let round_tripped: EventView = serde_json::from_str(&json_str).unwrap();
        assert_eq!(round_tripped.entity_id, "deser-1");
        assert_eq!(round_tripped.event_type, "item.created");
        assert_eq!(round_tripped.payload["x"], 1);
    }

    // =========================================================================
    // Phase 2: Multi-tenant support via tenant_id on IngestEvent
    // =========================================================================

    #[tokio::test]
    async fn ingest_with_explicit_tenant_id() {
        let core = EmbeddedCore::open(
            Config::builder().single_tenant(false).build().unwrap(),
        )
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "mt-1",
            event_type: "order.placed",
            payload: json!({}),
            metadata: None,
            tenant_id: Some("tenant-acme"),
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("mt-1"))
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tenant_id, "tenant-acme");
    }

    #[tokio::test]
    async fn ingest_without_tenant_id_in_multi_tenant_uses_default() {
        let core = EmbeddedCore::open(
            Config::builder().single_tenant(false).build().unwrap(),
        )
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "mt-2",
            event_type: "order.placed",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("mt-2"))
            .await
            .unwrap();

        assert_eq!(events[0].tenant_id, "default");
    }

    #[tokio::test]
    async fn single_tenant_ignores_explicit_tenant_id() {
        let core = open_in_memory_core().await; // single_tenant=true by default

        core.ingest(IngestEvent {
            entity_id: "st-override",
            event_type: "order.placed",
            payload: json!({}),
            metadata: None,
            tenant_id: Some("ignored-tenant"),
        })
        .await
        .unwrap();

        let events = core
            .query(Query::new().entity_id("st-override"))
            .await
            .unwrap();

        // In single-tenant mode, tenant_id is always "default" regardless of input
        assert_eq!(events[0].tenant_id, "default");
    }

    #[tokio::test]
    async fn multi_tenant_query_filters_by_tenant() {
        let core = EmbeddedCore::open(
            Config::builder().single_tenant(false).build().unwrap(),
        )
        .await
        .unwrap();

        // Ingest events for two tenants
        core.ingest(IngestEvent {
            entity_id: "shared-entity",
            event_type: "data.created",
            payload: json!({"tenant": "a"}),
            metadata: None,
            tenant_id: Some("tenant-a"),
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "shared-entity",
            event_type: "data.created",
            payload: json!({"tenant": "b"}),
            metadata: None,
            tenant_id: Some("tenant-b"),
        })
        .await
        .unwrap();

        // Both events stored with correct tenant IDs
        let all = core
            .query(Query::new().entity_id("shared-entity"))
            .await
            .unwrap();
        assert_eq!(all.len(), 2);

        // Verify tenant IDs were persisted correctly
        let tenants: Vec<&str> = all.iter().map(|e| e.tenant_id.as_str()).collect();
        assert!(tenants.contains(&"tenant-a"));
        assert!(tenants.contains(&"tenant-b"));
    }

    // =========================================================================
    // Phase 2: Root re-export
    // =========================================================================

    #[tokio::test]
    async fn embedded_core_accessible_from_crate_root() {
        // EmbeddedCore should be re-exported at the crate root
        let core = allsource_core::EmbeddedCore::open(
            Config::builder().build().unwrap(),
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
