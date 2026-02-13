use crate::{
    application::{
        dto::{
            AccessTokenDto, CheckAccessRequest, CheckAccessResponse, ConfirmTransactionRequest,
            ConfirmTransactionResponse, InitiatePaymentRequest, InitiatePaymentResponse,
            TransactionDto,
        },
        use_cases::{
            CheckAccessUseCase, ConfirmTransactionUseCase, GrantFreeAccessUseCase,
            InitiatePaymentUseCase,
        },
    },
    domain::repositories::{
        AccessTokenRepository, ArticleRepository, CreatorRepository, TransactionRepository,
    },
    error::Result,
};
use std::sync::Arc;

/// Coordinator service for payment operations.
///
/// This service orchestrates multiple use cases to handle complex payment
/// workflows like purchasing articles and managing access tokens.
pub struct PaymentCoordinator {
    initiate_payment: InitiatePaymentUseCase,
    confirm_transaction: ConfirmTransactionUseCase,
    check_access: CheckAccessUseCase,
    #[allow(dead_code)]
    grant_free_access: GrantFreeAccessUseCase,
}

impl PaymentCoordinator {
    pub fn new(
        transaction_repo: Arc<dyn TransactionRepository>,
        article_repo: Arc<dyn ArticleRepository>,
        creator_repo: Arc<dyn CreatorRepository>,
        access_token_repo: Arc<dyn AccessTokenRepository>,
    ) -> Self {
        Self {
            initiate_payment: InitiatePaymentUseCase::new(
                transaction_repo.clone(),
                article_repo.clone(),
                creator_repo.clone(),
            ),
            confirm_transaction: ConfirmTransactionUseCase::new(
                transaction_repo,
                access_token_repo.clone(),
                article_repo.clone(),
                creator_repo.clone(),
            ),
            check_access: CheckAccessUseCase::new(access_token_repo.clone()),
            grant_free_access: GrantFreeAccessUseCase::new(access_token_repo, article_repo),
        }
    }

    /// Initiate a payment for an article
    pub async fn initiate_payment(
        &self,
        request: InitiatePaymentRequest,
    ) -> Result<InitiatePaymentResponse> {
        self.initiate_payment.execute(request).await
    }

    /// Confirm a transaction and grant access
    pub async fn confirm_and_grant_access(
        &self,
        request: ConfirmTransactionRequest,
    ) -> Result<ConfirmTransactionResponse> {
        self.confirm_transaction.execute(request).await
    }

    /// Check if a user has access to an article
    pub async fn check_access(&self, request: CheckAccessRequest) -> Result<CheckAccessResponse> {
        self.check_access.execute(request).await
    }

    /// Full purchase flow: check access -> initiate payment -> confirm -> grant access
    ///
    /// This is a convenience method that handles the entire purchase flow.
    /// Returns the existing access if the user already has it, otherwise
    /// processes the payment and returns the new access.
    pub async fn purchase_article(
        &self,
        tenant_id: &str,
        article_id: &str,
        reader_wallet: &str,
        tx_signature: &str,
        blockchain: Option<crate::application::dto::BlockchainDto>,
    ) -> Result<PurchaseResult> {
        // First check if user already has access
        let access_check = self
            .check_access(CheckAccessRequest {
                article_id: article_id.to_string(),
                wallet_address: reader_wallet.to_string(),
            })
            .await?;

        if access_check.has_access {
            return Ok(PurchaseResult::AlreadyOwned {
                access_token: access_check.access_token,
            });
        }

        // Initiate the payment
        let payment_response = self
            .initiate_payment(InitiatePaymentRequest {
                tenant_id: tenant_id.to_string(),
                article_id: article_id.to_string(),
                reader_wallet: reader_wallet.to_string(),
                tx_signature: tx_signature.to_string(),
                blockchain,
            })
            .await?;

        // Confirm the transaction
        let confirm_response = self
            .confirm_and_grant_access(ConfirmTransactionRequest {
                transaction_id: payment_response.transaction.id.to_string(),
            })
            .await?;

        Ok(PurchaseResult::Purchased {
            transaction: confirm_response.transaction,
            access_token_id: confirm_response.access_token,
        })
    }
}

/// Result of a purchase operation
#[derive(Debug)]
pub enum PurchaseResult {
    /// User already has access to the article
    AlreadyOwned {
        access_token: Option<AccessTokenDto>,
    },
    /// Purchase was successful
    Purchased {
        transaction: TransactionDto,
        access_token_id: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_purchase_result_variants() {
        let owned = PurchaseResult::AlreadyOwned { access_token: None };
        assert!(matches!(owned, PurchaseResult::AlreadyOwned { .. }));
    }
}
