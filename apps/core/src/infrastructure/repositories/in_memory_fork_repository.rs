use crate::domain::entities::{EventStoreFork, ForkStatus};
use crate::domain::repositories::{ForkQuery, ForkRepository};
use crate::domain::value_objects::{ForkId, TenantId};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory implementation of ForkRepository
///
/// Suitable for:
/// - Development and testing
/// - Single-node deployments
/// - Caching layer for persistent backends
///
/// Thread-safe using RwLock for concurrent access.
///
/// # Example
/// ```
/// use allsource_core::infrastructure::repositories::InMemoryForkRepository;
///
/// let repo = InMemoryForkRepository::new();
/// // Use repo with fork use cases...
/// ```
pub struct InMemoryForkRepository {
    forks: RwLock<HashMap<String, EventStoreFork>>,
}

impl InMemoryForkRepository {
    /// Create a new empty in-memory fork repository
    pub fn new() -> Self {
        Self {
            forks: RwLock::new(HashMap::new()),
        }
    }

    /// Get the number of forks stored
    pub fn len(&self) -> usize {
        self.forks.read().unwrap().len()
    }

    /// Check if the repository is empty
    pub fn is_empty(&self) -> bool {
        self.forks.read().unwrap().is_empty()
    }

    /// Clear all forks (for testing)
    pub fn clear(&self) {
        self.forks.write().unwrap().clear();
    }
}

impl Default for InMemoryForkRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ForkRepository for InMemoryForkRepository {
    async fn create(&self, fork: EventStoreFork) -> Result<EventStoreFork> {
        let mut forks = self.forks.write().unwrap();
        let id = fork.id().as_str();

        if forks.contains_key(&id) {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Fork with ID '{}' already exists",
                id
            )));
        }

        forks.insert(id.clone(), fork.clone());
        Ok(fork)
    }

    async fn save(&self, fork: &EventStoreFork) -> Result<()> {
        let mut forks = self.forks.write().unwrap();
        forks.insert(fork.id().as_str(), fork.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &ForkId) -> Result<Option<EventStoreFork>> {
        let forks = self.forks.read().unwrap();
        Ok(forks.get(&id.as_str()).cloned())
    }

    async fn find_by_name(&self, tenant_id: &TenantId, name: &str) -> Result<Option<EventStoreFork>> {
        let forks = self.forks.read().unwrap();
        Ok(forks
            .values()
            .find(|f| f.tenant_id() == tenant_id && f.name() == name)
            .cloned())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventStoreFork>> {
        let forks = self.forks.read().unwrap();
        Ok(forks
            .values()
            .filter(|f| f.tenant_id() == tenant_id)
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn find_children(&self, parent_id: &ForkId) -> Result<Vec<EventStoreFork>> {
        let forks = self.forks.read().unwrap();
        Ok(forks
            .values()
            .filter(|f| f.parent_fork_id() == Some(parent_id))
            .cloned()
            .collect())
    }

    async fn find_active(
        &self,
        tenant_id: Option<&TenantId>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventStoreFork>> {
        let forks = self.forks.read().unwrap();
        let mut result: Vec<_> = forks
            .values()
            .filter(|f| f.is_active())
            .filter(|f| tenant_id.is_none() || f.tenant_id() == tenant_id.unwrap())
            .cloned()
            .collect();

        // Sort by created_at descending
        result.sort_by(|a, b| b.created_at().cmp(&a.created_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_by_status(
        &self,
        status: ForkStatus,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventStoreFork>> {
        let forks = self.forks.read().unwrap();
        Ok(forks
            .values()
            .filter(|f| f.status() == status)
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn find_expired(&self, before: DateTime<Utc>, limit: usize) -> Result<Vec<EventStoreFork>> {
        let forks = self.forks.read().unwrap();
        Ok(forks
            .values()
            .filter(|f| f.expires_at() < before && f.is_active())
            .take(limit)
            .cloned()
            .collect())
    }

    async fn find_by_agent(
        &self,
        agent_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventStoreFork>> {
        let forks = self.forks.read().unwrap();
        Ok(forks
            .values()
            .filter(|f| f.created_by_agent() == Some(agent_id))
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.forks.read().unwrap().len())
    }

    async fn count_by_tenant(&self, tenant_id: &TenantId) -> Result<usize> {
        let forks = self.forks.read().unwrap();
        Ok(forks.values().filter(|f| f.tenant_id() == tenant_id).count())
    }

    async fn count_by_status(&self, status: ForkStatus) -> Result<usize> {
        let forks = self.forks.read().unwrap();
        Ok(forks.values().filter(|f| f.status() == status).count())
    }

    async fn delete(&self, id: &ForkId) -> Result<bool> {
        let mut forks = self.forks.write().unwrap();
        Ok(forks.remove(&id.as_str()).is_some())
    }

    async fn delete_expired(&self, before: DateTime<Utc>) -> Result<usize> {
        let mut forks = self.forks.write().unwrap();
        let to_delete: Vec<_> = forks
            .values()
            .filter(|f| f.expires_at() < before)
            .map(|f| f.id().as_str())
            .collect();

        let count = to_delete.len();
        for id in to_delete {
            forks.remove(&id);
        }

        Ok(count)
    }

    async fn query(&self, query: &ForkQuery) -> Result<Vec<EventStoreFork>> {
        let forks = self.forks.read().unwrap();
        let mut result: Vec<_> = forks.values().cloned().collect();

        // Apply filters
        if let Some(ref tenant_id) = query.tenant_id {
            result.retain(|f| f.tenant_id() == tenant_id);
        }

        if let Some(status) = query.status {
            result.retain(|f| f.status() == status);
        }

        if let Some(ref parent_id) = query.parent_fork_id {
            result.retain(|f| f.parent_fork_id() == Some(parent_id));
        }

        if let Some(ref agent_id) = query.created_by_agent {
            result.retain(|f| f.created_by_agent() == Some(agent_id.as_str()));
        }

        if let Some(ref pattern) = query.name_contains {
            result.retain(|f| f.name().contains(pattern));
        }

        if let Some(date) = query.created_after {
            result.retain(|f| f.created_at() > date);
        }

        if let Some(date) = query.created_before {
            result.retain(|f| f.created_at() < date);
        }

        if let Some(date) = query.expires_before {
            result.retain(|f| f.expires_at() < date);
        }

        if let Some(min_count) = query.min_event_count {
            result.retain(|f| f.event_count() >= min_count);
        }

        // Sort by created_at descending
        result.sort_by(|a, b| b.created_at().cmp(&a.created_at()));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::EventStoreFork;

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn create_test_fork(name: &str) -> EventStoreFork {
        EventStoreFork::new(test_tenant_id(), name.to_string(), 0).unwrap()
    }

    #[tokio::test]
    async fn test_create_fork() {
        let repo = InMemoryForkRepository::new();
        let fork = create_test_fork("test-fork");

        let result = repo.create(fork).await;
        assert!(result.is_ok());
        assert_eq!(repo.len(), 1);
    }

    #[tokio::test]
    async fn test_create_duplicate_fork() {
        let repo = InMemoryForkRepository::new();
        let fork1 = create_test_fork("test-fork");
        let fork1_id = fork1.id().clone();

        repo.create(fork1).await.unwrap();

        // Try to create fork with same ID
        let fork2 = EventStoreFork::reconstruct(
            fork1_id,
            test_tenant_id(),
            "different-name".to_string(),
            None,
            None,
            0,
            ForkStatus::Active,
            crate::domain::entities::ForkIsolationLevel::ReadParent,
            std::collections::HashMap::new(),
            0,
            Utc::now() + chrono::Duration::hours(24),
            None,
            Utc::now(),
            Utc::now(),
            serde_json::json!({}),
        );

        let result = repo.create(fork2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let repo = InMemoryForkRepository::new();
        let fork = create_test_fork("test-fork");
        let fork_id = fork.id().clone();

        repo.create(fork).await.unwrap();

        let found = repo.find_by_id(&fork_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "test-fork");
    }

    #[tokio::test]
    async fn test_find_by_name() {
        let repo = InMemoryForkRepository::new();
        let fork = create_test_fork("my-unique-fork");

        repo.create(fork).await.unwrap();

        let found = repo.find_by_name(&test_tenant_id(), "my-unique-fork").await.unwrap();
        assert!(found.is_some());

        let not_found = repo.find_by_name(&test_tenant_id(), "non-existent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_tenant() {
        let repo = InMemoryForkRepository::new();

        // Create forks for test-tenant
        for i in 0..3 {
            let fork = create_test_fork(&format!("fork-{}", i));
            repo.create(fork).await.unwrap();
        }

        // Create fork for different tenant
        let other_tenant = TenantId::new("other-tenant".to_string()).unwrap();
        let other_fork = EventStoreFork::new(other_tenant, "other-fork".to_string(), 0).unwrap();
        repo.create(other_fork).await.unwrap();

        let forks = repo.find_by_tenant(&test_tenant_id(), 100, 0).await.unwrap();
        assert_eq!(forks.len(), 3);
    }

    #[tokio::test]
    async fn test_find_active() {
        let repo = InMemoryForkRepository::new();

        // Create active fork
        let active_fork = create_test_fork("active-fork");
        repo.create(active_fork).await.unwrap();

        // Create and discard a fork
        let mut discarded_fork = create_test_fork("discarded-fork");
        let discarded_id = discarded_fork.id().clone();
        repo.create(discarded_fork.clone()).await.unwrap();
        discarded_fork.discard().unwrap();
        repo.save(&discarded_fork).await.unwrap();

        let active = repo.find_active(None, 100, 0).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name(), "active-fork");
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryForkRepository::new();
        let fork = create_test_fork("test-fork");
        let fork_id = fork.id().clone();

        repo.create(fork).await.unwrap();
        assert_eq!(repo.len(), 1);

        let deleted = repo.delete(&fork_id).await.unwrap();
        assert!(deleted);
        assert_eq!(repo.len(), 0);

        // Delete non-existent
        let deleted_again = repo.delete(&fork_id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_query_with_filters() {
        let repo = InMemoryForkRepository::new();

        // Create forks with different attributes
        let mut fork1 = create_test_fork("agent-fork-1");
        fork1.set_created_by_agent("agent-123".to_string());
        repo.create(fork1).await.unwrap();

        let mut fork2 = create_test_fork("agent-fork-2");
        fork2.set_created_by_agent("agent-456".to_string());
        repo.create(fork2).await.unwrap();

        let fork3 = create_test_fork("no-agent-fork");
        repo.create(fork3).await.unwrap();

        // Query by agent
        let query = ForkQuery::new().created_by("agent-123".to_string());
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), "agent-fork-1");
    }

    #[tokio::test]
    async fn test_query_with_pagination() {
        let repo = InMemoryForkRepository::new();

        for i in 0..5 {
            let fork = create_test_fork(&format!("fork-{}", i));
            repo.create(fork).await.unwrap();
        }

        let query = ForkQuery::new().with_pagination(2, 1);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_count_methods() {
        let repo = InMemoryForkRepository::new();

        for i in 0..3 {
            let fork = create_test_fork(&format!("fork-{}", i));
            repo.create(fork).await.unwrap();
        }

        assert_eq!(repo.count().await.unwrap(), 3);
        assert_eq!(repo.count_by_tenant(&test_tenant_id()).await.unwrap(), 3);
        assert_eq!(repo.count_by_status(ForkStatus::Active).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_find_children() {
        let repo = InMemoryForkRepository::new();

        // Create parent
        let parent = create_test_fork("parent-fork");
        let parent_id = parent.id().clone();
        repo.create(parent.clone()).await.unwrap();

        // Create children
        let child1 = EventStoreFork::branch_from(&parent, "child-1".to_string()).unwrap();
        let child2 = EventStoreFork::branch_from(&parent, "child-2".to_string()).unwrap();
        repo.create(child1).await.unwrap();
        repo.create(child2).await.unwrap();

        // Create unrelated fork
        let other = create_test_fork("other-fork");
        repo.create(other).await.unwrap();

        let children = repo.find_children(&parent_id).await.unwrap();
        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn test_clear() {
        let repo = InMemoryForkRepository::new();

        for i in 0..3 {
            let fork = create_test_fork(&format!("fork-{}", i));
            repo.create(fork).await.unwrap();
        }

        assert_eq!(repo.len(), 3);
        repo.clear();
        assert!(repo.is_empty());
    }
}
