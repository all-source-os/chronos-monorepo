use crate::application::dto::{
    BlockchainDto, ConfirmTransactionRequest, ConfirmTransactionResponse, InitiatePaymentRequest,
    InitiatePaymentResponse, ListTransactionsResponse, RefundTransactionRequest,
    RefundTransactionResponse, TransactionDto,
};
use crate::domain::entities::{AccessToken, Transaction};
use crate::domain::repositories::{
    AccessTokenRepository, ArticleRepository, CreatorRepository, TransactionRepository,
};
use crate::domain::value_objects::{ArticleId, Money, TenantId, TransactionId, WalletAddress};
use crate::error::Result;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Use Case: Initiate Payment
///
/// Initiates a payment transaction for article access.
/// This creates a pending transaction that will be confirmed after on-chain verification.
///
/// Responsibilities:
/// - Validate input (DTO validation)
/// - Verify article exists and is purchasable
/// - Verify creator can receive payments
/// - Check for duplicate transaction signature (prevent replay attacks)
/// - Create domain Transaction entity
/// - Persist via repository
/// - Return response DTO
pub struct InitiatePaymentUseCase {
    transaction_repository: Arc<dyn TransactionRepository>,
    article_repository: Arc<dyn ArticleRepository>,
    creator_repository: Arc<dyn CreatorRepository>,
}

impl InitiatePaymentUseCase {
    pub fn new(
        transaction_repository: Arc<dyn TransactionRepository>,
        article_repository: Arc<dyn ArticleRepository>,
        creator_repository: Arc<dyn CreatorRepository>,
    ) -> Self {
        Self {
            transaction_repository,
            article_repository,
            creator_repository,
        }
    }

    pub async fn execute(
        &self,
        request: InitiatePaymentRequest,
    ) -> Result<InitiatePaymentResponse> {
        // Check for duplicate signature (replay attack prevention)
        if self
            .transaction_repository
            .signature_exists(&request.tx_signature)
            .await?
        {
            return Err(crate::error::AllSourceError::ValidationError(
                "Transaction signature already exists".to_string(),
            ));
        }

        // Parse IDs
        let tenant_id = TenantId::new(request.tenant_id)?;
        let article_id = ArticleId::new(request.article_id)?;
        let reader_wallet = WalletAddress::new(request.reader_wallet)?;

        // Verify article exists and is purchasable
        let article = self
            .article_repository
            .find_by_id(&article_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound("Article not found".to_string())
            })?;

        if !article.is_purchasable() {
            return Err(crate::error::AllSourceError::ValidationError(
                "Article is not available for purchase".to_string(),
            ));
        }

        // Verify creator can receive payments
        let creator = self
            .creator_repository
            .find_by_id(article.creator_id())
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound("Creator not found".to_string())
            })?;

        creator.can_receive_payments()?;

        // Determine blockchain
        let blockchain = request.blockchain.unwrap_or(BlockchainDto::Solana).into();

        // Create transaction
        let transaction = Transaction::new(
            tenant_id,
            article_id,
            *article.creator_id(),
            reader_wallet,
            Money::usd_cents(article.price_cents()),
            creator.fee_percentage(),
            blockchain,
            request.tx_signature,
        )?;

        // Persist transaction
        self.transaction_repository.save(&transaction).await?;

        Ok(InitiatePaymentResponse {
            transaction: TransactionDto::from(&transaction),
        })
    }
}

/// Use Case: Confirm Transaction
///
/// Confirms a transaction after on-chain verification.
/// This creates an access token for the reader.
pub struct ConfirmTransactionUseCase {
    transaction_repository: Arc<dyn TransactionRepository>,
    access_token_repository: Arc<dyn AccessTokenRepository>,
    article_repository: Arc<dyn ArticleRepository>,
    creator_repository: Arc<dyn CreatorRepository>,
}

impl ConfirmTransactionUseCase {
    pub fn new(
        transaction_repository: Arc<dyn TransactionRepository>,
        access_token_repository: Arc<dyn AccessTokenRepository>,
        article_repository: Arc<dyn ArticleRepository>,
        creator_repository: Arc<dyn CreatorRepository>,
    ) -> Self {
        Self {
            transaction_repository,
            access_token_repository,
            article_repository,
            creator_repository,
        }
    }

    pub async fn execute(
        &self,
        request: ConfirmTransactionRequest,
    ) -> Result<ConfirmTransactionResponse> {
        // Parse transaction ID
        let transaction_id = TransactionId::parse(&request.transaction_id)?;

        // Find transaction
        let mut transaction = self
            .transaction_repository
            .find_by_id(&transaction_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound("Transaction not found".to_string())
            })?;

        // Confirm transaction
        transaction.confirm()?;

        // Save updated transaction
        self.transaction_repository.save(&transaction).await?;

        // Generate access token
        let token_hash = generate_token_hash(&transaction);

        let access_token = AccessToken::new_paid(
            transaction.tenant_id().clone(),
            transaction.article_id().clone(),
            *transaction.creator_id(),
            transaction.reader_wallet().clone(),
            *transaction.id(),
            token_hash.clone(),
        )?;

        // Save access token
        self.access_token_repository.save(&access_token).await?;

        // Update article stats
        if let Some(mut article) = self
            .article_repository
            .find_by_id(transaction.article_id())
            .await?
        {
            article.record_purchase(transaction.amount_cents());
            self.article_repository.save(&article).await?;
        }

        // Update creator revenue
        if let Some(mut creator) = self
            .creator_repository
            .find_by_id(transaction.creator_id())
            .await?
        {
            creator.record_revenue(transaction.creator_amount_cents());
            self.creator_repository.save(&creator).await?;
        }

        Ok(ConfirmTransactionResponse {
            transaction: TransactionDto::from(&transaction),
            access_token: Some(token_hash),
        })
    }
}

/// Use Case: Fail Transaction
///
/// Marks a transaction as failed after on-chain verification fails.
pub struct FailTransactionUseCase;

impl FailTransactionUseCase {
    pub fn execute(mut transaction: Transaction, reason: &str) -> Result<TransactionDto> {
        transaction.fail(reason)?;
        Ok(TransactionDto::from(&transaction))
    }
}

/// Use Case: Refund Transaction
///
/// Processes a refund for a confirmed transaction.
/// This revokes the associated access token.
pub struct RefundTransactionUseCase {
    transaction_repository: Arc<dyn TransactionRepository>,
    access_token_repository: Arc<dyn AccessTokenRepository>,
}

impl RefundTransactionUseCase {
    pub fn new(
        transaction_repository: Arc<dyn TransactionRepository>,
        access_token_repository: Arc<dyn AccessTokenRepository>,
    ) -> Self {
        Self {
            transaction_repository,
            access_token_repository,
        }
    }

    pub async fn execute(
        &self,
        request: RefundTransactionRequest,
    ) -> Result<RefundTransactionResponse> {
        // Parse transaction ID
        let transaction_id = TransactionId::parse(&request.transaction_id)?;

        // Find transaction
        let mut transaction = self
            .transaction_repository
            .find_by_id(&transaction_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound("Transaction not found".to_string())
            })?;

        // Process refund
        transaction.refund(&request.refund_tx_signature)?;

        // Save updated transaction
        self.transaction_repository.save(&transaction).await?;

        // Revoke access token
        let reason = request
            .reason
            .unwrap_or_else(|| "Refund processed".to_string());
        self.access_token_repository
            .revoke_by_transaction(&transaction_id, &reason)
            .await?;

        Ok(RefundTransactionResponse {
            transaction: TransactionDto::from(&transaction),
        })
    }
}

/// Use Case: Dispute Transaction
///
/// Marks a transaction as disputed.
pub struct DisputeTransactionUseCase;

impl DisputeTransactionUseCase {
    pub fn execute(mut transaction: Transaction, reason: &str) -> Result<TransactionDto> {
        transaction.dispute(reason)?;
        Ok(TransactionDto::from(&transaction))
    }
}

/// Use Case: Resolve Dispute
///
/// Resolves a disputed transaction.
pub struct ResolveDisputeUseCase;

impl ResolveDisputeUseCase {
    pub fn execute(mut transaction: Transaction, resolution: &str) -> Result<TransactionDto> {
        transaction.resolve_dispute(resolution)?;
        Ok(TransactionDto::from(&transaction))
    }
}

/// Use Case: List Transactions
///
/// Returns a list of transactions.
pub struct ListTransactionsUseCase;

impl ListTransactionsUseCase {
    pub fn execute(transactions: Vec<Transaction>) -> ListTransactionsResponse {
        let transaction_dtos: Vec<TransactionDto> =
            transactions.iter().map(TransactionDto::from).collect();
        let count = transaction_dtos.len();

        ListTransactionsResponse {
            transactions: transaction_dtos,
            count,
        }
    }
}

/// Generate a secure token hash from transaction data
fn generate_token_hash(transaction: &Transaction) -> String {
    let mut hasher = Sha256::new();
    hasher.update(transaction.id().as_uuid().to_string().as_bytes());
    hasher.update(transaction.tx_signature().as_bytes());
    hasher.update(transaction.reader_wallet().to_string().as_bytes());
    hasher.update(transaction.article_id().to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        AccessTokenId, Blockchain, Creator, CreatorStatus, PaywallArticle, TransactionStatus,
    };
    use crate::domain::repositories::{
        AccessTokenQuery, ArticleQuery, CreatorQuery, RevenueDataPoint, RevenueGranularity,
        TransactionQuery,
    };
    use crate::domain::value_objects::CreatorId;
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::sync::Mutex;

    const VALID_WALLET: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
    const VALID_SIGNATURE: &str =
        "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9g8eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9g";

    struct MockTransactionRepository {
        transactions: Mutex<Vec<Transaction>>,
    }

    impl MockTransactionRepository {
        fn new() -> Self {
            Self {
                transactions: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl TransactionRepository for MockTransactionRepository {
        async fn save(&self, transaction: &Transaction) -> Result<()> {
            let mut transactions = self.transactions.lock().unwrap();
            if let Some(pos) = transactions.iter().position(|t| t.id() == transaction.id()) {
                transactions[pos] = transaction.clone();
            } else {
                transactions.push(transaction.clone());
            }
            Ok(())
        }

        async fn find_by_id(&self, id: &TransactionId) -> Result<Option<Transaction>> {
            let transactions = self.transactions.lock().unwrap();
            Ok(transactions.iter().find(|t| t.id() == id).cloned())
        }

        async fn find_by_signature(&self, signature: &str) -> Result<Option<Transaction>> {
            let transactions = self.transactions.lock().unwrap();
            Ok(transactions
                .iter()
                .find(|t| t.tx_signature() == signature)
                .cloned())
        }

        async fn find_by_article(
            &self,
            _article_id: &ArticleId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<Transaction>> {
            Ok(Vec::new())
        }

        async fn find_by_creator(
            &self,
            _creator_id: &CreatorId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<Transaction>> {
            Ok(Vec::new())
        }

        async fn find_by_reader(
            &self,
            _wallet: &WalletAddress,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<Transaction>> {
            Ok(Vec::new())
        }

        async fn find_by_status(
            &self,
            _status: TransactionStatus,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<Transaction>> {
            Ok(Vec::new())
        }

        async fn count(&self) -> Result<usize> {
            Ok(self.transactions.lock().unwrap().len())
        }

        async fn count_by_status(&self, _status: TransactionStatus) -> Result<usize> {
            Ok(0)
        }

        async fn get_creator_revenue(&self, _creator_id: &CreatorId) -> Result<u64> {
            Ok(0)
        }

        async fn get_article_revenue(&self, _article_id: &ArticleId) -> Result<u64> {
            Ok(0)
        }

        async fn query(&self, _query: &TransactionQuery) -> Result<Vec<Transaction>> {
            Ok(Vec::new())
        }

        async fn get_revenue_by_period(
            &self,
            _creator_id: &CreatorId,
            _start_date: DateTime<Utc>,
            _end_date: DateTime<Utc>,
            _granularity: RevenueGranularity,
        ) -> Result<Vec<RevenueDataPoint>> {
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
        async fn save(&self, article: &PaywallArticle) -> Result<()> {
            let mut articles = self.articles.lock().unwrap();
            if let Some(pos) = articles.iter().position(|a| a.id() == article.id()) {
                articles[pos] = article.clone();
            } else {
                articles.push(article.clone());
            }
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

    struct MockCreatorRepository {
        creators: Mutex<Vec<Creator>>,
    }

    impl MockCreatorRepository {
        fn new() -> Self {
            Self {
                creators: Mutex::new(Vec::new()),
            }
        }

        fn add_creator(&self, creator: Creator) {
            self.creators.lock().unwrap().push(creator);
        }
    }

    #[async_trait]
    impl CreatorRepository for MockCreatorRepository {
        async fn create(&self, creator: Creator) -> Result<Creator> {
            let mut creators = self.creators.lock().unwrap();
            creators.push(creator.clone());
            Ok(creator)
        }

        async fn save(&self, creator: &Creator) -> Result<()> {
            let mut creators = self.creators.lock().unwrap();
            if let Some(pos) = creators.iter().position(|c| c.id() == creator.id()) {
                creators[pos] = creator.clone();
            }
            Ok(())
        }

        async fn find_by_id(&self, id: &CreatorId) -> Result<Option<Creator>> {
            let creators = self.creators.lock().unwrap();
            Ok(creators.iter().find(|c| c.id() == id).cloned())
        }

        async fn find_by_email(&self, _email: &str) -> Result<Option<Creator>> {
            Ok(None)
        }

        async fn find_by_wallet(&self, _wallet: &WalletAddress) -> Result<Option<Creator>> {
            Ok(None)
        }

        async fn find_by_tenant(
            &self,
            _tenant_id: &TenantId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<Creator>> {
            Ok(Vec::new())
        }

        async fn find_active(&self, _limit: usize, _offset: usize) -> Result<Vec<Creator>> {
            Ok(Vec::new())
        }

        async fn count(&self) -> Result<usize> {
            Ok(0)
        }

        async fn count_by_status(&self, _status: CreatorStatus) -> Result<usize> {
            Ok(0)
        }

        async fn delete(&self, _id: &CreatorId) -> Result<bool> {
            Ok(false)
        }

        async fn query(&self, _query: &CreatorQuery) -> Result<Vec<Creator>> {
            Ok(Vec::new())
        }
    }

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

        async fn find_by_id(&self, _id: &AccessTokenId) -> Result<Option<AccessToken>> {
            Ok(None)
        }

        async fn find_by_hash(&self, _token_hash: &str) -> Result<Option<AccessToken>> {
            Ok(None)
        }

        async fn find_by_transaction(
            &self,
            _transaction_id: &TransactionId,
        ) -> Result<Option<AccessToken>> {
            Ok(None)
        }

        async fn find_by_article_and_wallet(
            &self,
            _article_id: &ArticleId,
            _wallet: &WalletAddress,
        ) -> Result<Vec<AccessToken>> {
            Ok(Vec::new())
        }

        async fn find_valid_token(
            &self,
            _article_id: &ArticleId,
            _wallet: &WalletAddress,
        ) -> Result<Option<AccessToken>> {
            Ok(None)
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
            Ok(0)
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

    #[tokio::test]
    async fn test_initiate_payment() {
        let tx_repo = Arc::new(MockTransactionRepository::new());
        let article_repo = Arc::new(MockArticleRepository::new());
        let creator_repo = Arc::new(MockCreatorRepository::new());

        // Create and add creator
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let wallet = WalletAddress::new(VALID_WALLET.to_string()).unwrap();
        let mut creator = Creator::new(
            tenant_id.clone(),
            "creator@example.com".to_string(),
            wallet,
            None,
        )
        .unwrap();
        creator.verify_email();
        let creator_id = *creator.id();
        creator_repo.add_creator(creator);

        // Create and add article
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
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

        let use_case =
            InitiatePaymentUseCase::new(tx_repo.clone(), article_repo.clone(), creator_repo);

        let request = InitiatePaymentRequest {
            tenant_id: "test-tenant".to_string(),
            article_id: "test-article".to_string(),
            reader_wallet: "11111111111111111111111111111111".to_string(),
            tx_signature: VALID_SIGNATURE.to_string(),
            blockchain: None,
        };

        let response = use_case.execute(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.transaction.amount_cents, 50);
        assert_eq!(
            response.transaction.status,
            crate::application::dto::TransactionStatusDto::Pending
        );
    }

    #[test]
    fn test_list_transactions() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
        let creator_id = CreatorId::new();
        let wallet = WalletAddress::new(VALID_WALLET.to_string()).unwrap();

        let transactions = vec![Transaction::new(
            tenant_id,
            article_id,
            creator_id,
            wallet,
            Money::usd_cents(50),
            10,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap()];

        let response = ListTransactionsUseCase::execute(transactions);
        assert_eq!(response.count, 1);
        assert_eq!(response.transactions.len(), 1);
    }
}
