//! Shared test fixture builders for integration tests and cross-crate use.
//!
//! This module is public so that dependent crates (e.g. `allsource-performance`)
//! can import ready-made fixtures instead of duplicating test data.

use crate::{
    domain::{
        entities::{Event, Tenant, tenant::TenantQuotas},
        value_objects::{EntityId, EventType, TenantId},
    },
    store::{EventStore, EventStoreConfig},
};

/// Create a minimal test event with sensible defaults.
pub fn test_event(entity_id: &str, event_type: &str) -> Event {
    Event::from_strings(
        event_type.to_string(),
        entity_id.to_string(),
        "default".to_string(),
        serde_json::json!({"test": true}),
        None,
    )
    .expect("test event should be valid")
}

/// Create a test event with a custom JSON payload.
pub fn test_event_with_payload(
    entity_id: &str,
    event_type: &str,
    payload: serde_json::Value,
) -> Event {
    Event::from_strings(
        event_type.to_string(),
        entity_id.to_string(),
        "default".to_string(),
        payload,
        None,
    )
    .expect("test event should be valid")
}

/// Create a test event scoped to a specific tenant.
pub fn test_event_for_tenant(entity_id: &str, event_type: &str, tenant_id: &str) -> Event {
    Event::from_strings(
        event_type.to_string(),
        entity_id.to_string(),
        tenant_id.to_string(),
        serde_json::json!({"test": true}),
        None,
    )
    .expect("test event should be valid")
}

/// Create an in-memory EventStore (no persistence).
pub fn test_store() -> EventStore {
    EventStore::new()
}

/// Create an EventStore backed by a temporary directory for persistence tests.
pub fn test_store_with_persistence(dir: &std::path::Path) -> EventStore {
    EventStore::with_config(EventStoreConfig::with_persistence(dir))
}

/// Create a test tenant with standard quotas.
pub fn test_tenant(id: &str, name: &str) -> Tenant {
    let tenant_id = TenantId::new(id.to_string()).expect("valid tenant id");
    Tenant::new(tenant_id, name.to_string(), TenantQuotas::standard()).expect("valid tenant")
}

/// Pre-built value objects for quick use in tests.
pub fn test_event_type(name: &str) -> EventType {
    EventType::new(name.to_string()).expect("valid event type")
}

pub fn test_entity_id(id: &str) -> EntityId {
    EntityId::new(id.to_string()).expect("valid entity id")
}

pub fn test_tenant_id(id: &str) -> TenantId {
    TenantId::new(id.to_string()).expect("valid tenant id")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_create_valid_objects() {
        let event = test_event("user-1", "user.created");
        assert_eq!(event.entity_id.as_str(), "user-1");
        assert_eq!(event.event_type.as_str(), "user.created");

        let event2 = test_event_with_payload(
            "order-1",
            "order.placed",
            serde_json::json!({"amount": 100}),
        );
        assert_eq!(event2.entity_id.as_str(), "order-1");

        let event3 = test_event_for_tenant("item-1", "item.added", "acme");
        assert_eq!(event3.tenant_id.as_str(), "acme");
    }

    #[test]
    fn test_store_fixture() {
        let store = test_store();
        let event = test_event("e-1", "test.event");
        store.ingest(event).unwrap();
        assert_eq!(store.stats().total_events, 1);
    }

    #[test]
    fn test_value_object_fixtures() {
        let et = test_event_type("user.created");
        assert_eq!(et.as_str(), "user.created");

        let eid = test_entity_id("user-123");
        assert_eq!(eid.as_str(), "user-123");

        let tid = test_tenant_id("acme");
        assert_eq!(tid.as_str(), "acme");
    }
}
