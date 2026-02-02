use crate::domain::entities::{Blockchain, Transaction, TransactionStatus};
use crate::domain::value_objects::{ArticleId, CreatorId, TenantId, TransactionId, WalletAddress};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Repository trait for transaction management
///
/// Provides persistent storage and retrieval operations for payment transactions.
///
/// # Responsibilities
/// - CRUD operations for transactions
/// - Transaction status updates
/// - Transaction lookup by various fields
/// - Revenue aggregation queries
///
/// # Thread Safety
/// Implementations must be thread-safe (Send + Sync).
#[async_trait]
pub trait TransactionRepository: Send + Sync {
    /// Save a new transaction
    ///
    /// # Arguments
    /// * `transaction` - The transaction to save
    ///
    /// # Errors
    /// - `ValidationError` - If a transaction with this signature already exists
    /// - `StorageError` - If the operation fails
    async fn save(&self, transaction: &Transaction) -> Result<()>;

    /// Find a transaction by ID
    ///
    /// # Arguments
    /// * `id` - The transaction ID to search for
    ///
    /// # Returns
    /// `Some(Transaction)` if found, `None` otherwise
    async fn find_by_id(&self, id: &TransactionId) -> Result<Option<Transaction>>;

    /// Find a transaction by blockchain signature
    ///
    /// # Arguments
    /// * `signature` - The blockchain transaction signature
    ///
    /// # Returns
    /// `Some(Transaction)` if found, `None` otherwise
    async fn find_by_signature(&self, signature: &str) -> Result<Option<Transaction>>;

    /// Find transactions by article
    ///
    /// # Arguments
    /// * `article_id` - The article ID
    /// * `limit` - Maximum number of transactions to return
    /// * `offset` - Number of transactions to skip
    ///
    /// # Returns
    /// Vector of transactions for this article
    async fn find_by_article(
        &self,
        article_id: &ArticleId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>>;

    /// Find transactions by creator
    ///
    /// # Arguments
    /// * `creator_id` - The creator ID
    /// * `limit` - Maximum number of transactions to return
    /// * `offset` - Number of transactions to skip
    ///
    /// # Returns
    /// Vector of transactions for this creator
    async fn find_by_creator(
        &self,
        creator_id: &CreatorId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>>;

    /// Find transactions by reader wallet
    ///
    /// # Arguments
    /// * `wallet` - The reader wallet address
    /// * `limit` - Maximum number of transactions to return
    /// * `offset` - Number of transactions to skip
    ///
    /// # Returns
    /// Vector of transactions for this reader
    async fn find_by_reader(
        &self,
        wallet: &WalletAddress,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>>;

    /// Find transactions by status
    ///
    /// # Arguments
    /// * `status` - The transaction status
    /// * `limit` - Maximum number of transactions to return
    /// * `offset` - Number of transactions to skip
    ///
    /// # Returns
    /// Vector of transactions with this status
    async fn find_by_status(
        &self,
        status: TransactionStatus,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>>;

    /// Count total transactions
    async fn count(&self) -> Result<usize>;

    /// Count transactions by status
    async fn count_by_status(&self, status: TransactionStatus) -> Result<usize>;

    /// Check if a transaction signature exists (prevent replay attacks)
    async fn signature_exists(&self, signature: &str) -> Result<bool> {
        Ok(self.find_by_signature(signature).await?.is_some())
    }

    /// Get total revenue for a creator in cents
    ///
    /// Only includes confirmed transactions.
    async fn get_creator_revenue(&self, creator_id: &CreatorId) -> Result<u64>;

    /// Get total revenue for an article in cents
    ///
    /// Only includes confirmed transactions.
    async fn get_article_revenue(&self, article_id: &ArticleId) -> Result<u64>;

    /// Query transactions with filters
    async fn query(&self, query: &TransactionQuery) -> Result<Vec<Transaction>>;

    /// Get revenue aggregated by time period
    async fn get_revenue_by_period(
        &self,
        creator_id: &CreatorId,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        granularity: RevenueGranularity,
    ) -> Result<Vec<RevenueDataPoint>>;
}

/// Query filter for finding transactions
#[derive(Debug, Clone, Default)]
pub struct TransactionQuery {
    pub tenant_id: Option<TenantId>,
    pub creator_id: Option<CreatorId>,
    pub article_id: Option<ArticleId>,
    pub reader_wallet: Option<WalletAddress>,
    pub status: Option<TransactionStatus>,
    pub blockchain: Option<Blockchain>,
    pub min_amount_cents: Option<u64>,
    pub max_amount_cents: Option<u64>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub confirmed_after: Option<DateTime<Utc>>,
    pub confirmed_before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl TransactionQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_tenant(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    pub fn for_creator(mut self, creator_id: CreatorId) -> Self {
        self.creator_id = Some(creator_id);
        self
    }

    pub fn for_article(mut self, article_id: ArticleId) -> Self {
        self.article_id = Some(article_id);
        self
    }

    pub fn for_reader(mut self, wallet: WalletAddress) -> Self {
        self.reader_wallet = Some(wallet);
        self
    }

    pub fn with_status(mut self, status: TransactionStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_blockchain(mut self, blockchain: Blockchain) -> Self {
        self.blockchain = Some(blockchain);
        self
    }

    pub fn with_amount_range(mut self, min_cents: u64, max_cents: u64) -> Self {
        self.min_amount_cents = Some(min_cents);
        self.max_amount_cents = Some(max_cents);
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

    pub fn confirmed_after(mut self, date: DateTime<Utc>) -> Self {
        self.confirmed_after = Some(date);
        self
    }

    pub fn confirmed_before(mut self, date: DateTime<Utc>) -> Self {
        self.confirmed_before = Some(date);
        self
    }

    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }

    /// Convenience: query for confirmed transactions only
    pub fn confirmed_only(self) -> Self {
        self.with_status(TransactionStatus::Confirmed)
    }
}

/// Time granularity for revenue aggregation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevenueGranularity {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

/// Revenue data point for analytics
#[derive(Debug, Clone)]
pub struct RevenueDataPoint {
    /// Start of the time period
    pub period_start: DateTime<Utc>,
    /// Total revenue in cents
    pub revenue_cents: u64,
    /// Number of transactions
    pub transaction_count: u64,
    /// Unique readers
    pub unique_readers: u64,
}

impl RevenueDataPoint {
    pub fn new(
        period_start: DateTime<Utc>,
        revenue_cents: u64,
        transaction_count: u64,
        unique_readers: u64,
    ) -> Self {
        Self {
            period_start,
            revenue_cents,
            transaction_count,
            unique_readers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_query_builder() {
        let query = TransactionQuery::new()
            .with_status(TransactionStatus::Confirmed)
            .with_pagination(10, 0);

        assert_eq!(query.status, Some(TransactionStatus::Confirmed));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_transaction_query_with_creator() {
        let creator_id = CreatorId::new();

        let query = TransactionQuery::new()
            .for_creator(creator_id)
            .confirmed_only();

        assert!(query.creator_id.is_some());
        assert_eq!(query.status, Some(TransactionStatus::Confirmed));
    }

    #[test]
    fn test_transaction_query_with_dates() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);

        let query = TransactionQuery::new()
            .created_after(yesterday)
            .created_before(now);

        assert!(query.created_after.is_some());
        assert!(query.created_before.is_some());
    }

    #[test]
    fn test_transaction_query_with_amount_range() {
        let query = TransactionQuery::new().with_amount_range(50, 500);

        assert_eq!(query.min_amount_cents, Some(50));
        assert_eq!(query.max_amount_cents, Some(500));
    }

    #[test]
    fn test_revenue_data_point() {
        let now = Utc::now();
        let data_point = RevenueDataPoint::new(now, 10000, 25, 20);

        assert_eq!(data_point.revenue_cents, 10000);
        assert_eq!(data_point.transaction_count, 25);
        assert_eq!(data_point.unique_readers, 20);
    }
}
