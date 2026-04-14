//! Integration tests for the live-reload subscription path.

use std::{sync::Arc, time::Duration};

use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent};
use chronis::infrastructure::{
    backend::{CoreBackend, SubscribeError},
    projection::TaskProjection,
};
use serde_json::json;
use tokio::time::timeout;

async fn build_embedded_backend() -> CoreBackend {
    let config = Config::builder()
        .single_tenant(true)
        .build()
        .expect("config");
    let core = EmbeddedCore::open(config).await.expect("core");
    let core = Arc::new(core);
    core.inner()
        .register_projection_with_backfill(
            &(Arc::new(TaskProjection::new()) as Arc<dyn allsource_core::application::Projection>),
        )
        .expect("projection");
    CoreBackend::new_embedded(core)
}

/// Subscribing to an embedded backend and ingesting an event should yield
/// a ChangeEvent on the subscription within the timeout window.
#[tokio::test]
async fn embedded_subscribe_fires_on_ingest() {
    let backend = build_embedded_backend().await;
    let mut sub = backend.subscribe();

    backend
        .ingest(IngestEvent {
            entity_id: "t-live-1",
            event_type: "task.created",
            payload: json!({"title": "Live"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .expect("ingest");

    let change = timeout(Duration::from_secs(2), sub.recv_change())
        .await
        .expect("recv timed out")
        .expect("recv error");

    assert_eq!(change.entity_id, "t-live-1");
    assert_eq!(change.event_type, "task.created");
}

/// Subscribers that never read should still let the subscription remain
/// usable for a fresh receiver — i.e. dropping one subscription does not
/// close the broadcast channel.
#[tokio::test]
async fn embedded_subscribe_supports_multiple_receivers() {
    let backend = build_embedded_backend().await;
    let mut sub_a = backend.subscribe();
    let mut sub_b = backend.subscribe();

    backend
        .ingest(IngestEvent {
            entity_id: "t-multi",
            event_type: "task.created",
            payload: json!({"title": "Multi"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .expect("ingest");

    let a = timeout(Duration::from_secs(2), sub_a.recv_change())
        .await
        .expect("a timeout")
        .expect("a recv");
    let b = timeout(Duration::from_secs(2), sub_b.recv_change())
        .await
        .expect("b timeout")
        .expect("b recv");

    assert_eq!(a.entity_id, "t-multi");
    assert_eq!(b.entity_id, "t-multi");
}

/// A subscription on a remote backend with no WS client running should
/// time out (no events) but not error — it's just quiet.
#[tokio::test]
async fn remote_subscribe_quiet_when_no_ws_traffic() {
    let local = {
        let config = Config::builder()
            .single_tenant(true)
            .build()
            .expect("config");
        let core = EmbeddedCore::open(config).await.expect("core");
        Arc::new(core)
    };
    let client = chronis::infrastructure::http_core_client::HttpCoreClient::new("http://unused");
    let backend = CoreBackend::new_remote(client, local);

    let mut sub = backend.subscribe();
    // Expect a timeout, not a closed channel.
    let result = timeout(Duration::from_millis(200), sub.recv_change()).await;
    assert!(result.is_err(), "expected timeout, got {result:?}");
}

/// The lagged-error branch should be triggered if a subscriber falls far
/// behind. We don't assert exact numbers because lag accounting depends on
/// the broadcast channel internals.
#[tokio::test]
async fn embedded_subscribe_lag_surfaces_as_error_or_change() {
    let backend = build_embedded_backend().await;
    let mut sub = backend.subscribe();

    // Fire many events without reading the subscription.
    for i in 0..2048 {
        backend
            .ingest(IngestEvent {
                entity_id: "t-lag",
                event_type: "task.created",
                payload: json!({ "i": i }),
                metadata: None,
                tenant_id: None,
            })
            .await
            .expect("ingest");
    }

    // Either we get a Lagged error or we get the next available event.
    let result = timeout(Duration::from_secs(2), sub.recv_change())
        .await
        .expect("recv timeout");
    match result {
        Ok(change) => {
            assert_eq!(change.entity_id, "t-lag");
        }
        Err(SubscribeError::Lagged(n)) => {
            assert!(n > 0);
        }
        Err(SubscribeError::Closed) => panic!("subscription unexpectedly closed"),
    }
}
