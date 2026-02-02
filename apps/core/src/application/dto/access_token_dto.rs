use crate::domain::entities::{AccessMethod, AccessToken};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DTO for granting free access
#[derive(Debug, Deserialize)]
pub struct GrantFreeAccessRequest {
    pub tenant_id: String,
    pub article_id: String,
    pub reader_wallet: String,
    pub duration_days: Option<i64>,
    pub reason: Option<String>,
}

/// DTO for granting access response
#[derive(Debug, Serialize)]
pub struct GrantAccessResponse {
    pub access_token: AccessTokenDto,
    pub raw_token: String,
}

/// DTO for checking access
#[derive(Debug, Deserialize)]
pub struct CheckAccessRequest {
    pub article_id: String,
    pub wallet_address: String,
}

/// DTO for access check response
#[derive(Debug, Serialize)]
pub struct CheckAccessResponse {
    pub has_access: bool,
    pub access_token: Option<AccessTokenDto>,
    pub remaining_days: Option<i64>,
}

/// DTO for revoking access
#[derive(Debug, Deserialize)]
pub struct RevokeAccessRequest {
    pub token_id: String,
    pub reason: String,
}

/// DTO for revoke access response
#[derive(Debug, Serialize)]
pub struct RevokeAccessResponse {
    pub revoked: bool,
    pub access_token: AccessTokenDto,
}

/// DTO for listing access tokens response
#[derive(Debug, Serialize)]
pub struct ListAccessTokensResponse {
    pub tokens: Vec<AccessTokenDto>,
    pub count: usize,
}

/// DTO for access method
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessMethodDto {
    Paid,
    Bundle,
    Free,
    Subscription,
}

impl From<AccessMethod> for AccessMethodDto {
    fn from(method: AccessMethod) -> Self {
        match method {
            AccessMethod::Paid => AccessMethodDto::Paid,
            AccessMethod::Bundle => AccessMethodDto::Bundle,
            AccessMethod::Free => AccessMethodDto::Free,
            AccessMethod::Subscription => AccessMethodDto::Subscription,
        }
    }
}

impl From<AccessMethodDto> for AccessMethod {
    fn from(dto: AccessMethodDto) -> Self {
        match dto {
            AccessMethodDto::Paid => AccessMethod::Paid,
            AccessMethodDto::Bundle => AccessMethod::Bundle,
            AccessMethodDto::Free => AccessMethod::Free,
            AccessMethodDto::Subscription => AccessMethod::Subscription,
        }
    }
}

/// DTO for an access token in responses
#[derive(Debug, Clone, Serialize)]
pub struct AccessTokenDto {
    pub id: Uuid,
    pub tenant_id: String,
    pub article_id: String,
    pub creator_id: String,
    pub reader_wallet: String,
    pub transaction_id: Option<Uuid>,
    pub access_method: AccessMethodDto,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_valid: bool,
    pub is_expired: bool,
    pub is_revoked: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revocation_reason: Option<String>,
    pub remaining_days: i64,
    pub access_count: u32,
    pub last_accessed_at: Option<DateTime<Utc>>,
}

impl From<&AccessToken> for AccessTokenDto {
    fn from(token: &AccessToken) -> Self {
        Self {
            id: token.id().as_uuid(),
            tenant_id: token.tenant_id().to_string(),
            article_id: token.article_id().to_string(),
            creator_id: token.creator_id().as_uuid().to_string(),
            reader_wallet: token.reader_wallet().to_string(),
            transaction_id: token.transaction_id().map(|t| t.as_uuid()),
            access_method: token.access_method().into(),
            issued_at: token.issued_at(),
            expires_at: token.expires_at(),
            is_valid: token.is_valid(),
            is_expired: token.is_expired(),
            is_revoked: token.is_revoked(),
            revoked_at: token.revoked_at(),
            revocation_reason: token.revocation_reason().map(|s| s.to_string()),
            remaining_days: token.remaining_days(),
            access_count: token.access_count(),
            last_accessed_at: token.last_accessed_at(),
        }
    }
}

impl From<AccessToken> for AccessTokenDto {
    fn from(token: AccessToken) -> Self {
        AccessTokenDto::from(&token)
    }
}
