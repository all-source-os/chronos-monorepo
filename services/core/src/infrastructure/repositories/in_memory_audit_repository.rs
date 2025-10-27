/// In-Memory Audit Event Repository
///
/// Thread-safe, in-memory implementation of the AuditEventRepository trait.
/// Suitable for testing, development, and ephemeral audit logging.
///
/// **Design**:
/// - Thread-safe using DashMap
/// - Fast O(1) lookups by ID
/// - In-memory filtering for queries
/// - No persistence (data lost on restart)

use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

use crate::domain::entities::{AuditEvent, AuditEventId};
use crate::domain::repositories::{AuditEventRepository, AuditEventQuery};
use crate::domain::value_objects::TenantId;
use crate::error::Result;
use chrono::{DateTime, Utc};

/// In-memory audit event repository
pub struct InMemoryAuditRepository {
    /// Storage: event_id -> AuditEvent
    events: Arc<DashMap<String, AuditEvent>>,
}

impl InMemoryAuditRepository {
    /// Create a new in-memory audit repository
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
        }
    }

    /// Get all events (for testing/debugging)
    #[cfg(test)]
    pub fn all_events(&self) -> Vec<AuditEvent> {
        self.events.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Clear all events (for testing)
    #[cfg(test)]
    pub fn clear(&self) {
        self.events.clear();
    }

    /// Count all events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if repository is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for InMemoryAuditRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditEventRepository for InMemoryAuditRepository {
    async fn append(&self, event: AuditEvent) -> Result<()> {
        let event_id = event.id().as_str();
        self.events.insert(event_id, event);
        Ok(())
    }

    async fn append_batch(&self, events: Vec<AuditEvent>) -> Result<()> {
        for event in events {
            self.append(event).await?;
        }
        Ok(())
    }

    async fn get_by_id(&self, id: &AuditEventId) -> Result<Option<AuditEvent>> {
        Ok(self.events.get(&id.as_str()).map(|entry| entry.value().clone()))
    }

    async fn query(&self, query: AuditEventQuery) -> Result<Vec<AuditEvent>> {
        let mut results: Vec<AuditEvent> = self
            .events
            .iter()
            .map(|entry| entry.value().clone())
            .filter(|event| {
                // Filter by tenant
                if event.tenant_id() != &query.tenant_id {
                    return false;
                }

                // Filter by time range
                if let Some(start) = query.start_time {
                    if event.timestamp() < &start {
                        return false;
                    }
                }
                if let Some(end) = query.end_time {
                    if event.timestamp() > &end {
                        return false;
                    }
                }

                // Filter by action
                if let Some(ref action) = query.action {
                    if event.action() != action {
                        return false;
                    }
                }

                // Filter by category
                if let Some(ref category) = query.category {
                    if &event.category() != category {
                        return false;
                    }
                }

                // Filter by actor
                if let Some(ref actor_id) = query.actor_identifier {
                    if event.actor().identifier() != *actor_id {
                        return false;
                    }
                }

                // Filter by resource
                if let Some(ref resource_type) = query.resource_type {
                    if event.resource_type() != Some(resource_type.as_str()) {
                        return false;
                    }
                }
                if let Some(ref resource_id) = query.resource_id {
                    if event.resource_id() != Some(resource_id.as_str()) {
                        return false;
                    }
                }

                // Filter security events
                if query.security_events_only && !event.is_security_event() {
                    return false;
                }

                true
            })
            .collect();

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp().cmp(a.timestamp()));

        // Apply pagination
        if let Some(offset) = query.offset {
            results = results.into_iter().skip(offset).collect();
        }
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn count(&self, query: AuditEventQuery) -> Result<usize> {
        let count = self
            .events
            .iter()
            .filter(|entry| {
                let event = entry.value();

                // Filter by tenant
                if event.tenant_id() != &query.tenant_id {
                    return false;
                }

                // Filter by time range
                if let Some(start) = query.start_time {
                    if event.timestamp() < &start {
                        return false;
                    }
                }
                if let Some(end) = query.end_time {
                    if event.timestamp() > &end {
                        return false;
                    }
                }

                // Filter by action
                if let Some(ref action) = query.action {
                    if event.action() != action {
                        return false;
                    }
                }

                // Filter by category
                if let Some(ref category) = query.category {
                    if &event.category() != category {
                        return false;
                    }
                }

                // Filter security events
                if query.security_events_only && !event.is_security_event() {
                    return false;
                }

                true
            })
            .count();

        Ok(count)
    }

    async fn get_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>> {
        let query = AuditEventQuery::new(tenant_id.clone())
            .with_pagination(limit, offset);
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
        older_than: DateTime<Utc>,
    ) -> Result<usize> {
        let to_remove: Vec<String> = self
            .events
            .iter()
            .filter(|entry| {
                let event = entry.value();
                event.tenant_id() == tenant_id && event.timestamp() < &older_than
            })
            .map(|entry| entry.key().clone())
            .collect();

        let count = to_remove.len();
        for key in to_remove {
            self.events.remove(&key);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Actor, AuditAction, AuditOutcome};
    use crate::domain::value_objects::TenantId;

    fn create_test_event(
        tenant_id: TenantId,
        action: AuditAction,
        actor_name: &str,
    ) -> AuditEvent {
        let actor = Actor::user(
            format!("user-{}", actor_name),
            actor_name.to_string(),
        );
        AuditEvent::new(tenant_id, action, actor, AuditOutcome::Success)
    }

    #[tokio::test]
    async fn test_create_repository() {
        let repo = InMemoryAuditRepository::new();
        assert_eq!(repo.len(), 0);
        assert!(repo.is_empty());
    }

    #[tokio::test]
    async fn test_append_event() {
        let repo = InMemoryAuditRepository::new();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        let event = create_test_event(tenant_id.clone(), AuditAction::Login, "john");

        let result = repo.append(event.clone()).await;
        assert!(result.is_ok());
        assert_eq!(repo.len(), 1);

        // Verify we can retrieve it
        let retrieved = repo.get_by_id(event.id()).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), event.id());
    }

    #[tokio::test]
    async fn test_append_batch() {
        let repo = InMemoryAuditRepository::new();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        let events = vec![
            create_test_event(tenant_id.clone(), AuditAction::Login, "john"),
            create_test_event(tenant_id.clone(), AuditAction::EventIngested, "jane"),
            create_test_event(tenant_id.clone(), AuditAction::Logout, "bob"),
        ];

        let result = repo.append_batch(events).await;
        assert!(result.is_ok());
        assert_eq!(repo.len(), 3);
    }

    #[tokio::test]
    async fn test_query_by_tenant() {
        let repo = InMemoryAuditRepository::new();
        let tenant1 = TenantId::new("tenant-1".to_string()).unwrap();
        let tenant2 = TenantId::new("tenant-2".to_string()).unwrap();

        // Add events for two tenants
        repo.append(create_test_event(tenant1.clone(), AuditAction::Login, "john")).await.unwrap();
        repo.append(create_test_event(tenant1.clone(), AuditAction::Logout, "john")).await.unwrap();
        repo.append(create_test_event(tenant2.clone(), AuditAction::Login, "jane")).await.unwrap();

        // Query tenant 1
        let results = repo.get_by_tenant(&tenant1, 10, 0).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.tenant_id() == &tenant1));

        // Query tenant 2
        let results = repo.get_by_tenant(&tenant2, 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tenant_id(), &tenant2);
    }

    #[tokio::test]
    async fn test_query_by_action() {
        let repo = InMemoryAuditRepository::new();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        repo.append(create_test_event(tenant_id.clone(), AuditAction::Login, "john")).await.unwrap();
        repo.append(create_test_event(tenant_id.clone(), AuditAction::Login, "jane")).await.unwrap();
        repo.append(create_test_event(tenant_id.clone(), AuditAction::Logout, "bob")).await.unwrap();

        let query = AuditEventQuery::new(tenant_id.clone())
            .with_action(AuditAction::Login);

        let results = repo.query(query).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.action() == &AuditAction::Login));
    }

    #[tokio::test]
    async fn test_query_by_actor() {
        let repo = InMemoryAuditRepository::new();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        repo.append(create_test_event(tenant_id.clone(), AuditAction::Login, "john")).await.unwrap();
        repo.append(create_test_event(tenant_id.clone(), AuditAction::EventIngested, "john")).await.unwrap();
        repo.append(create_test_event(tenant_id.clone(), AuditAction::Login, "jane")).await.unwrap();

        let results = repo.get_by_actor(&tenant_id, "user:user-john", 10).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.actor().identifier() == "user:user-john"));
    }

    #[tokio::test]
    async fn test_query_security_events() {
        let repo = InMemoryAuditRepository::new();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        repo.append(create_test_event(tenant_id.clone(), AuditAction::Login, "john")).await.unwrap();
        repo.append(create_test_event(tenant_id.clone(), AuditAction::LoginFailed, "john")).await.unwrap();
        repo.append(create_test_event(tenant_id.clone(), AuditAction::PermissionDenied, "jane")).await.unwrap();

        let results = repo.get_security_events(&tenant_id, 10).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.is_security_event()));
    }

    #[tokio::test]
    async fn test_query_with_pagination() {
        let repo = InMemoryAuditRepository::new();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        // Add 10 events
        for i in 0..10 {
            let actor_name = format!("user-{}", i);
            repo.append(create_test_event(tenant_id.clone(), AuditAction::Login, &actor_name)).await.unwrap();
        }

        // Get first page (5 events)
        let page1 = repo.get_by_tenant(&tenant_id, 5, 0).await.unwrap();
        assert_eq!(page1.len(), 5);

        // Get second page (next 5 events)
        let page2 = repo.get_by_tenant(&tenant_id, 5, 5).await.unwrap();
        assert_eq!(page2.len(), 5);

        // Verify no overlap
        let page1_ids: Vec<_> = page1.iter().map(|e| e.id().as_str()).collect();
        let page2_ids: Vec<_> = page2.iter().map(|e| e.id().as_str()).collect();
        assert!(page1_ids.iter().all(|id| !page2_ids.contains(id)));
    }

    #[tokio::test]
    async fn test_count() {
        let repo = InMemoryAuditRepository::new();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        repo.append(create_test_event(tenant_id.clone(), AuditAction::Login, "john")).await.unwrap();
        repo.append(create_test_event(tenant_id.clone(), AuditAction::Login, "jane")).await.unwrap();
        repo.append(create_test_event(tenant_id.clone(), AuditAction::Logout, "bob")).await.unwrap();

        let query = AuditEventQuery::new(tenant_id.clone())
            .with_action(AuditAction::Login);

        let count = repo.count(query).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_purge_old_events() {
        let repo = InMemoryAuditRepository::new();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        // Add some events
        for i in 0..5 {
            let actor_name = format!("user-{}", i);
            repo.append(create_test_event(tenant_id.clone(), AuditAction::Login, &actor_name)).await.unwrap();
        }

        assert_eq!(repo.len(), 5);

        // Purge events older than now (should delete all)
        let now = Utc::now();
        let deleted = repo.purge_old_events(&tenant_id, now).await.unwrap();

        assert_eq!(deleted, 5);
        assert_eq!(repo.len(), 0);
    }

    #[tokio::test]
    async fn test_query_with_resource() {
        let repo = InMemoryAuditRepository::new();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        let event1 = create_test_event(tenant_id.clone(), AuditAction::EventIngested, "john")
            .with_resource("event_stream".to_string(), "stream-1".to_string());

        let event2 = create_test_event(tenant_id.clone(), AuditAction::EventIngested, "jane")
            .with_resource("event_stream".to_string(), "stream-2".to_string());

        repo.append(event1).await.unwrap();
        repo.append(event2).await.unwrap();

        let query = AuditEventQuery::new(tenant_id.clone())
            .with_resource("event_stream".to_string(), "stream-1".to_string());

        let results = repo.query(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource_id(), Some("stream-1"));
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let repo = InMemoryAuditRepository::new();
        let tenant1 = TenantId::new("tenant-1".to_string()).unwrap();
        let tenant2 = TenantId::new("tenant-2".to_string()).unwrap();

        // Add events for both tenants
        repo.append(create_test_event(tenant1.clone(), AuditAction::Login, "john")).await.unwrap();
        repo.append(create_test_event(tenant2.clone(), AuditAction::Login, "jane")).await.unwrap();

        // Query should only return events for requested tenant
        let query1 = AuditEventQuery::new(tenant1.clone());
        let results1 = repo.query(query1).await.unwrap();
        assert_eq!(results1.len(), 1);
        assert_eq!(results1[0].tenant_id(), &tenant1);

        let query2 = AuditEventQuery::new(tenant2.clone());
        let results2 = repo.query(query2).await.unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].tenant_id(), &tenant2);
    }
}
