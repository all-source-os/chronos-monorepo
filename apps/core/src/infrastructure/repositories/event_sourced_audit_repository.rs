use crate::{
    domain::{
        entities::{AuditEvent, AuditEventId},
        repositories::{AuditEventQuery, AuditEventRepository},
        value_objects::{
            TenantId,
            system_stream::{SystemDomain, audit_events, system_entity_id_value},
        },
    },
    error::Result,
    infrastructure::persistence::SystemMetadataStore,
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

/// Event-sourced AuditRepository backed by SystemMetadataStore.
///
/// Audit events are the most natural fit for event sourcing — they are already
/// an append-only log. Each audit event is stored as a system event in the
/// `_system:audit:*` stream with full retention (no compaction).
///
/// # Cache Consistency
///
/// Uses `RwLock<Vec<AuditEvent>>` for the in-memory cache to guarantee temporal
/// ordering of audit entries. The write path is: WAL fsync → cache update, so
/// no acknowledged writes are lost even on crash. On startup, the cache is
/// rebuilt by replaying all audit events from the system store.
///
/// # Event Format
///
/// Each audit event is serialized as the payload of a `_system.audit.recorded`
/// system event. The entity_id is `_system:audit:<tenant_id>` to enable
/// tenant-scoped queries.
pub struct EventSourcedAuditRepository {
    /// Durable storage for system events
    system_store: Arc<SystemMetadataStore>,

    /// In-memory cache of deserialized audit events
    cache: Arc<RwLock<Vec<AuditEvent>>>,
}

impl EventSourcedAuditRepository {
    /// Create a new event-sourced audit repository.
    ///
    /// Replays all existing audit events from the system store.
    pub fn new(system_store: Arc<SystemMetadataStore>) -> Self {
        let cache = Arc::new(RwLock::new(Vec::new()));
        let repo = Self {
            system_store,
            cache,
        };
        repo.rebuild_cache();
        repo
    }

    /// Rebuild the in-memory cache from system store events.
    fn rebuild_cache(&self) {
        let events = self.system_store.read_stream(SystemDomain::Audit);
        let mut cache = self.cache.write();
        for event in events {
            if let Ok(audit_event) = serde_json::from_value::<AuditEvent>(event.payload().clone()) {
                cache.push(audit_event);
            }
        }
        tracing::info!(
            "Rebuilt audit cache: {} audit events from system store",
            cache.len()
        );
    }

    /// Emit an audit event to the system store and append it to the cache.
    ///
    /// The system store append is durable (WAL fsync) before the cache is
    /// updated, ensuring no acknowledged writes are lost.
    fn emit_event(&self, tenant_id_str: &str, audit_event: AuditEvent) -> Result<()> {
        let entity_id = system_entity_id_value(SystemDomain::Audit, tenant_id_str);
        let payload = serde_json::to_value(&audit_event).unwrap_or_default();

        self.system_store
            .append_system_event(audit_events::RECORDED, entity_id, payload, None)?;

        self.cache.write().push(audit_event);
        Ok(())
    }

    /// Apply query filters to a list of audit events.
    fn apply_filters<'a>(
        &self,
        events: impl Iterator<Item = &'a AuditEvent>,
        query: &AuditEventQuery,
    ) -> Vec<AuditEvent> {
        let mut results: Vec<AuditEvent> = events
            .filter(|e| e.tenant_id() == &query.tenant_id)
            .filter(|e| {
                if let Some(start) = query.start_time
                    && *e.timestamp() < start
                {
                    return false;
                }
                if let Some(end) = query.end_time
                    && *e.timestamp() > end
                {
                    return false;
                }
                true
            })
            .filter(|e| {
                if let Some(ref action) = query.action {
                    return e.action() == action;
                }
                true
            })
            .filter(|e| {
                if let Some(ref category) = query.category {
                    return e.category() == *category;
                }
                true
            })
            .filter(|e| {
                if let Some(ref actor_id) = query.actor_identifier {
                    return e.actor().identifier() == *actor_id;
                }
                true
            })
            .filter(|e| {
                if let Some(ref resource_type) = query.resource_type
                    && e.resource_type() != Some(resource_type.as_str())
                {
                    return false;
                }
                if let Some(ref resource_id) = query.resource_id
                    && e.resource_id() != Some(resource_id.as_str())
                {
                    return false;
                }
                true
            })
            .filter(|e| {
                if query.security_events_only {
                    return e.is_security_event();
                }
                true
            })
            .cloned()
            .collect();

        // Sort newest first
        results.sort_by(|a, b| b.timestamp().cmp(a.timestamp()));

        // Apply pagination
        if let Some(offset) = query.offset {
            results = results.into_iter().skip(offset).collect();
        }
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        results
    }
}

#[async_trait]
impl AuditEventRepository for EventSourcedAuditRepository {
    async fn append(&self, event: AuditEvent) -> Result<()> {
        let tenant_id_str = event.tenant_id().as_str().to_string();
        self.emit_event(&tenant_id_str, event)
    }

    async fn append_batch(&self, events: Vec<AuditEvent>) -> Result<()> {
        for event in events {
            self.append(event).await?;
        }
        Ok(())
    }

    async fn get_by_id(&self, id: &AuditEventId) -> Result<Option<AuditEvent>> {
        let cache = self.cache.read();
        Ok(cache.iter().find(|e| e.id() == id).cloned())
    }

    async fn query(&self, query: AuditEventQuery) -> Result<Vec<AuditEvent>> {
        let cache = self.cache.read();
        Ok(self.apply_filters(cache.iter(), &query))
    }

    async fn count(&self, query: AuditEventQuery) -> Result<usize> {
        let cache = self.cache.read();
        Ok(self.apply_filters(cache.iter(), &query).len())
    }

    async fn get_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>> {
        let query = AuditEventQuery::new(tenant_id.clone()).with_pagination(limit, offset);
        self.query(query).await
    }

    async fn get_security_events(
        &self,
        tenant_id: &TenantId,
        limit: usize,
    ) -> Result<Vec<AuditEvent>> {
        let query = AuditEventQuery::new(tenant_id.clone())
            .security_only()
            .with_pagination(limit, 0);
        self.query(query).await
    }

    async fn get_by_actor(
        &self,
        tenant_id: &TenantId,
        actor_identifier: &str,
        limit: usize,
    ) -> Result<Vec<AuditEvent>> {
        let query = AuditEventQuery::new(tenant_id.clone())
            .with_actor(actor_identifier.to_string())
            .with_pagination(limit, 0);
        self.query(query).await
    }

    async fn purge_old_events(
        &self,
        tenant_id: &TenantId,
        older_than: chrono::DateTime<chrono::Utc>,
    ) -> Result<usize> {
        let mut cache = self.cache.write();
        let before_len = cache.len();
        cache.retain(|e| !(e.tenant_id() == tenant_id && *e.timestamp() < older_than));
        let purged = before_len - cache.len();
        Ok(purged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::entities::{Actor, AuditAction, AuditOutcome},
        infrastructure::persistence::SystemMetadataStore,
    };
    use tempfile::TempDir;

    fn create_test_repo() -> (EventSourcedAuditRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let system_store =
            Arc::new(SystemMetadataStore::new(temp_dir.path().join("__system")).unwrap());
        let repo = EventSourcedAuditRepository::new(system_store);
        (repo, temp_dir)
    }

    fn test_audit_event(tenant_id: &str) -> AuditEvent {
        AuditEvent::new(
            TenantId::new(tenant_id.to_string()).unwrap(),
            AuditAction::EventIngested,
            Actor::system("test".to_string()),
            AuditOutcome::Success,
        )
    }

    #[tokio::test]
    async fn test_append_and_query() {
        let (repo, _dir) = create_test_repo();

        let event = test_audit_event("tenant-1");
        repo.append(event.clone()).await.unwrap();

        let query = AuditEventQuery::new(TenantId::new("tenant-1".to_string()).unwrap());
        let results = repo.query(query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_append_batch() {
        let (repo, _dir) = create_test_repo();

        let events = vec![
            test_audit_event("tenant-1"),
            test_audit_event("tenant-1"),
            test_audit_event("tenant-2"),
        ];
        repo.append_batch(events).await.unwrap();

        let query = AuditEventQuery::new(TenantId::new("tenant-1".to_string()).unwrap());
        let results = repo.query(query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_tenant() {
        let (repo, _dir) = create_test_repo();

        for _ in 0..5 {
            repo.append(test_audit_event("tenant-1")).await.unwrap();
        }

        let tenant_id = TenantId::new("tenant-1".to_string()).unwrap();
        let results = repo.get_by_tenant(&tenant_id, 3, 0).await.unwrap();
        assert_eq!(results.len(), 3);

        let results = repo.get_by_tenant(&tenant_id, 10, 3).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_security_events() {
        let (repo, _dir) = create_test_repo();

        let tenant_id = TenantId::new("tenant-1".to_string()).unwrap();

        // Normal event
        repo.append(test_audit_event("tenant-1")).await.unwrap();

        // Security event
        let security_event = AuditEvent::new(
            tenant_id.clone(),
            AuditAction::LoginFailed,
            Actor::user("u1".to_string(), "john".to_string()),
            AuditOutcome::Failure,
        );
        repo.append(security_event).await.unwrap();

        let results = repo.get_security_events(&tenant_id, 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_security_event());
    }

    #[tokio::test]
    async fn test_get_by_actor() {
        let (repo, _dir) = create_test_repo();

        let tenant_id = TenantId::new("tenant-1".to_string()).unwrap();

        let event1 = AuditEvent::new(
            tenant_id.clone(),
            AuditAction::Login,
            Actor::user("u1".to_string(), "alice".to_string()),
            AuditOutcome::Success,
        );
        let event2 = AuditEvent::new(
            tenant_id.clone(),
            AuditAction::Login,
            Actor::user("u2".to_string(), "bob".to_string()),
            AuditOutcome::Success,
        );

        repo.append(event1).await.unwrap();
        repo.append(event2).await.unwrap();

        let results = repo.get_by_actor(&tenant_id, "user:u1", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_wal_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let system_dir = temp_dir.path().join("__system");

        // Write events
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedAuditRepository::new(system_store);

            repo.append(test_audit_event("tenant-1")).await.unwrap();
            repo.append(test_audit_event("tenant-1")).await.unwrap();
        }

        // Recover
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedAuditRepository::new(system_store);

            let query = AuditEventQuery::new(TenantId::new("tenant-1".to_string()).unwrap());
            let results = repo.query(query).await.unwrap();
            assert_eq!(results.len(), 2);
        }
    }

    #[tokio::test]
    async fn test_count() {
        let (repo, _dir) = create_test_repo();

        for _ in 0..3 {
            repo.append(test_audit_event("tenant-1")).await.unwrap();
        }

        let query = AuditEventQuery::new(TenantId::new("tenant-1".to_string()).unwrap());
        let count = repo.count(query).await.unwrap();
        assert_eq!(count, 3);
    }
}
