use crate::domain::entities::{Transaction, TransactionStatus};
use crate::domain::repositories::{
    RevenueDataPoint, RevenueGranularity, TransactionQuery, TransactionRepository,
};
use crate::domain::value_objects::{ArticleId, CreatorId, TransactionId, WalletAddress};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

/// In-memory implementation of TransactionRepository
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
/// use allsource_core::infrastructure::repositories::InMemoryTransactionRepository;
///
/// let repo = InMemoryTransactionRepository::new();
/// // Use repo with transaction use cases...
/// ```
pub struct InMemoryTransactionRepository {
    transactions: RwLock<HashMap<String, Transaction>>,
}

impl InMemoryTransactionRepository {
    /// Create a new empty in-memory transaction repository
    pub fn new() -> Self {
        Self {
            transactions: RwLock::new(HashMap::new()),
        }
    }

    /// Get the number of transactions stored
    pub fn len(&self) -> usize {
        self.transactions.read().unwrap().len()
    }

    /// Check if the repository is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.read().unwrap().is_empty()
    }

    /// Clear all transactions (for testing)
    pub fn clear(&self) {
        self.transactions.write().unwrap().clear();
    }
}

impl Default for InMemoryTransactionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransactionRepository for InMemoryTransactionRepository {
    async fn save(&self, transaction: &Transaction) -> Result<()> {
        let mut transactions = self.transactions.write().unwrap();
        let id = transaction.id().to_string();

        // Check for duplicate signature (on new transactions only)
        if !transactions.contains_key(&id) {
            let signature = transaction.tx_signature();
            if transactions.values().any(|t| t.tx_signature() == signature) {
                return Err(crate::error::AllSourceError::ValidationError(format!(
                    "Transaction with signature '{}' already exists (replay attack prevention)",
                    signature
                )));
            }
        }

        transactions.insert(id, transaction.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &TransactionId) -> Result<Option<Transaction>> {
        let transactions = self.transactions.read().unwrap();
        Ok(transactions.get(&id.to_string()).cloned())
    }

    async fn find_by_signature(&self, signature: &str) -> Result<Option<Transaction>> {
        let transactions = self.transactions.read().unwrap();
        Ok(transactions
            .values()
            .find(|t| t.tx_signature() == signature)
            .cloned())
    }

    async fn find_by_article(
        &self,
        article_id: &ArticleId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read().unwrap();
        let mut result: Vec<_> = transactions
            .values()
            .filter(|t| t.article_id() == article_id)
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_by_creator(
        &self,
        creator_id: &CreatorId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read().unwrap();
        let mut result: Vec<_> = transactions
            .values()
            .filter(|t| t.creator_id() == creator_id)
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_by_reader(
        &self,
        wallet: &WalletAddress,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read().unwrap();
        let mut result: Vec<_> = transactions
            .values()
            .filter(|t| t.reader_wallet() == wallet)
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_by_status(
        &self,
        status: TransactionStatus,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read().unwrap();
        let mut result: Vec<_> = transactions
            .values()
            .filter(|t| t.status() == status)
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.transactions.read().unwrap().len())
    }

    async fn count_by_status(&self, status: TransactionStatus) -> Result<usize> {
        let transactions = self.transactions.read().unwrap();
        Ok(transactions
            .values()
            .filter(|t| t.status() == status)
            .count())
    }

    async fn get_creator_revenue(&self, creator_id: &CreatorId) -> Result<u64> {
        let transactions = self.transactions.read().unwrap();
        Ok(transactions
            .values()
            .filter(|t| t.creator_id() == creator_id && t.is_confirmed())
            .map(|t| t.creator_amount_cents())
            .sum())
    }

    async fn get_article_revenue(&self, article_id: &ArticleId) -> Result<u64> {
        let transactions = self.transactions.read().unwrap();
        Ok(transactions
            .values()
            .filter(|t| t.article_id() == article_id && t.is_confirmed())
            .map(|t| t.amount_cents())
            .sum())
    }

    async fn query(&self, query: &TransactionQuery) -> Result<Vec<Transaction>> {
        let transactions = self.transactions.read().unwrap();
        let mut result: Vec<_> = transactions.values().cloned().collect();

        // Apply filters
        if let Some(ref tenant_id) = query.tenant_id {
            result.retain(|t| t.tenant_id() == tenant_id);
        }

        if let Some(ref creator_id) = query.creator_id {
            result.retain(|t| t.creator_id() == creator_id);
        }

        if let Some(ref article_id) = query.article_id {
            result.retain(|t| t.article_id() == article_id);
        }

        if let Some(ref wallet) = query.reader_wallet {
            result.retain(|t| t.reader_wallet() == wallet);
        }

        if let Some(status) = query.status {
            result.retain(|t| t.status() == status);
        }

        if let Some(blockchain) = query.blockchain {
            result.retain(|t| t.blockchain() == blockchain);
        }

        if let Some(min_amount) = query.min_amount_cents {
            result.retain(|t| t.amount_cents() >= min_amount);
        }

        if let Some(max_amount) = query.max_amount_cents {
            result.retain(|t| t.amount_cents() <= max_amount);
        }

        if let Some(date) = query.created_after {
            result.retain(|t| t.created_at() > date);
        }

        if let Some(date) = query.created_before {
            result.retain(|t| t.created_at() < date);
        }

        if let Some(date) = query.confirmed_after {
            result.retain(|t| t.confirmed_at().map(|c| c > date).unwrap_or(false));
        }

        if let Some(date) = query.confirmed_before {
            result.retain(|t| t.confirmed_at().map(|c| c < date).unwrap_or(false));
        }

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn get_revenue_by_period(
        &self,
        creator_id: &CreatorId,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        granularity: RevenueGranularity,
    ) -> Result<Vec<RevenueDataPoint>> {
        let transactions = self.transactions.read().unwrap();

        // Filter to confirmed transactions for this creator in the date range
        let relevant: Vec<_> = transactions
            .values()
            .filter(|t| {
                t.creator_id() == creator_id
                    && t.is_confirmed()
                    && t.created_at() >= start_date
                    && t.created_at() <= end_date
            })
            .collect();

        // Group by period
        let mut periods: HashMap<DateTime<Utc>, (u64, u64, HashSet<String>)> = HashMap::new();

        for tx in relevant {
            let period_start = Self::truncate_to_period(tx.created_at(), granularity);
            let entry = periods
                .entry(period_start)
                .or_insert((0, 0, HashSet::new()));
            entry.0 += tx.creator_amount_cents(); // revenue
            entry.1 += 1; // transaction count
            entry.2.insert(tx.reader_wallet().to_string()); // unique readers
        }

        // Convert to data points and sort by period
        let mut data_points: Vec<_> = periods
            .into_iter()
            .map(|(period_start, (revenue, count, readers))| {
                RevenueDataPoint::new(period_start, revenue, count, readers.len() as u64)
            })
            .collect();

        data_points.sort_by(|a, b| a.period_start.cmp(&b.period_start));

        Ok(data_points)
    }
}

impl InMemoryTransactionRepository {
    /// Truncate a datetime to the start of its period based on granularity
    fn truncate_to_period(dt: DateTime<Utc>, granularity: RevenueGranularity) -> DateTime<Utc> {
        match granularity {
            RevenueGranularity::Hourly => dt
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap(),
            RevenueGranularity::Daily => dt
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap(),
            RevenueGranularity::Weekly => {
                // Truncate to start of week (Monday)
                let days_from_monday = dt.weekday().num_days_from_monday();
                let monday = dt - Duration::days(days_from_monday as i64);
                monday
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap()
            }
            RevenueGranularity::Monthly => dt
                .with_day(1)
                .unwrap()
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Blockchain;
    use crate::domain::value_objects::{Money, TenantId};

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
    const VALID_SIGNATURE: &str =
        "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9g8eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9g";

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

    fn create_test_transaction(signature_suffix: &str) -> Transaction {
        Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7, // 7% fee
            Blockchain::Solana,
            format!("{}{}", &VALID_SIGNATURE[..80], signature_suffix),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let repo = InMemoryTransactionRepository::new();
        let transaction = create_test_transaction("test");
        let tx_id = transaction.id().clone();

        repo.save(&transaction).await.unwrap();

        let found = repo.find_by_id(&tx_id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_signature_rejected() {
        let repo = InMemoryTransactionRepository::new();
        let tx1 = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();
        repo.save(&tx1).await.unwrap();

        let tx2 = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(), // Same signature
        )
        .unwrap();

        let result = repo.save(&tx2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_by_signature() {
        let repo = InMemoryTransactionRepository::new();
        let transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();
        repo.save(&transaction).await.unwrap();

        let found = repo.find_by_signature(VALID_SIGNATURE).await.unwrap();
        assert!(found.is_some());

        let not_found = repo.find_by_signature("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_article() {
        let repo = InMemoryTransactionRepository::new();
        let article_id = test_article_id();

        for i in 0..3 {
            let tx = Transaction::new(
                test_tenant_id(),
                article_id.clone(),
                test_creator_id(),
                test_wallet_n(i),
                Money::usd_cents(100),
                7,
                Blockchain::Solana,
                format!("{}{}", &VALID_SIGNATURE[..80], format!("{:08}", i)),
            )
            .unwrap();
            repo.save(&tx).await.unwrap();
        }

        let transactions = repo.find_by_article(&article_id, 100, 0).await.unwrap();
        assert_eq!(transactions.len(), 3);
    }

    #[tokio::test]
    async fn test_find_by_creator() {
        let repo = InMemoryTransactionRepository::new();
        let creator_id = test_creator_id();

        for i in 0..3 {
            let tx = Transaction::new(
                test_tenant_id(),
                ArticleId::new(format!("article-{}", i)).unwrap(),
                creator_id,
                test_wallet(),
                Money::usd_cents(100),
                7,
                Blockchain::Solana,
                format!("{}{}", &VALID_SIGNATURE[..80], format!("{:08}", i)),
            )
            .unwrap();
            repo.save(&tx).await.unwrap();
        }

        let transactions = repo.find_by_creator(&creator_id, 100, 0).await.unwrap();
        assert_eq!(transactions.len(), 3);
    }

    #[tokio::test]
    async fn test_find_by_reader() {
        let repo = InMemoryTransactionRepository::new();
        let wallet = test_wallet();

        for i in 0..3 {
            let tx = Transaction::new(
                test_tenant_id(),
                ArticleId::new(format!("article-{}", i)).unwrap(),
                test_creator_id(),
                wallet.clone(),
                Money::usd_cents(100),
                7,
                Blockchain::Solana,
                format!("{}{}", &VALID_SIGNATURE[..80], format!("{:08}", i)),
            )
            .unwrap();
            repo.save(&tx).await.unwrap();
        }

        let transactions = repo.find_by_reader(&wallet, 100, 0).await.unwrap();
        assert_eq!(transactions.len(), 3);
    }

    #[tokio::test]
    async fn test_find_by_status() {
        let repo = InMemoryTransactionRepository::new();

        // Create pending transaction
        let pending = create_test_transaction("pending1");
        repo.save(&pending).await.unwrap();

        // Create and confirm a transaction
        let mut confirmed = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet_2(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            format!("{}{}", &VALID_SIGNATURE[..80], "confirm1"),
        )
        .unwrap();
        confirmed.confirm().unwrap();
        repo.save(&confirmed).await.unwrap();

        let pending_txs = repo
            .find_by_status(TransactionStatus::Pending, 100, 0)
            .await
            .unwrap();
        assert_eq!(pending_txs.len(), 1);

        let confirmed_txs = repo
            .find_by_status(TransactionStatus::Confirmed, 100, 0)
            .await
            .unwrap();
        assert_eq!(confirmed_txs.len(), 1);
    }

    #[tokio::test]
    async fn test_get_creator_revenue() {
        let repo = InMemoryTransactionRepository::new();
        let creator_id = test_creator_id();

        // Create and confirm transactions
        for i in 0..3 {
            let mut tx = Transaction::new(
                test_tenant_id(),
                ArticleId::new(format!("article-{}", i)).unwrap(),
                creator_id,
                test_wallet_n(i),
                Money::usd_cents(100), // $1.00
                7,                     // 7% fee
                Blockchain::Solana,
                format!("{}{}", &VALID_SIGNATURE[..80], format!("{:08}", i)),
            )
            .unwrap();
            tx.confirm().unwrap();
            repo.save(&tx).await.unwrap();
        }

        // Creator receives 93 cents per transaction (100 - 7)
        let revenue = repo.get_creator_revenue(&creator_id).await.unwrap();
        assert_eq!(revenue, 93 * 3);
    }

    #[tokio::test]
    async fn test_get_article_revenue() {
        let repo = InMemoryTransactionRepository::new();
        let article_id = test_article_id();

        // Create and confirm transactions
        for i in 0..3 {
            let mut tx = Transaction::new(
                test_tenant_id(),
                article_id.clone(),
                test_creator_id(),
                test_wallet_n(i),
                Money::usd_cents(100), // $1.00
                7,
                Blockchain::Solana,
                format!("{}{}", &VALID_SIGNATURE[..80], format!("{:08}", i)),
            )
            .unwrap();
            tx.confirm().unwrap();
            repo.save(&tx).await.unwrap();
        }

        // Article revenue is the full amount
        let revenue = repo.get_article_revenue(&article_id).await.unwrap();
        assert_eq!(revenue, 100 * 3);
    }

    #[tokio::test]
    async fn test_query_with_status() {
        let repo = InMemoryTransactionRepository::new();

        let pending = create_test_transaction("pending1");
        repo.save(&pending).await.unwrap();

        let mut confirmed = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet_2(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            format!("{}{}", &VALID_SIGNATURE[..80], "confirm1"),
        )
        .unwrap();
        confirmed.confirm().unwrap();
        repo.save(&confirmed).await.unwrap();

        let query = TransactionQuery::new().confirmed_only();
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_confirmed());
    }

    #[tokio::test]
    async fn test_query_with_amount_range() {
        let repo = InMemoryTransactionRepository::new();

        // Create transactions with different amounts
        let small = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(50),
            7,
            Blockchain::Solana,
            format!("{}{}", &VALID_SIGNATURE[..80], "small001"),
        )
        .unwrap();
        repo.save(&small).await.unwrap();

        let medium = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet_2(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            format!("{}{}", &VALID_SIGNATURE[..80], "medium01"),
        )
        .unwrap();
        repo.save(&medium).await.unwrap();

        let large = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            WalletAddress::new("22222222222222222222222222222222".to_string()).unwrap(),
            Money::usd_cents(500),
            7,
            Blockchain::Solana,
            format!("{}{}", &VALID_SIGNATURE[..80], "large001"),
        )
        .unwrap();
        repo.save(&large).await.unwrap();

        let query = TransactionQuery::new().with_amount_range(75, 150);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].amount_cents(), 100);
    }

    #[tokio::test]
    async fn test_query_with_pagination() {
        let repo = InMemoryTransactionRepository::new();

        for i in 0..5 {
            let tx = Transaction::new(
                test_tenant_id(),
                ArticleId::new(format!("article-{}", i)).unwrap(),
                test_creator_id(),
                test_wallet_n(i),
                Money::usd_cents(100),
                7,
                Blockchain::Solana,
                format!("{}{}", &VALID_SIGNATURE[..80], format!("{:08}", i)),
            )
            .unwrap();
            repo.save(&tx).await.unwrap();
        }

        let query = TransactionQuery::new().with_pagination(2, 1);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_get_revenue_by_period_daily() {
        let repo = InMemoryTransactionRepository::new();
        let creator_id = test_creator_id();
        let now = Utc::now();

        // Create and confirm transactions across different days
        for i in 0..3 {
            let mut tx = Transaction::new(
                test_tenant_id(),
                ArticleId::new(format!("article-{}", i)).unwrap(),
                creator_id,
                test_wallet_n(i),
                Money::usd_cents(100),
                7,
                Blockchain::Solana,
                format!("{}{}", &VALID_SIGNATURE[..80], format!("{:08}", i)),
            )
            .unwrap();
            tx.confirm().unwrap();
            repo.save(&tx).await.unwrap();
        }

        let start = now - Duration::days(7);
        let end = now + Duration::days(1);
        let data = repo
            .get_revenue_by_period(&creator_id, start, end, RevenueGranularity::Daily)
            .await
            .unwrap();

        // All transactions should be in one day
        assert!(!data.is_empty());
        let total_revenue: u64 = data.iter().map(|d| d.revenue_cents).sum();
        assert_eq!(total_revenue, 93 * 3); // 3 transactions, 93 cents each after 7% fee
    }

    #[tokio::test]
    async fn test_count_methods() {
        let repo = InMemoryTransactionRepository::new();

        // Create pending transaction
        let pending = create_test_transaction("pending1");
        repo.save(&pending).await.unwrap();

        // Create confirmed transaction
        let mut confirmed = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet_2(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            format!("{}{}", &VALID_SIGNATURE[..80], "confirm1"),
        )
        .unwrap();
        confirmed.confirm().unwrap();
        repo.save(&confirmed).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 2);
        assert_eq!(
            repo.count_by_status(TransactionStatus::Pending)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            repo.count_by_status(TransactionStatus::Confirmed)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn test_clear() {
        let repo = InMemoryTransactionRepository::new();

        for i in 0..3 {
            let tx = create_test_transaction(&format!("{:08}", i));
            repo.save(&tx).await.unwrap();
        }

        assert_eq!(repo.len(), 3);
        repo.clear();
        assert!(repo.is_empty());
    }

    #[tokio::test]
    async fn test_truncate_to_period() {
        let dt = Utc::now();

        // Test hourly truncation
        let hourly =
            InMemoryTransactionRepository::truncate_to_period(dt, RevenueGranularity::Hourly);
        assert_eq!(hourly.minute(), 0);
        assert_eq!(hourly.second(), 0);

        // Test daily truncation
        let daily =
            InMemoryTransactionRepository::truncate_to_period(dt, RevenueGranularity::Daily);
        assert_eq!(daily.hour(), 0);
        assert_eq!(daily.minute(), 0);

        // Test monthly truncation
        let monthly =
            InMemoryTransactionRepository::truncate_to_period(dt, RevenueGranularity::Monthly);
        assert_eq!(monthly.day(), 1);
        assert_eq!(monthly.hour(), 0);
    }
}
