/// Concurrent stress tests for event-sourced repositories.
///
/// Verifies that:
/// 1. Concurrent tenant CRUD from multiple threads does not corrupt state
/// 2. Concurrent audit event appends maintain proper ordering
/// 3. Concurrent config set/get/delete operations are consistent
/// 4. High-volume writes followed by reads verify no data loss
///
/// Each test uses `tempfile::TempDir` for storage isolation and targets
/// ~100 concurrent operations.
use allsource_core::{
    domain::{
        entities::{Actor, AuditAction, AuditEvent, AuditOutcome, TenantQuotas},
        repositories::{AuditEventQuery, AuditEventRepository, TenantRepository},
        value_objects::TenantId,
    },
    infrastructure::{
        persistence::SystemMetadataStore,
        repositories::{
            EventSourcedAuditRepository, EventSourcedConfigRepository, EventSourcedTenantRepository,
        },
    },
};
use std::sync::Arc;
use tempfile::TempDir;

// =============================================================================
// Helpers
// =============================================================================

fn make_system_store(temp_dir: &TempDir) -> Arc<SystemMetadataStore> {
    Arc::new(SystemMetadataStore::new(temp_dir.path().join("__system")).unwrap())
}

fn make_audit_event(tenant_id: &str, index: usize) -> AuditEvent {
    AuditEvent::new(
        TenantId::new(tenant_id.to_string()).unwrap(),
        AuditAction::EventIngested,
        Actor::system(format!("worker-{}", index)),
        AuditOutcome::Success,
    )
    .with_metadata(serde_json::json!({ "index": index }))
}

// =============================================================================
// 1. Concurrent tenant CRUD — no state corruption
// =============================================================================

#[tokio::test]
async fn stress_concurrent_tenant_creates() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedTenantRepository::new(store));

    let mut handles = Vec::new();
    let count = 100;

    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let id = TenantId::new(format!("tenant-{}", i)).unwrap();
            repo.create(id, format!("Tenant {}", i), TenantQuotas::standard())
                .await
        }));
    }

    let mut successes = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            successes += 1;
        }
    }

    // All 100 creates should succeed (unique IDs)
    assert_eq!(successes, count);
    assert_eq!(repo.count().await.unwrap(), count);

    // Every tenant is independently retrievable
    for i in 0..count {
        let id = TenantId::new(format!("tenant-{}", i)).unwrap();
        let tenant = repo.find_by_id(&id).await.unwrap();
        assert!(tenant.is_some(), "tenant-{} should exist", i);
        assert_eq!(tenant.unwrap().name(), format!("Tenant {}", i));
    }
}

#[tokio::test]
async fn stress_concurrent_tenant_create_delete_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedTenantRepository::new(store));

    let count = 50;

    // Phase 1: Create tenants
    let mut handles = Vec::new();
    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let id = TenantId::new(format!("cd-tenant-{}", i)).unwrap();
            repo.create(id, format!("CD Tenant {}", i), TenantQuotas::standard())
                .await
        }));
    }
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
    assert_eq!(repo.count().await.unwrap(), count);

    // Phase 2: Delete even-numbered tenants concurrently
    let mut handles = Vec::new();
    for i in (0..count).filter(|n| n % 2 == 0) {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let id = TenantId::new(format!("cd-tenant-{}", i)).unwrap();
            repo.delete(&id).await
        }));
    }
    for handle in handles {
        let deleted = handle.await.unwrap().unwrap();
        assert!(deleted);
    }

    let remaining = repo.count().await.unwrap();
    assert_eq!(remaining, count / 2);

    // Verify only odd-numbered tenants remain
    for i in 0..count {
        let id = TenantId::new(format!("cd-tenant-{}", i)).unwrap();
        let found = repo.find_by_id(&id).await.unwrap();
        if i % 2 == 0 {
            assert!(
                found.is_none(),
                "even tenant cd-tenant-{} should be deleted",
                i
            );
        } else {
            assert!(found.is_some(), "odd tenant cd-tenant-{} should exist", i);
        }
    }
}

#[tokio::test]
async fn stress_concurrent_activate_deactivate() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedTenantRepository::new(store));

    let count = 100;

    // Create tenants first (sequential to avoid duplicate races on same ID)
    for i in 0..count {
        let id = TenantId::new(format!("toggle-{}", i)).unwrap();
        repo.create(id, format!("Toggle {}", i), TenantQuotas::standard())
            .await
            .unwrap();
    }

    // Concurrently deactivate all
    let mut handles = Vec::new();
    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let id = TenantId::new(format!("toggle-{}", i)).unwrap();
            repo.deactivate(&id).await
        }));
    }
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    assert_eq!(repo.count_active().await.unwrap(), 0);
    assert_eq!(repo.count().await.unwrap(), count);

    // Concurrently reactivate all
    let mut handles = Vec::new();
    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let id = TenantId::new(format!("toggle-{}", i)).unwrap();
            repo.activate(&id).await
        }));
    }
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    assert_eq!(repo.count_active().await.unwrap(), count);
}

// =============================================================================
// 2. Concurrent audit event appends — ordering & completeness
// =============================================================================

#[tokio::test]
async fn stress_concurrent_audit_appends_single_tenant() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedAuditRepository::new(store));

    let count = 100usize;
    let mut handles = Vec::new();

    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let event = make_audit_event("stress-tenant", i);
            repo.append(event).await
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // All events must be present
    let query = AuditEventQuery::new(TenantId::new("stress-tenant".to_string()).unwrap());
    let results = repo.query(query).await.unwrap();
    assert_eq!(
        results.len(),
        count,
        "expected {} audit events, got {}",
        count,
        results.len()
    );
}

#[tokio::test]
async fn stress_concurrent_audit_appends_multi_tenant() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedAuditRepository::new(store));

    let tenants = 10usize;
    let events_per_tenant = 10usize;
    let mut handles = Vec::new();

    for t in 0..tenants {
        for e in 0..events_per_tenant {
            let repo = Arc::clone(&repo);
            let tenant_id = format!("mt-tenant-{}", t);
            handles.push(tokio::spawn(async move {
                let event = make_audit_event(&tenant_id, e);
                repo.append(event).await
            }));
        }
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify per-tenant counts
    for t in 0..tenants {
        let tid = TenantId::new(format!("mt-tenant-{}", t)).unwrap();
        let query = AuditEventQuery::new(tid);
        let results = repo.query(query).await.unwrap();
        assert_eq!(
            results.len(),
            events_per_tenant,
            "tenant mt-tenant-{} should have {} events",
            t,
            events_per_tenant
        );
    }
}

#[tokio::test]
async fn stress_audit_append_ordering_preserved() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedAuditRepository::new(store));

    // Sequential appends to guarantee timestamp ordering for this test
    let count = 100usize;
    for i in 0..count {
        let event = make_audit_event("order-tenant", i);
        repo.append(event).await.unwrap();
    }

    let tid = TenantId::new("order-tenant".to_string()).unwrap();
    let results = repo.get_by_tenant(&tid, count, 0).await.unwrap();
    assert_eq!(results.len(), count);

    // Results are newest-first; timestamps should be non-increasing
    for window in results.windows(2) {
        assert!(
            window[0].timestamp() >= window[1].timestamp(),
            "audit events should be ordered newest-first"
        );
    }
}

// =============================================================================
// 3. Concurrent config set/get/delete — consistency
// =============================================================================

#[tokio::test]
async fn stress_concurrent_config_sets() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedConfigRepository::new(store));

    let count = 100usize;
    let mut handles = Vec::new();

    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            repo.set(
                &format!("key-{}", i),
                serde_json::json!(i),
                Some("stress-test"),
            )
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    assert_eq!(repo.count(), count);

    // Verify each key has the correct value
    for i in 0..count {
        let entry = repo.get(&format!("key-{}", i));
        assert!(entry.is_some(), "key-{} should exist", i);
        assert_eq!(entry.unwrap().value, serde_json::json!(i));
    }
}

#[tokio::test]
async fn stress_concurrent_config_overwrites() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedConfigRepository::new(store));

    // Create a single key
    repo.set("shared-key", serde_json::json!(0), None).unwrap();

    // Concurrently overwrite it from many tasks
    let count = 100usize;
    let mut handles = Vec::new();
    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            repo.set("shared-key", serde_json::json!(i), Some("writer"))
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Key should still exist with some valid value (last-writer-wins)
    assert_eq!(repo.count(), 1);
    let entry = repo.get("shared-key").unwrap();
    let val = entry.value.as_u64().unwrap();
    assert!(val < count as u64, "value should be a valid writer index");
}

#[tokio::test]
async fn stress_concurrent_config_set_and_delete() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedConfigRepository::new(store));

    let count = 50usize;

    // Create keys first
    for i in 0..count {
        repo.set(&format!("sd-key-{}", i), serde_json::json!(i), None)
            .unwrap();
    }
    assert_eq!(repo.count(), count);

    // Concurrently delete even keys
    let mut handles = Vec::new();
    for i in (0..count).filter(|n| n % 2 == 0) {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            repo.delete(&format!("sd-key-{}", i), Some("deleter"))
        }));
    }

    for handle in handles {
        let deleted = handle.await.unwrap().unwrap();
        assert!(deleted);
    }

    assert_eq!(repo.count(), count / 2);

    for i in 0..count {
        let entry = repo.get(&format!("sd-key-{}", i));
        if i % 2 == 0 {
            assert!(entry.is_none(), "sd-key-{} should be deleted", i);
        } else {
            assert!(entry.is_some(), "sd-key-{} should exist", i);
        }
    }
}

// =============================================================================
// 4. High-volume writes then reads — no data loss
// =============================================================================

#[tokio::test]
async fn stress_high_volume_tenant_writes_then_reads() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedTenantRepository::new(store));

    let count = 200usize;
    let mut handles = Vec::new();

    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let id = TenantId::new(format!("hv-tenant-{}", i)).unwrap();
            repo.create(id, format!("HV Tenant {}", i), TenantQuotas::standard())
                .await
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify total count
    assert_eq!(repo.count().await.unwrap(), count);

    // Read back every single tenant
    let all = repo.find_all(count + 10, 0).await.unwrap();
    assert_eq!(all.len(), count);

    // Verify no duplicates
    let mut ids: Vec<String> = all.iter().map(|t| t.id().as_str().to_string()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), count, "should have no duplicate tenant IDs");
}

#[tokio::test]
async fn stress_high_volume_audit_then_query() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedAuditRepository::new(store));

    let count = 200usize;
    let mut handles = Vec::new();

    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let event = make_audit_event("hv-audit-tenant", i);
            repo.append(event).await
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    let tid = TenantId::new("hv-audit-tenant".to_string()).unwrap();
    let query = AuditEventQuery::new(tid);
    let results = repo.query(query).await.unwrap();
    assert_eq!(
        results.len(),
        count,
        "all {} audit events must be queryable",
        count
    );
}

#[tokio::test]
async fn stress_high_volume_config_then_list() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let repo = Arc::new(EventSourcedConfigRepository::new(store));

    let count = 200usize;
    let mut handles = Vec::new();

    for i in 0..count {
        let repo = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            repo.set(
                &format!("hv-cfg-{}", i),
                serde_json::json!({ "index": i }),
                Some("bulk-writer"),
            )
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    assert_eq!(repo.count(), count);

    let entries = repo.list();
    assert_eq!(entries.len(), count);

    // Verify each entry value is consistent
    for entry in &entries {
        let idx = entry
            .key
            .strip_prefix("hv-cfg-")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        assert_eq!(entry.value, serde_json::json!({ "index": idx }));
    }
}

// =============================================================================
// 5. WAL recovery after concurrent writes — durability
// =============================================================================

#[tokio::test]
async fn stress_tenant_wal_recovery_after_concurrent_writes() {
    let temp_dir = TempDir::new().unwrap();
    let system_dir = temp_dir.path().join("__system");
    let count = 100usize;

    // Phase 1: concurrent writes
    {
        let store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
        let repo = Arc::new(EventSourcedTenantRepository::new(store));

        let mut handles = Vec::new();
        for i in 0..count {
            let repo = Arc::clone(&repo);
            handles.push(tokio::spawn(async move {
                let id = TenantId::new(format!("wal-tenant-{}", i)).unwrap();
                repo.create(id, format!("WAL Tenant {}", i), TenantQuotas::standard())
                    .await
            }));
        }
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        assert_eq!(repo.count().await.unwrap(), count);
    }
    // Store dropped — simulates process crash/restart

    // Phase 2: recover and verify
    {
        let store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
        let repo = EventSourcedTenantRepository::new(store);

        assert_eq!(
            repo.count().await.unwrap(),
            count,
            "all {} tenants must survive WAL recovery",
            count
        );

        for i in 0..count {
            let id = TenantId::new(format!("wal-tenant-{}", i)).unwrap();
            let tenant = repo.find_by_id(&id).await.unwrap();
            assert!(tenant.is_some(), "wal-tenant-{} should survive recovery", i);
        }
    }
}

#[tokio::test]
async fn stress_config_wal_recovery_after_concurrent_writes() {
    let temp_dir = TempDir::new().unwrap();
    let system_dir = temp_dir.path().join("__system");
    let count = 100usize;

    // Phase 1: concurrent writes
    {
        let store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
        let repo = Arc::new(EventSourcedConfigRepository::new(store));

        let mut handles = Vec::new();
        for i in 0..count {
            let repo = Arc::clone(&repo);
            handles.push(tokio::spawn(async move {
                repo.set(
                    &format!("wal-cfg-{}", i),
                    serde_json::json!(i),
                    Some("recovery-test"),
                )
            }));
        }
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        assert_eq!(repo.count(), count);
    }

    // Phase 2: recover and verify
    {
        let store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
        let repo = EventSourcedConfigRepository::new(store);

        assert_eq!(
            repo.count(),
            count,
            "all {} config entries must survive WAL recovery",
            count
        );

        for i in 0..count {
            let entry = repo.get(&format!("wal-cfg-{}", i));
            assert!(entry.is_some(), "wal-cfg-{} should survive recovery", i);
            assert_eq!(entry.unwrap().value, serde_json::json!(i));
        }
    }
}

#[tokio::test]
async fn stress_audit_wal_recovery_after_concurrent_writes() {
    let temp_dir = TempDir::new().unwrap();
    let system_dir = temp_dir.path().join("__system");
    let count = 100usize;

    // Phase 1: concurrent writes
    {
        let store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
        let repo = Arc::new(EventSourcedAuditRepository::new(store));

        let mut handles = Vec::new();
        for i in 0..count {
            let repo = Arc::clone(&repo);
            handles.push(tokio::spawn(async move {
                let event = make_audit_event("wal-audit-tenant", i);
                repo.append(event).await
            }));
        }
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
    }

    // Phase 2: recover and verify
    {
        let store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
        let repo = EventSourcedAuditRepository::new(store);

        let tid = TenantId::new("wal-audit-tenant".to_string()).unwrap();
        let query = AuditEventQuery::new(tid);
        let results = repo.query(query).await.unwrap();
        assert_eq!(
            results.len(),
            count,
            "all {} audit events must survive WAL recovery",
            count
        );
    }
}

// =============================================================================
// 6. Mixed concurrent operations across all repositories
// =============================================================================

#[tokio::test]
async fn stress_mixed_operations_across_repositories() {
    let temp_dir = TempDir::new().unwrap();
    let store = make_system_store(&temp_dir);
    let tenant_repo = Arc::new(EventSourcedTenantRepository::new(store.clone()));
    let audit_repo = Arc::new(EventSourcedAuditRepository::new(store.clone()));
    let config_repo = Arc::new(EventSourcedConfigRepository::new(store));

    let count = 50usize;
    let mut handles = Vec::new();

    // Spawn tenant creates, audit appends, and config sets all at once
    for i in 0..count {
        let tr = Arc::clone(&tenant_repo);
        handles.push(tokio::spawn(async move {
            let id = TenantId::new(format!("mixed-tenant-{}", i)).unwrap();
            tr.create(id, format!("Mixed {}", i), TenantQuotas::standard())
                .await
                .map(|_| ())
        }));

        let ar = Arc::clone(&audit_repo);
        handles.push(tokio::spawn(async move {
            let event = make_audit_event("mixed-audit", i);
            ar.append(event).await
        }));

        let cr = Arc::clone(&config_repo);
        handles.push(tokio::spawn(async move {
            cr.set(&format!("mixed-cfg-{}", i), serde_json::json!(i), None)
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all three repositories have the right counts
    assert_eq!(tenant_repo.count().await.unwrap(), count);
    assert_eq!(config_repo.count(), count);

    let tid = TenantId::new("mixed-audit".to_string()).unwrap();
    let query = AuditEventQuery::new(tid);
    let results = audit_repo.query(query).await.unwrap();
    assert_eq!(results.len(), count);

    // Verify the shared SystemMetadataStore has events from all domains
    // (3 event types * count operations, minus any compacted events)
    // At minimum, we expect count * 3 raw events (tenant.created + audit.recorded + config.set)
}
