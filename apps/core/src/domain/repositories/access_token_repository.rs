use crate::domain::entities::{AccessMethod, AccessToken, AccessTokenId};
use crate::domain::value_objects::{ArticleId, CreatorId, TenantId, TransactionId, WalletAddress};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Repository trait for access token management
///
/// Provides persistent storage and retrieval operations for access tokens.
/// Access tokens grant readers time-limited access to paywalled articles.
///
/// # Responsibilities
/// - CRUD operations for access tokens
/// - Token lookup by various fields
/// - Token revocation
/// - Access validation
///
/// # Thread Safety
/// Implementations must be thread-safe (Send + Sync).
#[async_trait]
pub trait AccessTokenRepository: Send + Sync {
    /// Save a new access token
    ///
    /// # Arguments
    /// * `token` - The access token to save
    ///
    /// # Errors
    /// - `ValidationError` - If a token with this hash already exists
    /// - `StorageError` - If the operation fails
    async fn save(&self, token: &AccessToken) -> Result<()>;

    /// Find an access token by ID
    ///
    /// # Arguments
    /// * `id` - The access token ID to search for
    ///
    /// # Returns
    /// `Some(AccessToken)` if found, `None` otherwise
    async fn find_by_id(&self, id: &AccessTokenId) -> Result<Option<AccessToken>>;

    /// Find an access token by token hash
    ///
    /// # Arguments
    /// * `token_hash` - The hash of the token to search for
    ///
    /// # Returns
    /// `Some(AccessToken)` if found, `None` otherwise
    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<AccessToken>>;

    /// Find an access token by transaction ID
    ///
    /// # Arguments
    /// * `transaction_id` - The transaction ID that created this token
    ///
    /// # Returns
    /// `Some(AccessToken)` if found, `None` otherwise
    async fn find_by_transaction(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Option<AccessToken>>;

    /// Find access tokens by article and reader wallet
    ///
    /// Returns all tokens (including expired/revoked) for audit purposes.
    ///
    /// # Arguments
    /// * `article_id` - The article ID
    /// * `wallet` - The reader wallet address
    ///
    /// # Returns
    /// Vector of access tokens for this article/wallet combination
    async fn find_by_article_and_wallet(
        &self,
        article_id: &ArticleId,
        wallet: &WalletAddress,
    ) -> Result<Vec<AccessToken>>;

    /// Find valid (not expired, not revoked) access token for article and wallet
    ///
    /// # Arguments
    /// * `article_id` - The article ID
    /// * `wallet` - The reader wallet address
    ///
    /// # Returns
    /// `Some(AccessToken)` if a valid token exists, `None` otherwise
    async fn find_valid_token(
        &self,
        article_id: &ArticleId,
        wallet: &WalletAddress,
    ) -> Result<Option<AccessToken>>;

    /// Find access tokens by reader wallet
    ///
    /// # Arguments
    /// * `wallet` - The reader wallet address
    /// * `limit` - Maximum number of tokens to return
    /// * `offset` - Number of tokens to skip
    ///
    /// # Returns
    /// Vector of access tokens for this reader
    async fn find_by_reader(
        &self,
        wallet: &WalletAddress,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AccessToken>>;

    /// Find access tokens by article
    ///
    /// # Arguments
    /// * `article_id` - The article ID
    /// * `limit` - Maximum number of tokens to return
    /// * `offset` - Number of tokens to skip
    ///
    /// # Returns
    /// Vector of access tokens for this article
    async fn find_by_article(
        &self,
        article_id: &ArticleId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AccessToken>>;

    /// Find access tokens by creator
    ///
    /// # Arguments
    /// * `creator_id` - The creator ID
    /// * `limit` - Maximum number of tokens to return
    /// * `offset` - Number of tokens to skip
    ///
    /// # Returns
    /// Vector of access tokens for this creator's articles
    async fn find_by_creator(
        &self,
        creator_id: &CreatorId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AccessToken>>;

    /// Count total access tokens
    async fn count(&self) -> Result<usize>;

    /// Count valid (not expired, not revoked) tokens
    async fn count_valid(&self) -> Result<usize>;

    /// Count tokens for a specific article
    async fn count_by_article(&self, article_id: &ArticleId) -> Result<usize>;

    /// Check if a valid token exists for an article and wallet
    async fn has_valid_access(
        &self,
        article_id: &ArticleId,
        wallet: &WalletAddress,
    ) -> Result<bool> {
        Ok(self.find_valid_token(article_id, wallet).await?.is_some())
    }

    /// Revoke a token by ID
    ///
    /// # Arguments
    /// * `id` - The access token ID to revoke
    /// * `reason` - The reason for revocation
    ///
    /// # Returns
    /// `true` if the token was revoked, `false` if it didn't exist or was already revoked
    async fn revoke(&self, id: &AccessTokenId, reason: &str) -> Result<bool>;

    /// Revoke all tokens for a transaction (e.g., on refund)
    ///
    /// # Arguments
    /// * `transaction_id` - The transaction ID
    /// * `reason` - The reason for revocation
    ///
    /// # Returns
    /// Number of tokens revoked
    async fn revoke_by_transaction(
        &self,
        transaction_id: &TransactionId,
        reason: &str,
    ) -> Result<usize>;

    /// Delete expired tokens (cleanup operation)
    ///
    /// # Arguments
    /// * `before` - Delete tokens that expired before this timestamp
    ///
    /// # Returns
    /// Number of tokens deleted
    async fn delete_expired(&self, before: DateTime<Utc>) -> Result<usize>;

    /// Query access tokens with filters
    async fn query(&self, query: &AccessTokenQuery) -> Result<Vec<AccessToken>>;
}

/// Query filter for finding access tokens
#[derive(Debug, Clone, Default)]
pub struct AccessTokenQuery {
    pub tenant_id: Option<TenantId>,
    pub article_id: Option<ArticleId>,
    pub creator_id: Option<CreatorId>,
    pub reader_wallet: Option<WalletAddress>,
    pub transaction_id: Option<TransactionId>,
    pub access_method: Option<AccessMethod>,
    pub valid_only: bool,
    pub revoked_only: bool,
    pub issued_after: Option<DateTime<Utc>>,
    pub issued_before: Option<DateTime<Utc>>,
    pub expires_after: Option<DateTime<Utc>>,
    pub expires_before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl AccessTokenQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_tenant(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    pub fn for_article(mut self, article_id: ArticleId) -> Self {
        self.article_id = Some(article_id);
        self
    }

    pub fn for_creator(mut self, creator_id: CreatorId) -> Self {
        self.creator_id = Some(creator_id);
        self
    }

    pub fn for_reader(mut self, wallet: WalletAddress) -> Self {
        self.reader_wallet = Some(wallet);
        self
    }

    pub fn for_transaction(mut self, transaction_id: TransactionId) -> Self {
        self.transaction_id = Some(transaction_id);
        self
    }

    pub fn with_access_method(mut self, method: AccessMethod) -> Self {
        self.access_method = Some(method);
        self
    }

    pub fn valid_only(mut self) -> Self {
        self.valid_only = true;
        self.revoked_only = false;
        self
    }

    pub fn revoked_only(mut self) -> Self {
        self.revoked_only = true;
        self.valid_only = false;
        self
    }

    pub fn issued_after(mut self, date: DateTime<Utc>) -> Self {
        self.issued_after = Some(date);
        self
    }

    pub fn issued_before(mut self, date: DateTime<Utc>) -> Self {
        self.issued_before = Some(date);
        self
    }

    pub fn expires_after(mut self, date: DateTime<Utc>) -> Self {
        self.expires_after = Some(date);
        self
    }

    pub fn expires_before(mut self, date: DateTime<Utc>) -> Self {
        self.expires_before = Some(date);
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
    fn test_access_token_query_builder() {
        let query = AccessTokenQuery::new().valid_only().with_pagination(10, 0);

        assert!(query.valid_only);
        assert!(!query.revoked_only);
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_access_token_query_for_article() {
        let article_id = ArticleId::new("test-article".to_string()).unwrap();

        let query = AccessTokenQuery::new().for_article(article_id).valid_only();

        assert!(query.article_id.is_some());
        assert!(query.valid_only);
    }

    #[test]
    fn test_access_token_query_with_dates() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);

        let query = AccessTokenQuery::new()
            .issued_after(yesterday)
            .expires_before(now);

        assert!(query.issued_after.is_some());
        assert!(query.expires_before.is_some());
    }

    #[test]
    fn test_revoked_only_clears_valid_only() {
        let query = AccessTokenQuery::new().valid_only().revoked_only();

        assert!(query.revoked_only);
        assert!(!query.valid_only);
    }
}
