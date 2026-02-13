use crate::{
    domain::{
        entities::{Creator, CreatorStatus, CreatorTier},
        value_objects::{CreatorId, TenantId, WalletAddress},
    },
    error::Result,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Repository trait for creator management
///
/// Provides persistent storage and retrieval operations for content creators.
///
/// # Responsibilities
/// - CRUD operations for creators
/// - Creator status management
/// - Creator lookup by various fields
/// - Revenue tracking
///
/// # Thread Safety
/// Implementations must be thread-safe (Send + Sync).
#[async_trait]
pub trait CreatorRepository: Send + Sync {
    /// Create a new creator
    ///
    /// # Arguments
    /// * `creator` - The creator to create
    ///
    /// # Errors
    /// - `ValidationError` - If a creator with this email already exists
    /// - `StorageError` - If the operation fails
    async fn create(&self, creator: Creator) -> Result<Creator>;

    /// Save or update a creator
    ///
    /// If the creator doesn't exist, it will be created.
    /// If it exists, it will be updated.
    ///
    /// # Arguments
    /// * `creator` - The creator to save
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn save(&self, creator: &Creator) -> Result<()>;

    /// Find a creator by ID
    ///
    /// # Arguments
    /// * `id` - The creator ID to search for
    ///
    /// # Returns
    /// `Some(Creator)` if found, `None` otherwise
    async fn find_by_id(&self, id: &CreatorId) -> Result<Option<Creator>>;

    /// Find a creator by email
    ///
    /// # Arguments
    /// * `email` - The email address to search for
    ///
    /// # Returns
    /// `Some(Creator)` if found, `None` otherwise
    async fn find_by_email(&self, email: &str) -> Result<Option<Creator>>;

    /// Find a creator by wallet address
    ///
    /// # Arguments
    /// * `wallet` - The wallet address to search for
    ///
    /// # Returns
    /// `Some(Creator)` if found, `None` otherwise
    async fn find_by_wallet(&self, wallet: &WalletAddress) -> Result<Option<Creator>>;

    /// Find creators by tenant
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant ID
    /// * `limit` - Maximum number of creators to return
    /// * `offset` - Number of creators to skip
    ///
    /// # Returns
    /// Vector of creators in this tenant
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Creator>>;

    /// Find active creators
    ///
    /// # Arguments
    /// * `limit` - Maximum number of creators to return
    /// * `offset` - Number of creators to skip
    ///
    /// # Returns
    /// Vector of active creators
    async fn find_active(&self, limit: usize, offset: usize) -> Result<Vec<Creator>>;

    /// Count total number of creators
    async fn count(&self) -> Result<usize>;

    /// Count creators by status
    async fn count_by_status(&self, status: CreatorStatus) -> Result<usize>;

    /// Delete a creator
    ///
    /// # Warning
    /// This is a destructive operation. Consider deactivating instead.
    ///
    /// # Arguments
    /// * `id` - The creator ID to delete
    ///
    /// # Returns
    /// `true` if the creator was deleted, `false` if it didn't exist
    async fn delete(&self, id: &CreatorId) -> Result<bool>;

    /// Check if a creator exists
    async fn exists(&self, id: &CreatorId) -> Result<bool> {
        Ok(self.find_by_id(id).await?.is_some())
    }

    /// Check if an email is already registered
    async fn email_exists(&self, email: &str) -> Result<bool> {
        Ok(self.find_by_email(email).await?.is_some())
    }

    /// Query creators with filters
    async fn query(&self, query: &CreatorQuery) -> Result<Vec<Creator>>;
}

/// Query filter for finding creators
#[derive(Debug, Clone, Default)]
pub struct CreatorQuery {
    pub tenant_id: Option<TenantId>,
    pub status: Option<CreatorStatus>,
    pub tier: Option<CreatorTier>,
    pub email_verified: Option<bool>,
    pub email_contains: Option<String>,
    pub name_contains: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub min_revenue_cents: Option<u64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl CreatorQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_tenant(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    pub fn with_status(mut self, status: CreatorStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_tier(mut self, tier: CreatorTier) -> Self {
        self.tier = Some(tier);
        self
    }

    pub fn verified_only(mut self) -> Self {
        self.email_verified = Some(true);
        self
    }

    pub fn with_email_filter(mut self, email: String) -> Self {
        self.email_contains = Some(email);
        self
    }

    pub fn with_name_filter(mut self, name: String) -> Self {
        self.name_contains = Some(name);
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

    pub fn with_min_revenue(mut self, min_cents: u64) -> Self {
        self.min_revenue_cents = Some(min_cents);
        self
    }

    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creator_query_builder() {
        let query = CreatorQuery::new()
            .with_status(CreatorStatus::Active)
            .verified_only()
            .with_pagination(10, 0);

        assert_eq!(query.status, Some(CreatorStatus::Active));
        assert_eq!(query.email_verified, Some(true));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_creator_query_with_tenant() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        let query = CreatorQuery::new()
            .for_tenant(tenant_id.clone())
            .with_tier(CreatorTier::Pro);

        assert!(query.tenant_id.is_some());
        assert_eq!(query.tier, Some(CreatorTier::Pro));
    }

    #[test]
    fn test_creator_query_with_dates() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);

        let query = CreatorQuery::new()
            .created_after(yesterday)
            .created_before(now);

        assert!(query.created_after.is_some());
        assert!(query.created_before.is_some());
    }
}
