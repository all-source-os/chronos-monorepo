use crate::domain::value_objects::{ArticleId, CreatorId, TenantId, TransactionId, WalletAddress};
use crate::error::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Access token ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccessTokenId(Uuid);

impl AccessTokenId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn parse(value: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(value).map_err(|e| {
            crate::error::AllSourceError::InvalidInput(format!(
                "Invalid access token ID '{}': {}",
                value, e
            ))
        })?;
        Ok(Self(uuid))
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for AccessTokenId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AccessTokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// How access was granted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessMethod {
    /// Paid for directly
    Paid,
    /// Accessed through a bundle
    Bundle,
    /// Free access (promotional)
    Free,
    /// Subscription access
    Subscription,
}

impl Default for AccessMethod {
    fn default() -> Self {
        Self::Paid
    }
}

/// Default access duration: 30 days
const DEFAULT_ACCESS_DAYS: i64 = 30;

/// Domain Entity: AccessToken
///
/// Represents a reader's access to a specific article.
/// Access tokens are issued after successful payment and grant time-limited access.
///
/// Domain Rules:
/// - Token is tied to a specific article and reader wallet
/// - Token has an expiration date (default 30 days)
/// - Token can be revoked (e.g., on refund)
/// - Token cannot be transferred to another wallet
/// - Expired or revoked tokens do not grant access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    id: AccessTokenId,
    tenant_id: TenantId,
    article_id: ArticleId,
    creator_id: CreatorId,
    reader_wallet: WalletAddress,
    transaction_id: Option<TransactionId>,
    access_method: AccessMethod,
    token_hash: String,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    revoked: bool,
    revoked_at: Option<DateTime<Utc>>,
    revocation_reason: Option<String>,
    last_accessed_at: Option<DateTime<Utc>>,
    access_count: u32,
    metadata: serde_json::Value,
}

impl AccessToken {
    /// Create a new access token for a paid transaction
    pub fn new_paid(
        tenant_id: TenantId,
        article_id: ArticleId,
        creator_id: CreatorId,
        reader_wallet: WalletAddress,
        transaction_id: TransactionId,
        token_hash: String,
    ) -> Result<Self> {
        Self::validate_token_hash(&token_hash)?;

        let now = Utc::now();
        let expires_at = now + Duration::days(DEFAULT_ACCESS_DAYS);

        Ok(Self {
            id: AccessTokenId::new(),
            tenant_id,
            article_id,
            creator_id,
            reader_wallet,
            transaction_id: Some(transaction_id),
            access_method: AccessMethod::Paid,
            token_hash,
            issued_at: now,
            expires_at,
            revoked: false,
            revoked_at: None,
            revocation_reason: None,
            last_accessed_at: None,
            access_count: 0,
            metadata: serde_json::json!({}),
        })
    }

    /// Create a new access token with custom duration
    pub fn new_with_duration(
        tenant_id: TenantId,
        article_id: ArticleId,
        creator_id: CreatorId,
        reader_wallet: WalletAddress,
        transaction_id: Option<TransactionId>,
        access_method: AccessMethod,
        token_hash: String,
        duration_days: i64,
    ) -> Result<Self> {
        Self::validate_token_hash(&token_hash)?;

        if duration_days <= 0 || duration_days > 365 {
            return Err(crate::error::AllSourceError::ValidationError(
                "Access duration must be between 1 and 365 days".to_string(),
            ));
        }

        let now = Utc::now();
        let expires_at = now + Duration::days(duration_days);

        Ok(Self {
            id: AccessTokenId::new(),
            tenant_id,
            article_id,
            creator_id,
            reader_wallet,
            transaction_id,
            access_method,
            token_hash,
            issued_at: now,
            expires_at,
            revoked: false,
            revoked_at: None,
            revocation_reason: None,
            last_accessed_at: None,
            access_count: 0,
            metadata: serde_json::json!({}),
        })
    }

    /// Create a free access token (promotional)
    pub fn new_free(
        tenant_id: TenantId,
        article_id: ArticleId,
        creator_id: CreatorId,
        reader_wallet: WalletAddress,
        token_hash: String,
        duration_days: i64,
    ) -> Result<Self> {
        Self::new_with_duration(
            tenant_id,
            article_id,
            creator_id,
            reader_wallet,
            None,
            AccessMethod::Free,
            token_hash,
            duration_days,
        )
    }

    /// Reconstruct access token from storage (bypasses validation)
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: AccessTokenId,
        tenant_id: TenantId,
        article_id: ArticleId,
        creator_id: CreatorId,
        reader_wallet: WalletAddress,
        transaction_id: Option<TransactionId>,
        access_method: AccessMethod,
        token_hash: String,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        revoked: bool,
        revoked_at: Option<DateTime<Utc>>,
        revocation_reason: Option<String>,
        last_accessed_at: Option<DateTime<Utc>>,
        access_count: u32,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            tenant_id,
            article_id,
            creator_id,
            reader_wallet,
            transaction_id,
            access_method,
            token_hash,
            issued_at,
            expires_at,
            revoked,
            revoked_at,
            revocation_reason,
            last_accessed_at,
            access_count,
            metadata,
        }
    }

    // Getters

    pub fn id(&self) -> &AccessTokenId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn article_id(&self) -> &ArticleId {
        &self.article_id
    }

    pub fn creator_id(&self) -> &CreatorId {
        &self.creator_id
    }

    pub fn reader_wallet(&self) -> &WalletAddress {
        &self.reader_wallet
    }

    pub fn transaction_id(&self) -> Option<&TransactionId> {
        self.transaction_id.as_ref()
    }

    pub fn access_method(&self) -> AccessMethod {
        self.access_method
    }

    pub fn token_hash(&self) -> &str {
        &self.token_hash
    }

    pub fn issued_at(&self) -> DateTime<Utc> {
        self.issued_at
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked
    }

    pub fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }

    pub fn revocation_reason(&self) -> Option<&str> {
        self.revocation_reason.as_deref()
    }

    pub fn last_accessed_at(&self) -> Option<DateTime<Utc>> {
        self.last_accessed_at
    }

    pub fn access_count(&self) -> u32 {
        self.access_count
    }

    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    // Domain behavior methods

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if token grants access (not expired and not revoked)
    pub fn is_valid(&self) -> bool {
        !self.revoked && !self.is_expired()
    }

    /// Check if this token grants access to a specific article for a specific wallet
    pub fn grants_access_to(&self, article_id: &ArticleId, wallet: &WalletAddress) -> bool {
        self.is_valid() && &self.article_id == article_id && &self.reader_wallet == wallet
    }

    /// Get remaining access time in seconds
    pub fn remaining_seconds(&self) -> i64 {
        if self.is_expired() {
            0
        } else {
            (self.expires_at - Utc::now()).num_seconds().max(0)
        }
    }

    /// Get remaining access time in days
    pub fn remaining_days(&self) -> i64 {
        if self.is_expired() {
            0
        } else {
            (self.expires_at - Utc::now()).num_days().max(0)
        }
    }

    /// Record an access (reading the article)
    pub fn record_access(&mut self) {
        self.last_accessed_at = Some(Utc::now());
        self.access_count += 1;
    }

    /// Revoke the access token
    pub fn revoke(&mut self, reason: &str) -> Result<()> {
        if self.revoked {
            return Err(crate::error::AllSourceError::ValidationError(
                "Token is already revoked".to_string(),
            ));
        }

        self.revoked = true;
        self.revoked_at = Some(Utc::now());
        self.revocation_reason = Some(reason.to_string());
        Ok(())
    }

    /// Extend the expiration date
    pub fn extend(&mut self, additional_days: i64) -> Result<()> {
        if self.revoked {
            return Err(crate::error::AllSourceError::ValidationError(
                "Cannot extend a revoked token".to_string(),
            ));
        }

        if additional_days <= 0 || additional_days > 365 {
            return Err(crate::error::AllSourceError::ValidationError(
                "Extension must be between 1 and 365 days".to_string(),
            ));
        }

        self.expires_at = self.expires_at + Duration::days(additional_days);
        Ok(())
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
    }

    // Validation

    fn validate_token_hash(hash: &str) -> Result<()> {
        if hash.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Token hash cannot be empty".to_string(),
            ));
        }

        // Token hash should be reasonable length (typically 64 chars for SHA-256)
        if hash.len() < 32 || hash.len() > 128 {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Invalid token hash length: {}",
                hash.len()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_WALLET: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
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
        WalletAddress::new(VALID_WALLET.to_string()).unwrap()
    }

    fn test_transaction_id() -> TransactionId {
        TransactionId::new()
    }

    #[test]
    fn test_create_paid_access_token() {
        let token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        );

        assert!(token.is_ok());
        let token = token.unwrap();
        assert!(token.is_valid());
        assert!(!token.is_expired());
        assert!(!token.is_revoked());
        assert_eq!(token.access_method(), AccessMethod::Paid);
        assert!(token.transaction_id().is_some());
    }

    #[test]
    fn test_create_free_access_token() {
        let token = AccessToken::new_free(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            VALID_TOKEN_HASH.to_string(),
            7, // 7 days
        );

        assert!(token.is_ok());
        let token = token.unwrap();
        assert!(token.is_valid());
        assert_eq!(token.access_method(), AccessMethod::Free);
        assert!(token.transaction_id().is_none());
    }

    #[test]
    fn test_reject_empty_token_hash() {
        let result = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            "".to_string(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_invalid_duration() {
        let result = AccessToken::new_with_duration(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            None,
            AccessMethod::Free,
            VALID_TOKEN_HASH.to_string(),
            0, // Invalid duration
        );

        assert!(result.is_err());

        let result = AccessToken::new_with_duration(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            None,
            AccessMethod::Free,
            VALID_TOKEN_HASH.to_string(),
            400, // Too long
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_grants_access_to() {
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

        assert!(token.grants_access_to(&article_id, &wallet));

        // Different article
        let other_article = ArticleId::new("other-article".to_string()).unwrap();
        assert!(!token.grants_access_to(&other_article, &wallet));

        // Different wallet
        let other_wallet =
            WalletAddress::new("11111111111111111111111111111111".to_string()).unwrap();
        assert!(!token.grants_access_to(&article_id, &other_wallet));
    }

    #[test]
    fn test_revoke_token() {
        let mut token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();

        assert!(token.is_valid());

        token.revoke("Refund processed").unwrap();

        assert!(!token.is_valid());
        assert!(token.is_revoked());
        assert!(token.revoked_at().is_some());
        assert_eq!(token.revocation_reason(), Some("Refund processed"));
    }

    #[test]
    fn test_cannot_revoke_twice() {
        let mut token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();

        token.revoke("First revoke").unwrap();
        let result = token.revoke("Second revoke");
        assert!(result.is_err());
    }

    #[test]
    fn test_record_access() {
        let mut token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();

        assert_eq!(token.access_count(), 0);
        assert!(token.last_accessed_at().is_none());

        token.record_access();

        assert_eq!(token.access_count(), 1);
        assert!(token.last_accessed_at().is_some());

        token.record_access();
        assert_eq!(token.access_count(), 2);
    }

    #[test]
    fn test_extend_token() {
        let mut token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();

        let original_expiry = token.expires_at();
        token.extend(7).unwrap();

        assert!(token.expires_at() > original_expiry);
    }

    #[test]
    fn test_cannot_extend_revoked_token() {
        let mut token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();

        token.revoke("Revoked").unwrap();
        let result = token.extend(7);
        assert!(result.is_err());
    }

    #[test]
    fn test_remaining_time() {
        let token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();

        // Should have approximately 30 days remaining
        assert!(token.remaining_days() >= 29);
        assert!(token.remaining_seconds() > 0);
    }

    #[test]
    fn test_serde_serialization() {
        let token = AccessToken::new_paid(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            test_transaction_id(),
            VALID_TOKEN_HASH.to_string(),
        )
        .unwrap();

        let json = serde_json::to_string(&token);
        assert!(json.is_ok());

        let deserialized: AccessToken = serde_json::from_str(&json.unwrap()).unwrap();
        assert_eq!(deserialized.access_method(), AccessMethod::Paid);
    }
}
