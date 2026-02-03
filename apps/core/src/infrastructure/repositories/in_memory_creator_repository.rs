use crate::domain::entities::{Creator, CreatorStatus};
use crate::domain::repositories::{CreatorQuery, CreatorRepository};
use crate::domain::value_objects::{CreatorId, TenantId, WalletAddress};
use crate::error::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory implementation of CreatorRepository
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
/// use allsource_core::infrastructure::repositories::InMemoryCreatorRepository;
///
/// let repo = InMemoryCreatorRepository::new();
/// // Use repo with creator use cases...
/// ```
pub struct InMemoryCreatorRepository {
    creators: RwLock<HashMap<String, Creator>>,
}

impl InMemoryCreatorRepository {
    /// Create a new empty in-memory creator repository
    pub fn new() -> Self {
        Self {
            creators: RwLock::new(HashMap::new()),
        }
    }

    /// Get the number of creators stored
    pub fn len(&self) -> usize {
        self.creators.read().unwrap().len()
    }

    /// Check if the repository is empty
    pub fn is_empty(&self) -> bool {
        self.creators.read().unwrap().is_empty()
    }

    /// Clear all creators (for testing)
    pub fn clear(&self) {
        self.creators.write().unwrap().clear();
    }
}

impl Default for InMemoryCreatorRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CreatorRepository for InMemoryCreatorRepository {
    async fn create(&self, creator: Creator) -> Result<Creator> {
        let mut creators = self.creators.write().unwrap();
        let id = creator.id().to_string();

        // Check for duplicate ID
        if creators.contains_key(&id) {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Creator with ID '{}' already exists",
                id
            )));
        }

        // Check for duplicate email
        let email = creator.email();
        if creators.values().any(|c| c.email() == email) {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Creator with email '{}' already exists",
                email
            )));
        }

        creators.insert(id, creator.clone());
        Ok(creator)
    }

    async fn save(&self, creator: &Creator) -> Result<()> {
        let mut creators = self.creators.write().unwrap();
        creators.insert(creator.id().to_string(), creator.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &CreatorId) -> Result<Option<Creator>> {
        let creators = self.creators.read().unwrap();
        Ok(creators.get(&id.to_string()).cloned())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Creator>> {
        let creators = self.creators.read().unwrap();
        Ok(creators.values().find(|c| c.email() == email).cloned())
    }

    async fn find_by_wallet(&self, wallet: &WalletAddress) -> Result<Option<Creator>> {
        let creators = self.creators.read().unwrap();
        Ok(creators
            .values()
            .find(|c| c.wallet_address() == wallet)
            .cloned())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Creator>> {
        let creators = self.creators.read().unwrap();
        Ok(creators
            .values()
            .filter(|c| c.tenant_id() == tenant_id)
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn find_active(&self, limit: usize, offset: usize) -> Result<Vec<Creator>> {
        let creators = self.creators.read().unwrap();
        Ok(creators
            .values()
            .filter(|c| c.is_active())
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.creators.read().unwrap().len())
    }

    async fn count_by_status(&self, status: CreatorStatus) -> Result<usize> {
        let creators = self.creators.read().unwrap();
        Ok(creators.values().filter(|c| c.status() == status).count())
    }

    async fn delete(&self, id: &CreatorId) -> Result<bool> {
        let mut creators = self.creators.write().unwrap();
        Ok(creators.remove(&id.to_string()).is_some())
    }

    async fn query(&self, query: &CreatorQuery) -> Result<Vec<Creator>> {
        let creators = self.creators.read().unwrap();
        let mut result: Vec<_> = creators.values().cloned().collect();

        // Apply filters
        if let Some(ref tenant_id) = query.tenant_id {
            result.retain(|c| c.tenant_id() == tenant_id);
        }

        if let Some(status) = query.status {
            result.retain(|c| c.status() == status);
        }

        if let Some(tier) = query.tier {
            result.retain(|c| c.tier() == tier);
        }

        if let Some(verified) = query.email_verified {
            result.retain(|c| c.is_email_verified() == verified);
        }

        if let Some(ref email_pattern) = query.email_contains {
            result.retain(|c| c.email().contains(email_pattern));
        }

        if let Some(ref name_pattern) = query.name_contains {
            result.retain(|c| {
                c.name()
                    .map(|n| n.contains(name_pattern))
                    .unwrap_or(false)
            });
        }

        if let Some(date) = query.created_after {
            result.retain(|c| c.created_at() > date);
        }

        if let Some(date) = query.created_before {
            result.retain(|c| c.created_at() < date);
        }

        if let Some(min_revenue) = query.min_revenue_cents {
            result.retain(|c| c.total_revenue_cents() >= min_revenue);
        }

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::CreatorTier;
    use crate::domain::value_objects::WalletAddress;

    const VALID_WALLETS: [&str; 8] = [
        "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM",
        "11111111111111111111111111111111",
        "22222222222222222222222222222222",
        "33333333333333333333333333333333",
        "44444444444444444444444444444444",
        "55555555555555555555555555555555",
        "66666666666666666666666666666666",
        "77777777777777777777777777777777",
    ];

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn test_wallet() -> WalletAddress {
        WalletAddress::new(VALID_WALLETS[0].to_string()).unwrap()
    }

    fn test_wallet_2() -> WalletAddress {
        WalletAddress::new(VALID_WALLETS[1].to_string()).unwrap()
    }

    fn test_wallet_n(n: usize) -> WalletAddress {
        WalletAddress::new(VALID_WALLETS[n % VALID_WALLETS.len()].to_string()).unwrap()
    }

    fn create_test_creator(email: &str) -> Creator {
        Creator::new(
            test_tenant_id(),
            email.to_string(),
            test_wallet(),
            None,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_create_creator() {
        let repo = InMemoryCreatorRepository::new();
        let creator = create_test_creator("test@example.com");

        let result = repo.create(creator).await;
        assert!(result.is_ok());
        assert_eq!(repo.len(), 1);
    }

    #[tokio::test]
    async fn test_create_duplicate_email() {
        let repo = InMemoryCreatorRepository::new();
        let creator1 = create_test_creator("test@example.com");
        let creator2 = create_test_creator("test@example.com");

        repo.create(creator1).await.unwrap();
        let result = repo.create(creator2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let repo = InMemoryCreatorRepository::new();
        let creator = create_test_creator("test@example.com");
        let creator_id = creator.id().clone();

        repo.create(creator).await.unwrap();

        let found = repo.find_by_id(&creator_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email(), "test@example.com");
    }

    #[tokio::test]
    async fn test_find_by_email() {
        let repo = InMemoryCreatorRepository::new();
        let creator = create_test_creator("unique@example.com");

        repo.create(creator).await.unwrap();

        let found = repo.find_by_email("unique@example.com").await.unwrap();
        assert!(found.is_some());

        let not_found = repo.find_by_email("nonexistent@example.com").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_wallet() {
        let repo = InMemoryCreatorRepository::new();
        let creator = create_test_creator("test@example.com");

        repo.create(creator).await.unwrap();

        let found = repo.find_by_wallet(&test_wallet()).await.unwrap();
        assert!(found.is_some());

        let not_found = repo.find_by_wallet(&test_wallet_2()).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_tenant() {
        let repo = InMemoryCreatorRepository::new();

        // Create creators for test tenant
        for i in 0..3 {
            let creator = Creator::new(
                test_tenant_id(),
                format!("creator{}@example.com", i),
                test_wallet_n(i),
                None,
            )
            .unwrap();
            repo.create(creator).await.unwrap();
        }

        // Create creator for different tenant
        let other_tenant = TenantId::new("other-tenant".to_string()).unwrap();
        let other_creator = Creator::new(
            other_tenant,
            "other@example.com".to_string(),
            test_wallet_2(),
            None,
        )
        .unwrap();
        repo.create(other_creator).await.unwrap();

        let creators = repo.find_by_tenant(&test_tenant_id(), 100, 0).await.unwrap();
        assert_eq!(creators.len(), 3);
    }

    #[tokio::test]
    async fn test_find_active() {
        let repo = InMemoryCreatorRepository::new();

        // Create and verify a creator (makes them active)
        let mut active_creator = create_test_creator("active@example.com");
        active_creator.verify_email();
        repo.create(active_creator).await.unwrap();

        // Create pending creator
        let pending_creator = Creator::new(
            test_tenant_id(),
            "pending@example.com".to_string(),
            test_wallet_2(),
            None,
        )
        .unwrap();
        repo.create(pending_creator).await.unwrap();

        let active = repo.find_active(100, 0).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].email(), "active@example.com");
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryCreatorRepository::new();
        let creator = create_test_creator("test@example.com");
        let creator_id = creator.id().clone();

        repo.create(creator).await.unwrap();
        assert_eq!(repo.len(), 1);

        let deleted = repo.delete(&creator_id).await.unwrap();
        assert!(deleted);
        assert_eq!(repo.len(), 0);

        // Delete non-existent
        let deleted_again = repo.delete(&creator_id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_query_with_status() {
        let repo = InMemoryCreatorRepository::new();

        // Create active creator
        let mut active = create_test_creator("active@example.com");
        active.verify_email();
        repo.create(active).await.unwrap();

        // Create pending creator
        let pending = Creator::new(
            test_tenant_id(),
            "pending@example.com".to_string(),
            test_wallet_2(),
            None,
        )
        .unwrap();
        repo.create(pending).await.unwrap();

        let query = CreatorQuery::new().with_status(CreatorStatus::Active);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].email(), "active@example.com");
    }

    #[tokio::test]
    async fn test_query_with_tier() {
        let repo = InMemoryCreatorRepository::new();

        // Create free tier creator
        let free_creator = create_test_creator("free@example.com");
        repo.create(free_creator).await.unwrap();

        // Create and upgrade to pro
        let mut pro_creator = Creator::new(
            test_tenant_id(),
            "pro@example.com".to_string(),
            test_wallet_2(),
            None,
        )
        .unwrap();
        pro_creator.upgrade_tier(CreatorTier::Pro);
        repo.create(pro_creator).await.unwrap();

        let query = CreatorQuery::new().with_tier(CreatorTier::Pro);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].email(), "pro@example.com");
    }

    #[tokio::test]
    async fn test_query_with_pagination() {
        let repo = InMemoryCreatorRepository::new();

        for i in 0..5 {
            let creator = Creator::new(
                test_tenant_id(),
                format!("creator{}@example.com", i),
                test_wallet_n(i),
                None,
            )
            .unwrap();
            repo.create(creator).await.unwrap();
        }

        let query = CreatorQuery::new().with_pagination(2, 1);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_query_verified_only() {
        let repo = InMemoryCreatorRepository::new();

        // Create verified creator
        let mut verified = create_test_creator("verified@example.com");
        verified.verify_email();
        repo.create(verified).await.unwrap();

        // Create unverified creator
        let unverified = Creator::new(
            test_tenant_id(),
            "unverified@example.com".to_string(),
            test_wallet_2(),
            None,
        )
        .unwrap();
        repo.create(unverified).await.unwrap();

        let query = CreatorQuery::new().verified_only();
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].email(), "verified@example.com");
    }

    #[tokio::test]
    async fn test_count_methods() {
        let repo = InMemoryCreatorRepository::new();

        // Create active creator
        let mut active = create_test_creator("active@example.com");
        active.verify_email();
        repo.create(active).await.unwrap();

        // Create pending creator
        let pending = Creator::new(
            test_tenant_id(),
            "pending@example.com".to_string(),
            test_wallet_2(),
            None,
        )
        .unwrap();
        repo.create(pending).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 2);
        assert_eq!(repo.count_by_status(CreatorStatus::Active).await.unwrap(), 1);
        assert_eq!(repo.count_by_status(CreatorStatus::Pending).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let repo = InMemoryCreatorRepository::new();

        for i in 0..3 {
            let creator = Creator::new(
                test_tenant_id(),
                format!("creator{}@example.com", i),
                test_wallet_n(i),
                None,
            )
            .unwrap();
            repo.create(creator).await.unwrap();
        }

        assert_eq!(repo.len(), 3);
        repo.clear();
        assert!(repo.is_empty());
    }
}
