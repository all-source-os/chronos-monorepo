//! Phase 3 TDD tests: verify the embedded feature compiles and works
//! WITHOUT server dependencies (axum, prometheus, reqwest, etc.).
//!
//! Run with: cargo test --no-default-features --features embedded --test embedded_minimal_build
//!
//! The fact that this test file compiles is the primary assertion —
//! it proves the embedded feature doesn't pull in server dependencies.

#[cfg(feature = "embedded")]
mod tests {
    use allsource_core::embedded::{Config, EmbeddedCore, EventView, IngestEvent, Query};
    use serde_json::json;
    use tempfile::TempDir;

    #[tokio::test]
    async fn embedded_compiles_without_server_features() {
        let core = EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap();
        core.ingest(IngestEvent {
            entity_id: "e1",
            event_type: "test.event",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();
        assert_eq!(core.stats().total_events, 1);
    }

    #[tokio::test]
    async fn error_types_work_without_axum() {
        let core = EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap();
        let result = core
            .ingest(IngestEvent {
                entity_id: "",
                event_type: "bad",
                payload: json!({}),
                metadata: None,
                tenant_id: None,
            })
            .await;
        let err = result.unwrap_err();
        // thiserror Display still works without axum IntoResponse
        assert!(!err.to_string().is_empty());
    }

    #[tokio::test]
    async fn query_works_without_server() {
        let core = EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap();

        core.ingest(IngestEvent {
            entity_id: "q1",
            event_type: "item.created",
            payload: json!({"name": "test"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core.query(Query::new().entity_id("q1")).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity_id, "q1");
    }

    #[tokio::test]
    async fn persistence_works_without_server() {
        let tmp = TempDir::new().unwrap();
        let core = EmbeddedCore::open(Config::builder().data_dir(tmp.path()).build().unwrap())
            .await
            .unwrap();

        core.ingest(IngestEvent {
            entity_id: "p1",
            event_type: "data.saved",
            payload: json!({"important": true}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn event_view_serde_works_without_server() {
        let core = EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap();

        core.ingest(IngestEvent {
            entity_id: "s1",
            event_type: "test.serde",
            payload: json!({"x": 42}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core.query(Query::new().entity_id("s1")).await.unwrap();
        let json_str = serde_json::to_string(&events[0]).unwrap();
        let round_tripped: EventView = serde_json::from_str(&json_str).unwrap();
        assert_eq!(round_tripped.entity_id, "s1");
    }
}
