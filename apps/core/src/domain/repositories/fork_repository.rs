use crate::{
    domain::{
        entities::{EventStoreFork, ForkStatus},
        value_objects::{ForkId, TenantId},
    },
    error::Result,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Repository trait for event store fork management
///
/// Provides persistent storage and retrieval operations for event store forks.
/// Forks enable the Agentic Postgres pattern for AI agent experimentation.
///
/// # Responsibilities
/// - CRUD operations for forks
/// - Fork status management
/// - Fork cleanup (expired, orphaned)
/// - Fork hierarchy queries
///
/// # Thread Safety
/// Implementations must be thread-safe (Send + Sync).
#[async_trait]
pub trait ForkRepository: Send + Sync {
    /// Create a new fork
    ///
    /// # Arguments
    /// * `fork` - The fork to create
    ///
    /// # Errors
    /// - `ValidationError` - If a fork with this ID already exists
    /// - `StorageError` - If the operation fails
    async fn create(&self, fork: EventStoreFork) -> Result<EventStoreFork>;

    /// Save or update a fork
    ///
    /// If the fork doesn't exist, it will be created.
    /// If it exists, it will be updated.
    ///
    /// # Arguments
    /// * `fork` - The fork to save
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn save(&self, fork: &EventStoreFork) -> Result<()>;

    /// Find a fork by ID
    ///
    /// # Arguments
    /// * `id` - The fork ID to search for
    ///
    /// # Returns
    /// `Some(Fork)` if found, `None` otherwise
    async fn find_by_id(&self, id: &ForkId) -> Result<Option<EventStoreFork>>;

    /// Find a fork by name within a tenant
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant ID
    /// * `name` - The fork name
    ///
    /// # Returns
    /// `Some(Fork)` if found, `None` otherwise
    async fn find_by_name(
        &self,
        tenant_id: &TenantId,
        name: &str,
    ) -> Result<Option<EventStoreFork>>;

    /// Find forks by tenant
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant ID
    /// * `limit` - Maximum number of forks to return
    /// * `offset` - Number of forks to skip
    ///
    /// # Returns
    /// Vector of forks in this tenant
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventStoreFork>>;

    /// Find child forks of a parent fork
    ///
    /// # Arguments
    /// * `parent_id` - The parent fork ID
    ///
    /// # Returns
    /// Vector of child forks
    async fn find_children(&self, parent_id: &ForkId) -> Result<Vec<EventStoreFork>>;

    /// Find active forks
    ///
    /// # Arguments
    /// * `tenant_id` - Optional tenant filter
    /// * `limit` - Maximum number of forks to return
    /// * `offset` - Number of forks to skip
    ///
    /// # Returns
    /// Vector of active forks
    async fn find_active(
        &self,
        tenant_id: Option<&TenantId>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventStoreFork>>;

    /// Find forks by status
    ///
    /// # Arguments
    /// * `status` - The status to filter by
    /// * `limit` - Maximum number of forks to return
    /// * `offset` - Number of forks to skip
    ///
    /// # Returns
    /// Vector of forks with the given status
    async fn find_by_status(
        &self,
        status: ForkStatus,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventStoreFork>>;

    /// Find expired forks (for cleanup)
    ///
    /// # Arguments
    /// * `before` - Find forks expired before this time
    /// * `limit` - Maximum number of forks to return
    ///
    /// # Returns
    /// Vector of expired forks
    async fn find_expired(
        &self,
        before: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<EventStoreFork>>;

    /// Find forks created by a specific agent
    ///
    /// # Arguments
    /// * `agent_id` - The agent ID
    /// * `limit` - Maximum number of forks to return
    /// * `offset` - Number of forks to skip
    ///
    /// # Returns
    /// Vector of forks created by the agent
    async fn find_by_agent(
        &self,
        agent_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<EventStoreFork>>;

    /// Count total number of forks
    async fn count(&self) -> Result<usize>;

    /// Count forks by tenant
    async fn count_by_tenant(&self, tenant_id: &TenantId) -> Result<usize>;

    /// Count forks by status
    async fn count_by_status(&self, status: ForkStatus) -> Result<usize>;

    /// Delete a fork
    ///
    /// # Warning
    /// This permanently removes the fork and all its events.
    /// Consider using `discard()` on the fork entity instead for soft delete.
    ///
    /// # Arguments
    /// * `id` - The fork ID to delete
    ///
    /// # Returns
    /// `true` if the fork was deleted, `false` if it didn't exist
    async fn delete(&self, id: &ForkId) -> Result<bool>;

    /// Delete all expired forks (batch cleanup)
    ///
    /// # Arguments
    /// * `before` - Delete forks expired before this time
    ///
    /// # Returns
    /// Number of forks deleted
    async fn delete_expired(&self, before: DateTime<Utc>) -> Result<usize>;

    /// Check if a fork exists
    async fn exists(&self, id: &ForkId) -> Result<bool> {
        Ok(self.find_by_id(id).await?.is_some())
    }

    /// Check if a fork name is already taken within a tenant
    async fn name_exists(&self, tenant_id: &TenantId, name: &str) -> Result<bool> {
        Ok(self.find_by_name(tenant_id, name).await?.is_some())
    }

    /// Query forks with filters
    async fn query(&self, query: &ForkQuery) -> Result<Vec<EventStoreFork>>;
}

/// Query filter for finding forks
#[derive(Debug, Clone, Default)]
pub struct ForkQuery {
    pub tenant_id: Option<TenantId>,
    pub status: Option<ForkStatus>,
    pub parent_fork_id: Option<ForkId>,
    pub created_by_agent: Option<String>,
    pub name_contains: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub expires_before: Option<DateTime<Utc>>,
    pub min_event_count: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl ForkQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_tenant(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    pub fn with_status(mut self, status: ForkStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_parent(mut self, parent_id: ForkId) -> Self {
        self.parent_fork_id = Some(parent_id);
        self
    }

    pub fn created_by(mut self, agent_id: String) -> Self {
        self.created_by_agent = Some(agent_id);
        self
    }

    pub fn name_contains(mut self, pattern: String) -> Self {
        self.name_contains = Some(pattern);
        self
    }

    pub fn created_after(mut self, date: DateTime<Utc>) -> Self {
        self.created_after = Some(date);
        self
    }

    pub fn created_before(mut self, date: DateTime<Utc>) -> Self {
        self.created_before = Some(date);
        self
    }

    pub fn expires_before(mut self, date: DateTime<Utc>) -> Self {
        self.expires_before = Some(date);
        self
    }

    pub fn with_min_events(mut self, count: u64) -> Self {
        self.min_event_count = Some(count);
        self
    }

    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }

    /// Get only active forks
    pub fn active_only(mut self) -> Self {
        self.status = Some(ForkStatus::Active);
        self
    }

    /// Get only root forks (no parent)
    pub fn root_only(self) -> Self {
        // Note: This requires special handling in the repository implementation
        // to filter where parent_fork_id IS NULL
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fork_query_builder() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        let query = ForkQuery::new()
            .for_tenant(tenant_id)
            .with_status(ForkStatus::Active)
            .with_pagination(10, 0);

        assert!(query.tenant_id.is_some());
        assert_eq!(query.status, Some(ForkStatus::Active));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_fork_query_active_only() {
        let query = ForkQuery::new().active_only();
        assert_eq!(query.status, Some(ForkStatus::Active));
    }

    #[test]
    fn test_fork_query_created_by() {
        let query = ForkQuery::new().created_by("agent-123".to_string());
        assert_eq!(query.created_by_agent, Some("agent-123".to_string()));
    }

    #[test]
    fn test_fork_query_with_dates() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);

        let query = ForkQuery::new()
            .created_after(yesterday)
            .created_before(now);

        assert!(query.created_after.is_some());
        assert!(query.created_before.is_some());
    }

    #[test]
    fn test_fork_query_with_parent() {
        let parent_id = ForkId::new();
        let query = ForkQuery::new().with_parent(parent_id.clone());
        assert_eq!(query.parent_fork_id, Some(parent_id));
    }
}
