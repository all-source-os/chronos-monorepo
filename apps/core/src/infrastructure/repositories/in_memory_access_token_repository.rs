use crate::domain::entities::{AccessToken, AccessTokenId};
use crate::domain::repositories::{AccessTokenQuery, AccessTokenRepository};
use crate::domain::value_objects::{ArticleId, CreatorId, TransactionId, WalletAddress};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory implementation of AccessTokenRepository
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
/// use allsource_core::infrastructure::repositories::InMemoryAccessTokenRepository;
///
/// let repo = InMemoryAccessTokenRepository::new();
/// // Use repo with access token use cases...
/// ```
pub struct InMemoryAccessTokenRepository {
    tokens: RwLock<HashMap<String, AccessToken>>,
}

impl InMemoryAccessTokenRepository {
    /// Create a new empty in-memory access token repository
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
        }
    }

    /// Get the number of tokens stored
    pub fn len(&self) -> usize {
        self.tokens.read().unwrap().len()
    }

    /// Check if the repository is empty
    pub fn is_empty(&self) -> bool {
        self.tokens.read().unwrap().is_empty()
    }

    /// Clear all tokens (for testing)
    pub fn clear(&self) {
        self.tokens.write().unwrap().clear();
    }
}

impl Default for InMemoryAccessTokenRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AccessTokenRepository for InMemoryAccessTokenRepository {
    async fn save(&self, token: &AccessToken) -> Result<()> {
        let mut tokens = self.tokens.write().unwrap();
        let id = token.id().to_string();

        // Check for duplicate token hash (on new tokens only)
        if !tokens.contains_key(&id) {
            let hash = token.token_hash();
            if tokens.values().any(|t| t.token_hash() == hash) {
                return Err(crate::error::AllSourceError::ValidationError(format!(
                    "Token with hash '{}' already exists",
                    hash
                )));
            }
        }

        tokens.insert(id, token.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &AccessTokenId) -> Result<Option<AccessToken>> {
        let tokens = self.tokens.read().unwrap();
        Ok(tokens.get(&id.to_string()).cloned())
    }

    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<AccessToken>> {
        let tokens = self.tokens.read().unwrap();
        Ok(tokens
            .values()
            .find(|t| t.token_hash() == token_hash)
            .cloned())
    }

    async fn find_by_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<AccessToken>> {
        let tokens = self.tokens.read().unwrap();
        Ok(tokens
            .values()
            .find(|t| t.transaction_id() == Some(transaction_id))
            .cloned())
    }

    async fn find_by_article_and_wallet(
        &self,
        article_id: &ArticleId,
        wallet: &WalletAddress,
    ) -> Result<Vec<AccessToken>> {
        let tokens = self.tokens.read().unwrap();
        Ok(tokens
            .values()
            .filter(|t| t.article_id() == article_id && t.reader_wallet() == wallet)
            .cloned()
            .collect())
    }

    async fn find_valid_token(
        &self,
        article_id: &ArticleId,
        wallet: &WalletAddress,
    ) -> Result<Option<AccessToken>> {
        let tokens = self.tokens.read().unwrap();
        Ok(tokens
            .values()
            .find(|t| t.grants_access_to(article_id, wallet))
            .cloned())
    }

    async fn find_by_reader(
        &self,
        wallet: &WalletAddress,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AccessToken>> {
        let tokens = self.tokens.read().unwrap();
        let mut result: Vec<_> = tokens
            .values()
            .filter(|t| t.reader_wallet() == wallet)
            .cloned()
            .collect();

        // Sort by issued_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.issued_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_by_article(
        &self,
        article_id: &ArticleId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AccessToken>> {
        let tokens = self.tokens.read().unwrap();
        let mut result: Vec<_> = tokens
            .values()
            .filter(|t| t.article_id() == article_id)
            .cloned()
            .collect();

        // Sort by issued_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.issued_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_by_creator(
        &self,
        creator_id: &CreatorId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AccessToken>> {
        let tokens = self.tokens.read().unwrap();
        let mut result: Vec<_> = tokens
            .values()
            .filter(|t| t.creator_id() == creator_id)
            .cloned()
            .collect();

        // Sort by issued_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.issued_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.tokens.read().unwrap().len())
    }

    async fn count_valid(&self) -> Result<usize> {
        let tokens = self.tokens.read().unwrap();
        Ok(tokens.values().filter(|t| t.is_valid()).count())
    }

    async fn count_by_article(&self, article_id: &ArticleId) -> Result<usize> {
        let tokens = self.tokens.read().unwrap();
        Ok(tokens
            .values()
            .filter(|t| t.article_id() == article_id)
            .count())
    }

    async fn revoke(&self, id: &AccessTokenId, reason: &str) -> Result<bool> {
        let mut tokens = self.tokens.write().unwrap();
        let id_str = id.to_string();

        if let Some(token) = tokens.get_mut(&id_str) {
            if token.is_revoked() {
                return Ok(false);
            }
            // We need to clone, mutate, and reinsert because we can't call &mut self methods
            let mut updated = token.clone();
            updated.revoke(reason)?;
            tokens.insert(id_str, updated);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn revoke_by_transaction(
        &self,
        transaction_id: &TransactionId,
        reason: &str,
    ) -> Result<usize> {
        let mut tokens = self.tokens.write().unwrap();
        let mut count = 0;

        let ids_to_revoke: Vec<_> = tokens
            .values()
            .filter(|t| t.transaction_id() == Some(transaction_id) && !t.is_revoked())
            .map(|t| t.id().to_string())
            .collect();

        for id in ids_to_revoke {
            if let Some(token) = tokens.get(&id) {
                let mut updated = token.clone();
                if updated.revoke(reason).is_ok() {
                    tokens.insert(id, updated);
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    async fn delete_expired(&self, before: DateTime<Utc>) -> Result<usize> {
        let mut tokens = self.tokens.write().unwrap();
        let to_delete: Vec<_> = tokens
            .values()
            .filter(|t| t.expires_at() < before)
            .map(|t| t.id().to_string())
            .collect();

        let count = to_delete.len();
        for id in to_delete {
            tokens.remove(&id);
        }

        Ok(count)
    }

    async fn query(&self, query: &AccessTokenQuery) -> Result<Vec<AccessToken>> {
        let tokens = self.tokens.read().unwrap();
        let mut result: Vec<_> = tokens.values().cloned().collect();

        // Apply filters
        if let Some(ref tenant_id) = query.tenant_id {
            result.retain(|t| t.tenant_id() == tenant_id);
        }

        if let Some(ref article_id) = query.article_id {
            result.retain(|t| t.article_id() == article_id);
        }

        if let Some(ref creator_id) = query.creator_id {
            result.retain(|t| t.creator_id() == creator_id);
        }

        if let Some(ref wallet) = query.reader_wallet {
            result.retain(|t| t.reader_wallet() == wallet);
        }

        if let Some(ref transaction_id) = query.transaction_id {
            result.retain(|t| t.transaction_id() == Some(transaction_id));
        }

        if let Some(method) = query.access_method {
            result.retain(|t| t.access_method() == method);
        }

        if query.valid_only {
            result.retain(|t| t.is_valid());
        }

        if query.revoked_only {
            result.retain(|t| t.is_revoked());
        }

        if let Some(date) = query.issued_after {
            result.retain(|t| t.issued_at() > date);
        }

        if let Some(date) = query.issued_before {
            result.retain(|t| t.issued_at() < date);
        }

        if let Some(date) = query.expires_after {
            result.retain(|t| t.expires_at() > date);
        }

        if let Some(date) = query.expires_before {
            result.retain(|t| t.expires_at() < date);
        }

        // Sort by issued_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.issued_at()));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::AccessMethod;
    use crate::domain::value_objects::{
        ArticleId, CreatorId, TenantId, TransactionId, WalletAddress,
    };

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
    const VALID_TOKEN_HASH: &str =
        "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd";

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn test_article_id() -> ArticleId {
        ArticleId::new("test-article".to_string()).unwrap()
    }

    fn test_creator_id() -> CreatorId {
        CreatorId::new()
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

    fn test_transaction_id() -> TransactionId {
        TransactionId::new()
    }

    fn create_test_token(hash_suffix: &str) -> AccessToken {
        AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            format!("{}{}", VALID_TOKEN_HASH[..60].to_string(), hash_suffix),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let repo = InMemoryAccessTokenRepository::new();
        let token = create_test_token("test");
        let token_id = token.id().clone();

        repo.save(&token).await.unwrap();

        let found = repo.find_by_id(&token_id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_hash_rejected() {
        let repo = InMemoryAccessTokenRepository::new();
        let token1 = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();
        repo.save(&token1).await.unwrap();

        let token2 = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(), // Same hash
        )
        .unwrap();

        let result = repo.save(&token2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_by_hash() {
        let repo = InMemoryAccessTokenRepository::new();
        let token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();
        repo.save(&token).await.unwrap();

        let found = repo.find_by_hash(VALID_TOKEN_HASH).await.unwrap();
        assert!(found.is_some());

        let not_found = repo.find_by_hash("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_transaction() {
        let repo = InMemoryAccessTokenRepository::new();
        let transaction_id = test_transaction_id();
        let token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            transaction_id,
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();
        repo.save(&token).await.unwrap();

        let found = repo.find_by_transaction(&transaction_id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_find_by_article_and_wallet() {
        let repo = InMemoryAccessTokenRepository::new();
        let article_id = test_article_id();
        let wallet = test_wallet();

        let token = AccessToken::new_paid(
            test_tenant_id(),
            article_id.clone(),
            test_creator_id(),
            wallet.clone(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();
        repo.save(&token).await.unwrap();

        let found = repo
            .find_by_article_and_wallet(&article_id, &wallet)
            .await
            .unwrap();
        assert_eq!(found.len(), 1);

        let not_found = repo
            .find_by_article_and_wallet(&article_id, &test_wallet_2())
            .await
            .unwrap();
        assert!(not_found.is_empty());
    }

    #[tokio::test]
    async fn test_find_valid_token() {
        let repo = InMemoryAccessTokenRepository::new();
        let article_id = test_article_id();
        let wallet = test_wallet();

        let token = AccessToken::new_paid(
            test_tenant_id(),
            article_id.clone(),
            test_creator_id(),
            wallet.clone(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();
        repo.save(&token).await.unwrap();

        let found = repo.find_valid_token(&article_id, &wallet).await.unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().is_valid());
    }

    #[tokio::test]
    async fn test_find_by_reader() {
        let repo = InMemoryAccessTokenRepository::new();
        let wallet = test_wallet();

        for i in 0..3 {
            let token = AccessToken::new_paid(
                test_tenant_id(),
                ArticleId::new(format!("article-{}", i)).unwrap(),
                test_creator_id(),
                wallet.clone(),
                test_transaction_id(),
                format!(
                    "{}{}",
                    VALID_TOKEN_HASH[..60].to_string(),
                    format!("{:04}", i)
                ),
            )
            .unwrap();
            repo.save(&token).await.unwrap();
        }

        let tokens = repo.find_by_reader(&wallet, 100, 0).await.unwrap();
        assert_eq!(tokens.len(), 3);
    }

    #[tokio::test]
    async fn test_find_by_article() {
        let repo = InMemoryAccessTokenRepository::new();
        let article_id = test_article_id();

        for i in 0..3 {
            let token = AccessToken::new_paid(
                test_tenant_id(),
                article_id.clone(),
                test_creator_id(),
                test_wallet_n(i),
                test_transaction_id(),
                format!(
                    "{}{}",
                    VALID_TOKEN_HASH[..60].to_string(),
                    format!("{:04}", i)
                ),
            )
            .unwrap();
            repo.save(&token).await.unwrap();
        }

        let tokens = repo.find_by_article(&article_id, 100, 0).await.unwrap();
        assert_eq!(tokens.len(), 3);
    }

    #[tokio::test]
    async fn test_find_by_creator() {
        let repo = InMemoryAccessTokenRepository::new();
        let creator_id = test_creator_id();

        for i in 0..3 {
            let token = AccessToken::new_paid(
                test_tenant_id(),
                ArticleId::new(format!("article-{}", i)).unwrap(),
                creator_id,
                test_wallet_n(i),
                test_transaction_id(),
                format!(
                    "{}{}",
                    VALID_TOKEN_HASH[..60].to_string(),
                    format!("{:04}", i)
                ),
            )
            .unwrap();
            repo.save(&token).await.unwrap();
        }

        let tokens = repo.find_by_creator(&creator_id, 100, 0).await.unwrap();
        assert_eq!(tokens.len(), 3);
    }

    #[tokio::test]
    async fn test_revoke() {
        let repo = InMemoryAccessTokenRepository::new();
        let token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();
        let token_id = token.id().clone();
        repo.save(&token).await.unwrap();

        let revoked = repo.revoke(&token_id, "Test revocation").await.unwrap();
        assert!(revoked);

        // Check token is revoked
        let found = repo.find_by_id(&token_id).await.unwrap().unwrap();
        assert!(found.is_revoked());

        // Try to revoke again
        let revoked_again = repo.revoke(&token_id, "Second revocation").await.unwrap();
        assert!(!revoked_again);
    }

    #[tokio::test]
    async fn test_revoke_by_transaction() {
        let repo = InMemoryAccessTokenRepository::new();
        let transaction_id = test_transaction_id();

        // Create multiple tokens for the same transaction
        for i in 0..3 {
            let token = AccessToken::new_paid(
                test_tenant_id(),
                ArticleId::new(format!("article-{}", i)).unwrap(),
                test_creator_id(),
                test_wallet(),
                transaction_id,
                format!(
                    "{}{}",
                    VALID_TOKEN_HASH[..60].to_string(),
                    format!("{:04}", i)
                ),
            )
            .unwrap();
            repo.save(&token).await.unwrap();
        }

        let count = repo
            .revoke_by_transaction(&transaction_id, "Refund")
            .await
            .unwrap();
        assert_eq!(count, 3);

        // Verify all are revoked
        let valid_count = repo.count_valid().await.unwrap();
        assert_eq!(valid_count, 0);
    }

    #[tokio::test]
    async fn test_delete_expired() {
        let repo = InMemoryAccessTokenRepository::new();

        // Create a token that expires soon
        let token = AccessToken::new_with_duration(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Some(test_transaction_id()),
            AccessMethod::Paid,
            VALID_TOKEN_HASH.to_string(),
            1, // 1 day
        )
        .unwrap();
        repo.save(&token).await.unwrap();

        // Delete tokens expired before 2 days from now
        let future = Utc::now() + chrono::Duration::days(2);
        let deleted = repo.delete_expired(future).await.unwrap();
        assert_eq!(deleted, 1);
        assert!(repo.is_empty());
    }

    #[tokio::test]
    async fn test_query_valid_only() {
        let repo = InMemoryAccessTokenRepository::new();

        // Create valid token
        let valid = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            format!("{}val1", VALID_TOKEN_HASH[..60].to_string()),
        )
        .unwrap();
        repo.save(&valid).await.unwrap();

        // Create and revoke a token
        let mut revoked = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet_2(),
            test_transaction_id(),
            format!("{}rev1", VALID_TOKEN_HASH[..60].to_string()),
        )
        .unwrap();
        revoked.revoke("Test").unwrap();
        repo.save(&revoked).await.unwrap();

        let query = AccessTokenQuery::new().valid_only();
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_query_by_access_method() {
        let repo = InMemoryAccessTokenRepository::new();

        // Create paid token
        let paid = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            format!("{}paid", VALID_TOKEN_HASH[..60].to_string()),
        )
        .unwrap();
        repo.save(&paid).await.unwrap();

        // Create free token
        let free = AccessToken::new_free(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet_2(),
            format!("{}free", VALID_TOKEN_HASH[..60].to_string()),
            7,
        )
        .unwrap();
        repo.save(&free).await.unwrap();

        let query = AccessTokenQuery::new().with_access_method(AccessMethod::Free);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].access_method(), AccessMethod::Free);
    }

    #[tokio::test]
    async fn test_query_with_pagination() {
        let repo = InMemoryAccessTokenRepository::new();

        for i in 0..5 {
            let token = AccessToken::new_paid(
                test_tenant_id(),
                ArticleId::new(format!("article-{}", i)).unwrap(),
                test_creator_id(),
                test_wallet_n(i),
                test_transaction_id(),
                format!(
                    "{}{}",
                    VALID_TOKEN_HASH[..60].to_string(),
                    format!("{:04}", i)
                ),
            )
            .unwrap();
            repo.save(&token).await.unwrap();
        }

        let query = AccessTokenQuery::new().with_pagination(2, 1);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_count_methods() {
        let repo = InMemoryAccessTokenRepository::new();
        let article_id = test_article_id();

        // Create valid token
        let valid = AccessToken::new_paid(
            test_tenant_id(),
            article_id.clone(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            format!("{}val1", VALID_TOKEN_HASH[..60].to_string()),
        )
        .unwrap();
        repo.save(&valid).await.unwrap();

        // Create and revoke a token
        let mut revoked = AccessToken::new_paid(
            test_tenant_id(),
            article_id.clone(),
            test_creator_id(),
            test_wallet_2(),
            test_transaction_id(),
            format!("{}rev1", VALID_TOKEN_HASH[..60].to_string()),
        )
        .unwrap();
        revoked.revoke("Test").unwrap();
        repo.save(&revoked).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 2);
        assert_eq!(repo.count_valid().await.unwrap(), 1);
        assert_eq!(repo.count_by_article(&article_id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_clear() {
        let repo = InMemoryAccessTokenRepository::new();

        for i in 0..3 {
            let token = create_test_token(&format!("{:04}", i));
            repo.save(&token).await.unwrap();
        }

        assert_eq!(repo.len(), 3);
        repo.clear();
        assert!(repo.is_empty());
    }
}
