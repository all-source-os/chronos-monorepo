use crate::application::dto::{
    AccessTokenDto, CheckAccessRequest, CheckAccessResponse, GrantAccessResponse,
    GrantFreeAccessRequest, ListAccessTokensResponse, RevokeAccessRequest, RevokeAccessResponse,
};
use crate::domain::entities::{AccessToken, AccessTokenId};
use crate::domain::repositories::{AccessTokenRepository, ArticleRepository};
use crate::domain::value_objects::{ArticleId, WalletAddress};
use crate::error::Result;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Use Case: Grant Free Access
///
/// Grants free (promotional) access to an article for a reader.
/// This is used for promotional access, reviewer access, etc.
///
/// Responsibilities:
/// - Validate input
/// - Verify article exists
/// - Check for existing valid access
/// - Create access token
/// - Persist via repository
/// - Return response with raw token
pub struct GrantFreeAccessUseCase {
    access_token_repository: Arc<dyn AccessTokenRepository>,
    article_repository: Arc<dyn ArticleRepository>,
}

impl GrantFreeAccessUseCase {
    pub fn new(
        access_token_repository: Arc<dyn AccessTokenRepository>,
        article_repository: Arc<dyn ArticleRepository>,
    ) -> Self {
        Self {
            access_token_repository,
            article_repository,
        }
    }

    pub async fn execute(&self, request: GrantFreeAccessRequest) -> Result<GrantAccessResponse> {
        // Parse IDs
        let article_id = ArticleId::new(request.article_id)?;
        let reader_wallet = WalletAddress::new(request.reader_wallet)?;
        let tenant_id = crate::domain::value_objects::TenantId::new(request.tenant_id)?;

        // Verify article exists
        let article = self
            .article_repository
            .find_by_id(&article_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound("Article not found".to_string())
            })?;

        // Check for existing valid access
        if self
            .access_token_repository
            .has_valid_access(&article_id, &reader_wallet)
            .await?
        {
            return Err(crate::error::AllSourceError::ValidationError(
                "Reader already has valid access to this article".to_string(),
            ));
        }

        // Duration defaults to 30 days
        let duration_days = request.duration_days.unwrap_or(30);

        // Generate token hash
        let raw_token = generate_raw_token(&article_id, &reader_wallet);
        let token_hash = hash_token(&raw_token);

        // Create access token
        let access_token = AccessToken::new_free(
            tenant_id,
            article_id,
            *article.creator_id(),
            reader_wallet,
            token_hash,
            duration_days,
        )?;

        // Persist access token
        self.access_token_repository.save(&access_token).await?;

        Ok(GrantAccessResponse {
            access_token: AccessTokenDto::from(&access_token),
            raw_token,
        })
    }
}

/// Use Case: Check Access
///
/// Checks if a reader has valid access to an article.
pub struct CheckAccessUseCase {
    repository: Arc<dyn AccessTokenRepository>,
}

impl CheckAccessUseCase {
    pub fn new(repository: Arc<dyn AccessTokenRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: CheckAccessRequest) -> Result<CheckAccessResponse> {
        let article_id = ArticleId::new(request.article_id)?;
        let wallet = WalletAddress::new(request.wallet_address)?;

        let token = self
            .repository
            .find_valid_token(&article_id, &wallet)
            .await?;

        match token {
            Some(token) => Ok(CheckAccessResponse {
                has_access: true,
                remaining_days: Some(token.remaining_days()),
                access_token: Some(AccessTokenDto::from(&token)),
            }),
            None => Ok(CheckAccessResponse {
                has_access: false,
                remaining_days: None,
                access_token: None,
            }),
        }
    }
}

/// Use Case: Validate Token
///
/// Validates a raw access token and records access if valid.
pub struct ValidateTokenUseCase {
    repository: Arc<dyn AccessTokenRepository>,
}

impl ValidateTokenUseCase {
    pub fn new(repository: Arc<dyn AccessTokenRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        raw_token: &str,
        article_id: &str,
        wallet_address: &str,
    ) -> Result<AccessTokenDto> {
        let article_id = ArticleId::new(article_id.to_string())?;
        let wallet = WalletAddress::new(wallet_address.to_string())?;

        // Hash the raw token
        let token_hash = hash_token(raw_token);

        // Find token by hash
        let mut token = self
            .repository
            .find_by_hash(&token_hash)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound("Token not found".to_string())
            })?;

        // Verify token grants access to this article and wallet
        if !token.grants_access_to(&article_id, &wallet) {
            return Err(crate::error::AllSourceError::ValidationError(
                "Token does not grant access to this article".to_string(),
            ));
        }

        // Record access
        token.record_access();

        // Note: In a real implementation, we would save the updated token here
        // For now, we return the DTO with the recorded access

        Ok(AccessTokenDto::from(&token))
    }
}

/// Use Case: Revoke Access
///
/// Revokes an access token.
pub struct RevokeAccessUseCase {
    repository: Arc<dyn AccessTokenRepository>,
}

impl RevokeAccessUseCase {
    pub fn new(repository: Arc<dyn AccessTokenRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: RevokeAccessRequest) -> Result<RevokeAccessResponse> {
        let token_id = AccessTokenId::parse(&request.token_id)?;

        // Find token
        let mut token = self
            .repository
            .find_by_id(&token_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound("Token not found".to_string())
            })?;

        // Revoke token
        token.revoke(&request.reason)?;

        // Persist via repository
        let revoked = self.repository.revoke(&token_id, &request.reason).await?;

        Ok(RevokeAccessResponse {
            revoked,
            access_token: AccessTokenDto::from(&token),
        })
    }
}

/// Use Case: Extend Access
///
/// Extends the duration of an access token.
pub struct ExtendAccessUseCase;

impl ExtendAccessUseCase {
    pub fn execute(mut token: AccessToken, additional_days: i64) -> Result<AccessTokenDto> {
        token.extend(additional_days)?;
        Ok(AccessTokenDto::from(&token))
    }
}

/// Use Case: Record Access
///
/// Records that an access token was used.
pub struct RecordAccessUseCase;

impl RecordAccessUseCase {
    pub fn execute(mut token: AccessToken) -> AccessTokenDto {
        token.record_access();
        AccessTokenDto::from(&token)
    }
}

/// Use Case: List Access Tokens
///
/// Returns a list of access tokens.
pub struct ListAccessTokensUseCase;

impl ListAccessTokensUseCase {
    pub fn execute(tokens: Vec<AccessToken>) -> ListAccessTokensResponse {
        let token_dtos: Vec<AccessTokenDto> = tokens.iter().map(AccessTokenDto::from).collect();
        let count = token_dtos.len();

        ListAccessTokensResponse {
            tokens: token_dtos,
            count,
        }
    }
}

/// Use Case: Cleanup Expired Tokens
///
/// Deletes expired tokens for cleanup.
pub struct CleanupExpiredTokensUseCase {
    repository: Arc<dyn AccessTokenRepository>,
}

impl CleanupExpiredTokensUseCase {
    pub fn new(repository: Arc<dyn AccessTokenRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, before: chrono::DateTime<chrono::Utc>) -> Result<usize> {
        self.repository.delete_expired(before).await
    }
}

/// Generate a raw token for access
fn generate_raw_token(article_id: &ArticleId, wallet: &WalletAddress) -> String {
    use rand::Rng;
    let random_bytes: [u8; 32] = rand::thread_rng().gen();
    let mut hasher = Sha256::new();
    hasher.update(article_id.to_string().as_bytes());
    hasher.update(wallet.to_string().as_bytes());
    hasher.update(random_bytes);
    hasher.update(chrono::Utc::now().to_rfc3339().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Hash a raw token for storage
fn hash_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::PaywallArticle;
    use crate::domain::repositories::{AccessTokenQuery, ArticleQuery};
    use crate::domain::value_objects::{CreatorId, TenantId, TransactionId};
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    const VALID_WALLET: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
    const VALID_TOKEN_HASH: &str =
        "a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd";

    struct MockAccessTokenRepository {
        tokens: Mutex<Vec<AccessToken>>,
    }

    impl MockAccessTokenRepository {
        fn new() -> Self {
            Self {
                tokens: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl AccessTokenRepository for MockAccessTokenRepository {
        async fn save(&self, token: &AccessToken) -> Result<()> {
            self.tokens.lock().unwrap().push(token.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: &AccessTokenId) -> Result<Option<AccessToken>> {
            let tokens = self.tokens.lock().unwrap();
            Ok(tokens.iter().find(|t| t.id() == id).cloned())
        }

        async fn find_by_hash(&self, token_hash: &str) -> Result<Option<AccessToken>> {
            let tokens = self.tokens.lock().unwrap();
            Ok(tokens
                .iter()
                .find(|t| t.token_hash() == token_hash)
                .cloned())
        }

        async fn find_by_transaction(
            &self,
            _transaction_id: &TransactionId,
        ) -> Result<Option<AccessToken>> {
            Ok(None)
        }

        async fn find_by_article_and_wallet(
            &self,
            article_id: &ArticleId,
            wallet: &WalletAddress,
        ) -> Result<Vec<AccessToken>> {
            let tokens = self.tokens.lock().unwrap();
            Ok(tokens
                .iter()
                .filter(|t| t.article_id() == article_id && t.reader_wallet() == wallet)
                .cloned()
                .collect())
        }

        async fn find_valid_token(
            &self,
            article_id: &ArticleId,
            wallet: &WalletAddress,
        ) -> Result<Option<AccessToken>> {
            let tokens = self.tokens.lock().unwrap();
            Ok(tokens
                .iter()
                .find(|t| {
                    t.article_id() == article_id && t.reader_wallet() == wallet && t.is_valid()
                })
                .cloned())
        }

        async fn find_by_reader(
            &self,
            _wallet: &WalletAddress,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<AccessToken>> {
            Ok(Vec::new())
        }

        async fn find_by_article(
            &self,
            _article_id: &ArticleId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<AccessToken>> {
            Ok(Vec::new())
        }

        async fn find_by_creator(
            &self,
            _creator_id: &CreatorId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<AccessToken>> {
            Ok(Vec::new())
        }

        async fn count(&self) -> Result<usize> {
            Ok(self.tokens.lock().unwrap().len())
        }

        async fn count_valid(&self) -> Result<usize> {
            Ok(0)
        }

        async fn count_by_article(&self, _article_id: &ArticleId) -> Result<usize> {
            Ok(0)
        }

        async fn revoke(&self, _id: &AccessTokenId, _reason: &str) -> Result<bool> {
            Ok(true)
        }

        async fn revoke_by_transaction(
            &self,
            _transaction_id: &TransactionId,
            _reason: &str,
        ) -> Result<usize> {
            Ok(1)
        }

        async fn delete_expired(&self, _before: DateTime<Utc>) -> Result<usize> {
            Ok(0)
        }

        async fn query(&self, _query: &AccessTokenQuery) -> Result<Vec<AccessToken>> {
            Ok(Vec::new())
        }
    }

    struct MockArticleRepository {
        articles: Mutex<Vec<PaywallArticle>>,
    }

    impl MockArticleRepository {
        fn new() -> Self {
            Self {
                articles: Mutex::new(Vec::new()),
            }
        }

        fn add_article(&self, article: PaywallArticle) {
            self.articles.lock().unwrap().push(article);
        }
    }

    #[async_trait]
    impl ArticleRepository for MockArticleRepository {
        async fn save(&self, _article: &PaywallArticle) -> Result<()> {
            Ok(())
        }

        async fn find_by_id(&self, id: &ArticleId) -> Result<Option<PaywallArticle>> {
            let articles = self.articles.lock().unwrap();
            Ok(articles.iter().find(|a| a.id() == id).cloned())
        }

        async fn find_by_url(&self, _url: &str) -> Result<Option<PaywallArticle>> {
            Ok(None)
        }

        async fn find_by_creator(
            &self,
            _creator_id: &CreatorId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<PaywallArticle>> {
            Ok(Vec::new())
        }

        async fn find_by_tenant(
            &self,
            _tenant_id: &TenantId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<PaywallArticle>> {
            Ok(Vec::new())
        }

        async fn find_active_by_creator(
            &self,
            _creator_id: &CreatorId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<PaywallArticle>> {
            Ok(Vec::new())
        }

        async fn find_by_status(
            &self,
            _status: crate::domain::entities::ArticleStatus,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<PaywallArticle>> {
            Ok(Vec::new())
        }

        async fn count(&self) -> Result<usize> {
            Ok(0)
        }

        async fn count_by_creator(&self, _creator_id: &CreatorId) -> Result<usize> {
            Ok(0)
        }

        async fn count_by_status(
            &self,
            _status: crate::domain::entities::ArticleStatus,
        ) -> Result<usize> {
            Ok(0)
        }

        async fn delete(&self, _id: &ArticleId) -> Result<bool> {
            Ok(false)
        }

        async fn query(&self, _query: &ArticleQuery) -> Result<Vec<PaywallArticle>> {
            Ok(Vec::new())
        }

        async fn find_top_by_revenue(
            &self,
            _creator_id: Option<&CreatorId>,
            _limit: usize,
        ) -> Result<Vec<PaywallArticle>> {
            Ok(Vec::new())
        }

        async fn find_recent(
            &self,
            _creator_id: Option<&CreatorId>,
            _limit: usize,
        ) -> Result<Vec<PaywallArticle>> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn test_grant_free_access() {
        let token_repo = Arc::new(MockAccessTokenRepository::new());
        let article_repo = Arc::new(MockArticleRepository::new());

        // Create and add article
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
        let creator_id = CreatorId::new();

        let article = PaywallArticle::new(
            article_id.clone(),
            tenant_id.clone(),
            creator_id,
            "Test Article".to_string(),
            "https://example.com/article".to_string(),
            50,
        )
        .unwrap();
        article_repo.add_article(article);

        let use_case = GrantFreeAccessUseCase::new(token_repo.clone(), article_repo);

        let request = GrantFreeAccessRequest {
            tenant_id: "test-tenant".to_string(),
            article_id: "test-article".to_string(),
            reader_wallet: VALID_WALLET.to_string(),
            duration_days: Some(7),
            reason: Some("Promotional access".to_string()),
        };

        let response = use_case.execute(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert!(!response.raw_token.is_empty());
        assert!(response.access_token.is_valid);
    }

    #[tokio::test]
    async fn test_check_access_no_token() {
        let token_repo = Arc::new(MockAccessTokenRepository::new());
        let use_case = CheckAccessUseCase::new(token_repo);

        let request = CheckAccessRequest {
            article_id: "test-article".to_string(),
            wallet_address: VALID_WALLET.to_string(),
        };

        let response = use_case.execute(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert!(!response.has_access);
        assert!(response.access_token.is_none());
    }

    #[tokio::test]
    async fn test_check_access_with_token() {
        let token_repo = Arc::new(MockAccessTokenRepository::new());

        // Add a valid token
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
        let creator_id = CreatorId::new();
        let wallet = WalletAddress::new(VALID_WALLET.to_string()).unwrap();

        let token = AccessToken::new_free(
            tenant_id,
            article_id.clone(),
            creator_id,
            wallet.clone(),
            VALID_TOKEN_HASH.to_string(),
            30,
        )
        .unwrap();

        token_repo.save(&token).await.unwrap();

        let use_case = CheckAccessUseCase::new(token_repo);

        let request = CheckAccessRequest {
            article_id: "test-article".to_string(),
            wallet_address: VALID_WALLET.to_string(),
        };

        let response = use_case.execute(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert!(response.has_access);
        assert!(response.access_token.is_some());
    }

    #[test]
    fn test_extend_access() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
        let creator_id = CreatorId::new();
        let wallet = WalletAddress::new(VALID_WALLET.to_string()).unwrap();

        let token = AccessToken::new_free(
            tenant_id,
            article_id,
            creator_id,
            wallet,
            VALID_TOKEN_HASH.to_string(),
            7,
        )
        .unwrap();

        let original_days = token.remaining_days();

        let result = ExtendAccessUseCase::execute(token, 7);
        assert!(result.is_ok());

        let dto = result.unwrap();
        assert!(dto.remaining_days >= original_days + 6); // Allow for timing variations
    }

    #[test]
    fn test_list_access_tokens() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
        let creator_id = CreatorId::new();
        let wallet = WalletAddress::new(VALID_WALLET.to_string()).unwrap();

        let tokens = vec![AccessToken::new_free(
            tenant_id,
            article_id,
            creator_id,
            wallet,
            VALID_TOKEN_HASH.to_string(),
            30,
        )
        .unwrap()];

        let response = ListAccessTokensUseCase::execute(tokens);
        assert_eq!(response.count, 1);
        assert_eq!(response.tokens.len(), 1);
    }

    #[test]
    fn test_token_hashing() {
        let raw_token = "test_token_12345";
        let hash1 = hash_token(raw_token);
        let hash2 = hash_token(raw_token);

        // Same input should produce same hash
        assert_eq!(hash1, hash2);

        // Hash should be 64 characters (SHA-256 hex)
        assert_eq!(hash1.len(), 64);
    }
}
