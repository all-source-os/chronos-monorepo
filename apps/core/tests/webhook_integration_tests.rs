//! Webhook integration tests
//!
//! Tests the full webhook lifecycle: register → ingest event → verify delivery attempt.

use allsource_core::{
    application::services::webhook::{DeliveryStatus, RegisterWebhookRequest},
    domain::entities::Event,
    store::EventStore,
    webhook_worker,
};
use serde_json::json;
use std::sync::Arc;

fn create_test_event(entity_id: &str, event_type: &str, payload: serde_json::Value) -> Event {
    Event::from_strings(
        event_type.to_string(),
        entity_id.to_string(),
        "default".to_string(),
        payload,
        None,
    )
    .unwrap()
}

/// Test: register a webhook, ingest an event, verify the delivery was attempted.
///
/// We use a URL that will fail (no server listening), so we expect
/// a delivery record with a Failed/Retrying status after the worker processes it.
#[tokio::test]
async fn test_webhook_register_ingest_delivery() {
    let store = Arc::new(EventStore::new());

    // Wire up the webhook delivery worker
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    store.set_webhook_tx(tx);
    let registry = store.webhook_registry();

    // Spawn the delivery worker (it will process tasks from the channel)
    let registry_clone = Arc::clone(&registry);
    let worker_handle = tokio::spawn(async move {
        webhook_worker::run_webhook_delivery_worker(rx, registry_clone).await;
    });

    // Step 1: Register a webhook targeting user.* events
    let webhook = registry.register(RegisterWebhookRequest {
        tenant_id: "default".to_string(),
        url: "http://127.0.0.1:19999/webhook-test".to_string(), // No server here — will fail
        event_types: vec!["user.*".to_string()],
        entity_ids: vec![],
        secret: Some("test-secret-123".to_string()),
        description: Some("Integration test webhook".to_string()),
    });

    assert!(registry.get(webhook.id).is_some());
    assert!(webhook.active);

    // Step 2: Ingest an event that matches the webhook filter
    let event = create_test_event("user-42", "user.created", json!({"name": "Alice"}));
    let event_id = event.id;
    store.ingest(&event).unwrap();

    // Step 3: Wait for the delivery worker to process and exhaust retries.
    // The worker does exponential backoff (2s, 4s, 8s, 16s, 32s).
    // For testing, we just need to wait long enough for at least the first attempt.
    // We'll poll for delivery records.
    let mut found_delivery = false;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let deliveries = registry.get_deliveries(webhook.id, 100);
        if !deliveries.is_empty() {
            found_delivery = true;
            let first = &deliveries[0];
            assert_eq!(first.webhook_id, webhook.id);
            assert_eq!(first.event_id, event_id);
            // The delivery should have failed (connection refused) or be retrying
            assert!(
                first.status == DeliveryStatus::Retrying || first.status == DeliveryStatus::Failed,
                "Expected Retrying or Failed, got {:?}",
                first.status
            );
            assert_eq!(first.attempt, 1);
            // Should have a connection error
            assert!(first.error.is_some() || first.response_status.is_some());
            break;
        }
    }

    assert!(found_delivery, "Expected at least one delivery record");

    // Clean up: drop the store to close the channel, which stops the worker
    drop(store);
    // Worker will exit when channel is closed
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), worker_handle).await;
}

/// Test: webhook with event type filter does not deliver non-matching events.
#[tokio::test]
async fn test_webhook_event_type_filter() {
    let store = Arc::new(EventStore::new());
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    store.set_webhook_tx(tx);
    let registry = store.webhook_registry();

    let registry_clone = Arc::clone(&registry);
    let worker_handle = tokio::spawn(async move {
        webhook_worker::run_webhook_delivery_worker(rx, registry_clone).await;
    });

    // Register webhook that only listens to order.* events
    let webhook = registry.register(RegisterWebhookRequest {
        tenant_id: "default".to_string(),
        url: "http://127.0.0.1:19998/webhook-filter-test".to_string(),
        event_types: vec!["order.*".to_string()],
        entity_ids: vec![],
        secret: None,
        description: None,
    });

    // Ingest a user event (should NOT trigger webhook)
    let event = create_test_event("user-1", "user.created", json!({"name": "Bob"}));
    store.ingest(&event).unwrap();

    // Wait a bit and verify no deliveries
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let deliveries = registry.get_deliveries(webhook.id, 100);
    assert!(
        deliveries.is_empty(),
        "Non-matching event should not trigger delivery"
    );

    drop(store);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), worker_handle).await;
}

/// Test: webhook CRUD lifecycle
#[test]
fn test_webhook_crud_lifecycle() {
    let registry = allsource_core::application::services::webhook::WebhookRegistry::new();

    // Create
    let webhook = registry.register(RegisterWebhookRequest {
        tenant_id: "tenant-1".to_string(),
        url: "https://example.com/hook".to_string(),
        event_types: vec!["user.*".to_string()],
        entity_ids: vec![],
        secret: Some("secret-key".to_string()),
        description: Some("Test hook".to_string()),
    });
    let id = webhook.id;

    // Read
    let fetched = registry.get(id).unwrap();
    assert_eq!(fetched.url, "https://example.com/hook");
    assert!(fetched.active);

    // List by tenant
    let hooks = registry.list_by_tenant("tenant-1");
    assert_eq!(hooks.len(), 1);

    // Update
    let updated = registry
        .update(
            id,
            allsource_core::application::services::webhook::UpdateWebhookRequest {
                url: Some("https://example.com/new-hook".to_string()),
                event_types: None,
                entity_ids: None,
                active: Some(false),
                description: None,
            },
        )
        .unwrap();
    assert_eq!(updated.url, "https://example.com/new-hook");
    assert!(!updated.active);

    // Delete
    let deleted = registry.delete(id);
    assert!(deleted.is_some());
    assert!(registry.get(id).is_none());
    assert!(registry.list_by_tenant("tenant-1").is_empty());
}

/// Test: delivery status tracking with multiple delivery records
#[test]
fn test_delivery_status_tracking() {
    let registry = allsource_core::application::services::webhook::WebhookRegistry::new();
    let webhook_id = uuid::Uuid::new_v4();
    let event_id = uuid::Uuid::new_v4();

    // Record a retrying delivery
    registry.record_delivery(
        allsource_core::application::services::webhook::WebhookDelivery {
            id: uuid::Uuid::new_v4(),
            webhook_id,
            event_id,
            status: DeliveryStatus::Retrying,
            attempt: 1,
            max_attempts: 3,
            response_status: Some(500),
            response_body: Some("Internal Server Error".to_string()),
            error: None,
            created_at: chrono::Utc::now(),
            next_retry_at: Some(chrono::Utc::now() + chrono::Duration::seconds(1)),
        },
    );

    // Record a success delivery
    registry.record_delivery(
        allsource_core::application::services::webhook::WebhookDelivery {
            id: uuid::Uuid::new_v4(),
            webhook_id,
            event_id,
            status: DeliveryStatus::Success,
            attempt: 2,
            max_attempts: 3,
            response_status: Some(200),
            response_body: None,
            error: None,
            created_at: chrono::Utc::now(),
            next_retry_at: None,
        },
    );

    let deliveries = registry.get_deliveries(webhook_id, 10);
    assert_eq!(deliveries.len(), 2);
    assert_eq!(deliveries[0].status, DeliveryStatus::Retrying);
    assert_eq!(deliveries[0].attempt, 1);
    assert_eq!(deliveries[1].status, DeliveryStatus::Success);
    assert_eq!(deliveries[1].attempt, 2);
}
