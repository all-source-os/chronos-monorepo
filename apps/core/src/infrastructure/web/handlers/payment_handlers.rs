// Payment HTTP handlers - delegate to PaymentCoordinator
// Clean Architecture: Infrastructure Layer (HTTP) -> Application Layer (Coordinator)

use crate::{
    application::{
        dto::{
            AccessTokenDto, BlockchainDto, CheckAccessRequest, CheckAccessResponse,
            ConfirmTransactionRequest, ConfirmTransactionResponse, GrantAccessResponse,
            GrantFreeAccessRequest, InitiatePaymentRequest, InitiatePaymentResponse,
            ListAccessTokensResponse, ListTransactionsResponse, RefundTransactionRequest,
            RefundTransactionResponse, TransactionDto,
        },
        services::PaymentCoordinator,
        use_cases::{ListTransactionsUseCase, RefundTransactionUseCase},
    },
    domain::{
        entities::AccessToken,
        repositories::{
            AccessTokenRepository, ArticleRepository, CreatorRepository, TransactionRepository,
        },
        value_objects::{ArticleId, TransactionId, WalletAddress},
    },
    error::Result,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Application state for payment handlers
#[derive(Clone)]
pub struct PaymentHandlerState {
    pub coordinator: Arc<PaymentCoordinator>,
    pub transaction_repo: Arc<dyn TransactionRepository>,
    pub article_repo: Arc<dyn ArticleRepository>,
    pub creator_repo: Arc<dyn CreatorRepository>,
    pub access_token_repo: Arc<dyn AccessTokenRepository>,
}

/// Query parameters for listing transactions
#[derive(Debug, Deserialize)]
pub struct ListTransactionsParams {
    pub tenant_id: Option<String>,
    pub article_id: Option<String>,
    pub creator_id: Option<String>,
    pub reader_wallet: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Query parameters for listing access tokens
#[derive(Debug, Deserialize)]
pub struct ListAccessTokensParams {
    pub article_id: Option<String>,
    pub wallet_address: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Request for extending access
#[derive(Debug, Deserialize)]
pub struct ExtendAccessRequest {
    pub additional_days: i64,
}

/// Request for recording access
#[derive(Debug, Deserialize)]
pub struct RecordAccessRequest {
    pub token_id: String,
}

/// Response for purchase operation
#[derive(Debug, Serialize)]
pub struct PurchaseResponse {
    pub status: String,
    pub transaction: Option<TransactionDto>,
    pub access_token: Option<AccessTokenDto>,
    pub already_owned: bool,
}

/// Initiate a payment for an article
///
/// POST /api/v1/payments
pub async fn initiate_payment_handler(
    State(state): State<PaymentHandlerState>,
    Json(request): Json<InitiatePaymentRequest>,
) -> Result<Json<InitiatePaymentResponse>> {
    tracing::info!(
        article_id = %request.article_id,
        reader_wallet = %request.reader_wallet,
        "Initiating payment"
    );

    let response = state.coordinator.initiate_payment(request).await?;

    tracing::info!(
        transaction_id = %response.transaction.id,
        "Payment initiated"
    );

    Ok(Json(response))
}

/// Confirm a transaction and grant access
///
/// POST /api/v1/payments/{transaction_id}/confirm
pub async fn confirm_payment_handler(
    State(state): State<PaymentHandlerState>,
    Path(transaction_id): Path<String>,
) -> Result<Json<ConfirmTransactionResponse>> {
    tracing::info!(
        transaction_id = %transaction_id,
        "Confirming payment"
    );

    let response = state
        .coordinator
        .confirm_and_grant_access(ConfirmTransactionRequest {
            transaction_id: transaction_id.clone(),
        })
        .await?;

    tracing::info!(
        transaction_id = %transaction_id,
        "Payment confirmed"
    );

    Ok(Json(response))
}

/// Check if a user has access to an article
///
/// POST /api/v1/access/check
pub async fn check_access_handler(
    State(state): State<PaymentHandlerState>,
    Json(request): Json<CheckAccessRequest>,
) -> Result<Json<CheckAccessResponse>> {
    let response = state.coordinator.check_access(request).await?;
    Ok(Json(response))
}

/// Purchase an article (full flow: check -> pay -> confirm -> grant)
///
/// POST /api/v1/articles/{article_id}/purchase
#[derive(Debug, Deserialize)]
pub struct PurchaseArticleRequest {
    pub reader_wallet: String,
    pub tx_signature: String,
    pub blockchain: Option<BlockchainDto>,
}

#[derive(Debug, Deserialize)]
pub struct TenantIdParam {
    pub tenant_id: Option<String>,
}

pub async fn purchase_article_handler(
    State(state): State<PaymentHandlerState>,
    Path(article_id): Path<String>,
    Query(tenant_id): Query<TenantIdParam>,
    Json(request): Json<PurchaseArticleRequest>,
) -> Result<Json<PurchaseResponse>> {
    use crate::application::services::PurchaseResult;

    let tenant = tenant_id.tenant_id.unwrap_or_else(|| "default".to_string());

    tracing::info!(
        article_id = %article_id,
        reader_wallet = %request.reader_wallet,
        "Processing article purchase"
    );

    let result = state
        .coordinator
        .purchase_article(
            &tenant,
            &article_id,
            &request.reader_wallet,
            &request.tx_signature,
            request.blockchain,
        )
        .await?;

    let response = match result {
        PurchaseResult::AlreadyOwned { access_token } => PurchaseResponse {
            status: "already_owned".to_string(),
            transaction: None,
            access_token,
            already_owned: true,
        },
        PurchaseResult::Purchased {
            transaction,
            access_token_id: _,
        } => PurchaseResponse {
            status: "purchased".to_string(),
            transaction: Some(transaction),
            access_token: None,
            already_owned: false,
        },
    };

    Ok(Json(response))
}

/// List transactions with filtering
///
/// GET /api/v1/transactions
pub async fn list_transactions_handler(
    State(state): State<PaymentHandlerState>,
    Query(params): Query<ListTransactionsParams>,
) -> Result<Json<ListTransactionsResponse>> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    // Build query based on parameters
    let transactions = if let Some(article_id_str) = params.article_id {
        let article_id = ArticleId::new(article_id_str)?;
        state
            .transaction_repo
            .find_by_article(&article_id, limit, offset)
            .await?
    } else if let Some(creator_id_str) = params.creator_id {
        let creator_id = crate::domain::value_objects::CreatorId::parse(&creator_id_str)?;
        state
            .transaction_repo
            .find_by_creator(&creator_id, limit, offset)
            .await?
    } else if let Some(wallet_str) = params.reader_wallet {
        let wallet = WalletAddress::new(wallet_str)?;
        state
            .transaction_repo
            .find_by_reader(&wallet, limit, offset)
            .await?
    } else {
        // Default: return recent transactions
        use crate::domain::repositories::TransactionQuery;
        let query = TransactionQuery::new().with_pagination(limit, offset);
        state.transaction_repo.query(&query).await?
    };

    let response = ListTransactionsUseCase::execute(transactions);

    tracing::debug!(count = response.count, "Listed transactions");

    Ok(Json(response))
}

/// Get a specific transaction
///
/// GET /api/v1/transactions/{transaction_id}
pub async fn get_transaction_handler(
    State(state): State<PaymentHandlerState>,
    Path(transaction_id): Path<String>,
) -> Result<Json<TransactionDto>> {
    let id = TransactionId::parse(&transaction_id).map_err(|_| {
        crate::error::AllSourceError::InvalidInput("Invalid transaction ID".to_string())
    })?;

    let transaction = state
        .transaction_repo
        .find_by_id(&id)
        .await?
        .ok_or_else(|| {
            crate::error::AllSourceError::EntityNotFound(format!(
                "Transaction not found: {transaction_id}"
            ))
        })?;

    Ok(Json(TransactionDto::from(&transaction)))
}

/// Refund a transaction
///
/// POST /api/v1/transactions/{transaction_id}/refund
pub async fn refund_transaction_handler(
    State(state): State<PaymentHandlerState>,
    Path(transaction_id): Path<String>,
    Json(mut request): Json<RefundTransactionRequest>,
) -> Result<Json<RefundTransactionResponse>> {
    // Ensure the transaction ID matches the path parameter
    request.transaction_id = transaction_id.clone();

    let use_case = RefundTransactionUseCase::new(
        state.transaction_repo.clone(),
        state.access_token_repo.clone(),
    );

    let response = use_case.execute(request).await?;

    tracing::info!(
        transaction_id = %transaction_id,
        "Transaction refunded"
    );

    Ok(Json(response))
}

/// Grant free access to an article
///
/// POST /api/v1/access/grant-free
pub async fn grant_free_access_handler(
    State(state): State<PaymentHandlerState>,
    Json(request): Json<GrantFreeAccessRequest>,
) -> Result<Json<GrantAccessResponse>> {
    tracing::info!(
        article_id = %request.article_id,
        reader_wallet = %request.reader_wallet,
        "Granting free access"
    );

    // Validate and get article
    let article_id = ArticleId::new(request.article_id.clone())?;
    let article = state
        .article_repo
        .find_by_id(&article_id)
        .await?
        .ok_or_else(|| {
            crate::error::AllSourceError::EntityNotFound("Article not found".to_string())
        })?;

    // Parse tenant and wallet
    let tenant_id = crate::domain::value_objects::TenantId::new(request.tenant_id)?;
    let wallet = WalletAddress::new(request.reader_wallet)?;

    // Generate token hash
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(article_id.to_string().as_bytes());
    hasher.update(wallet.to_string().as_bytes());
    hasher.update(chrono::Utc::now().to_rfc3339().as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());

    // Create access token with specified duration
    let duration_days = request.duration_days.unwrap_or(365);
    let access_token = AccessToken::new_free(
        tenant_id,
        article_id,
        *article.creator_id(),
        wallet,
        token_hash.clone(),
        duration_days,
    )?;

    // Save access token
    state.access_token_repo.save(&access_token).await?;

    tracing::info!(
        access_token_id = %access_token.id().as_uuid(),
        "Free access granted"
    );

    Ok(Json(GrantAccessResponse {
        access_token: AccessTokenDto::from(&access_token),
        raw_token: token_hash,
    }))
}

/// List access tokens with filtering
///
/// GET /api/v1/access/tokens
pub async fn list_access_tokens_handler(
    State(state): State<PaymentHandlerState>,
    Query(params): Query<ListAccessTokensParams>,
) -> Result<Json<ListAccessTokensResponse>> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let tokens = if let Some(article_id_str) = params.article_id {
        let article_id = ArticleId::new(article_id_str)?;
        state
            .access_token_repo
            .find_by_article(&article_id, limit, offset)
            .await?
    } else if let Some(wallet_str) = params.wallet_address {
        let wallet = WalletAddress::new(wallet_str)?;
        state
            .access_token_repo
            .find_by_reader(&wallet, limit, offset)
            .await?
    } else {
        // Default: use query with pagination
        use crate::domain::repositories::AccessTokenQuery;
        let query = AccessTokenQuery::new().with_pagination(limit, offset);
        state.access_token_repo.query(&query).await?
    };

    let token_dtos: Vec<AccessTokenDto> = tokens.iter().map(AccessTokenDto::from).collect();
    let count = token_dtos.len();

    tracing::debug!(count = count, "Listed access tokens");

    Ok(Json(ListAccessTokensResponse {
        tokens: token_dtos,
        count,
    }))
}

/// Get a specific access token
///
/// GET /api/v1/access/tokens/{token_id}
pub async fn get_access_token_handler(
    State(state): State<PaymentHandlerState>,
    Path(token_id): Path<String>,
) -> Result<Json<AccessTokenDto>> {
    let id = uuid::Uuid::parse_str(&token_id)
        .map_err(|_| crate::error::AllSourceError::InvalidInput("Invalid token ID".to_string()))?;

    let token = state
        .access_token_repo
        .find_by_id(&crate::domain::entities::AccessTokenId::from_uuid(id))
        .await?
        .ok_or_else(|| {
            crate::error::AllSourceError::EntityNotFound(format!(
                "Access token not found: {token_id}"
            ))
        })?;

    Ok(Json(AccessTokenDto::from(&token)))
}

/// Revoke an access token
///
/// POST /api/v1/access/tokens/{token_id}/revoke
#[derive(Debug, Deserialize)]
pub struct RevokeAccessHandlerRequest {
    pub reason: String,
}

pub async fn revoke_access_handler(
    State(state): State<PaymentHandlerState>,
    Path(token_id): Path<String>,
    Json(request): Json<RevokeAccessHandlerRequest>,
) -> Result<Json<serde_json::Value>> {
    let id = uuid::Uuid::parse_str(&token_id)
        .map_err(|_| crate::error::AllSourceError::InvalidInput("Invalid token ID".to_string()))?;

    let revoked = state
        .access_token_repo
        .revoke(
            &crate::domain::entities::AccessTokenId::from_uuid(id),
            &request.reason,
        )
        .await?;

    tracing::info!(
        token_id = %token_id,
        revoked = revoked,
        "Access token revoke attempted"
    );

    Ok(Json(serde_json::json!({
        "revoked": revoked,
        "token_id": token_id
    })))
}

/// Cleanup expired access tokens
///
/// POST /api/v1/access/cleanup
pub async fn cleanup_expired_tokens_handler(
    State(state): State<PaymentHandlerState>,
) -> Result<Json<serde_json::Value>> {
    let count = state
        .access_token_repo
        .delete_expired(chrono::Utc::now())
        .await?;

    tracing::info!(tokens_cleaned = count, "Expired tokens cleaned up");

    Ok(Json(serde_json::json!({
        "tokens_cleaned": count
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_transactions_params_default() {
        let json = r#"{}"#;
        let params: ListTransactionsParams = serde_json::from_str(json).unwrap();
        assert!(params.tenant_id.is_none());
        assert!(params.article_id.is_none());
    }

    #[test]
    fn test_list_access_tokens_params() {
        let json = r#"{"article_id": "art-123", "limit": 10}"#;
        let params: ListAccessTokensParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.article_id, Some("art-123".to_string()));
        assert_eq!(params.limit, Some(10));
    }

    #[test]
    fn test_purchase_response_serialization() {
        let response = PurchaseResponse {
            status: "purchased".to_string(),
            transaction: None,
            access_token: None,
            already_owned: false,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("purchased"));
        assert!(json.contains("\"already_owned\":false"));
    }
}
