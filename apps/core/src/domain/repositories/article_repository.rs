use crate::domain::entities::{ArticleStatus, PaywallArticle};
use crate::domain::value_objects::{ArticleId, CreatorId, TenantId};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Repository trait for paywall article management
///
/// Provides persistent storage and retrieval operations for paywalled articles.
///
/// # Responsibilities
/// - CRUD operations for articles
/// - Article status management
/// - Article lookup by various fields
/// - Statistics tracking
///
/// # Thread Safety
/// Implementations must be thread-safe (Send + Sync).
#[async_trait]
pub trait ArticleRepository: Send + Sync {
    /// Save a new article or update an existing one
    ///
    /// # Arguments
    /// * `article` - The article to save
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn save(&self, article: &PaywallArticle) -> Result<()>;

    /// Find an article by ID
    ///
    /// # Arguments
    /// * `id` - The article ID to search for
    ///
    /// # Returns
    /// `Some(PaywallArticle)` if found, `None` otherwise
    async fn find_by_id(&self, id: &ArticleId) -> Result<Option<PaywallArticle>>;

    /// Find an article by URL
    ///
    /// # Arguments
    /// * `url` - The article URL to search for
    ///
    /// # Returns
    /// `Some(PaywallArticle)` if found, `None` otherwise
    async fn find_by_url(&self, url: &str) -> Result<Option<PaywallArticle>>;

    /// Find articles by creator
    ///
    /// # Arguments
    /// * `creator_id` - The creator ID
    /// * `limit` - Maximum number of articles to return
    /// * `offset` - Number of articles to skip
    ///
    /// # Returns
    /// Vector of articles by this creator, ordered by creation date (newest first)
    async fn find_by_creator(
        &self,
        creator_id: &CreatorId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<PaywallArticle>>;

    /// Find articles by tenant
    ///
    /// # Arguments
    /// * `tenant_id` - The tenant ID
    /// * `limit` - Maximum number of articles to return
    /// * `offset` - Number of articles to skip
    ///
    /// # Returns
    /// Vector of articles in this tenant
    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<PaywallArticle>>;

    /// Find active (purchasable) articles by creator
    ///
    /// # Arguments
    /// * `creator_id` - The creator ID
    /// * `limit` - Maximum number of articles to return
    /// * `offset` - Number of articles to skip
    ///
    /// # Returns
    /// Vector of active articles by this creator
    async fn find_active_by_creator(
        &self,
        creator_id: &CreatorId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<PaywallArticle>>;

    /// Find articles by status
    ///
    /// # Arguments
    /// * `status` - The article status
    /// * `limit` - Maximum number of articles to return
    /// * `offset` - Number of articles to skip
    ///
    /// # Returns
    /// Vector of articles with this status
    async fn find_by_status(
        &self,
        status: ArticleStatus,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<PaywallArticle>>;

    /// Count total articles
    async fn count(&self) -> Result<usize>;

    /// Count articles by creator
    async fn count_by_creator(&self, creator_id: &CreatorId) -> Result<usize>;

    /// Count articles by status
    async fn count_by_status(&self, status: ArticleStatus) -> Result<usize>;

    /// Delete an article
    ///
    /// # Warning
    /// This performs a hard delete. Consider using soft delete (status = Deleted) instead.
    ///
    /// # Arguments
    /// * `id` - The article ID to delete
    ///
    /// # Returns
    /// `true` if the article was deleted, `false` if it didn't exist
    async fn delete(&self, id: &ArticleId) -> Result<bool>;

    /// Check if an article exists
    async fn exists(&self, id: &ArticleId) -> Result<bool> {
        Ok(self.find_by_id(id).await?.is_some())
    }

    /// Check if a URL is already registered
    async fn url_exists(&self, url: &str) -> Result<bool> {
        Ok(self.find_by_url(url).await?.is_some())
    }

    /// Query articles with filters
    async fn query(&self, query: &ArticleQuery) -> Result<Vec<PaywallArticle>>;

    /// Get top performing articles by revenue
    ///
    /// # Arguments
    /// * `creator_id` - Optional creator ID to filter by
    /// * `limit` - Maximum number of articles to return
    ///
    /// # Returns
    /// Vector of articles ordered by total revenue (highest first)
    async fn find_top_by_revenue(
        &self,
        creator_id: Option<&CreatorId>,
        limit: usize,
    ) -> Result<Vec<PaywallArticle>>;

    /// Get recently published articles
    ///
    /// # Arguments
    /// * `creator_id` - Optional creator ID to filter by
    /// * `limit` - Maximum number of articles to return
    ///
    /// # Returns
    /// Vector of recently published articles
    async fn find_recent(
        &self,
        creator_id: Option<&CreatorId>,
        limit: usize,
    ) -> Result<Vec<PaywallArticle>>;
}

/// Query filter for finding articles
#[derive(Debug, Clone, Default)]
pub struct ArticleQuery {
    pub tenant_id: Option<TenantId>,
    pub creator_id: Option<CreatorId>,
    pub status: Option<ArticleStatus>,
    pub title_contains: Option<String>,
    pub url_contains: Option<String>,
    pub min_price_cents: Option<u64>,
    pub max_price_cents: Option<u64>,
    pub min_purchases: Option<u64>,
    pub min_revenue_cents: Option<u64>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub published_after: Option<DateTime<Utc>>,
    pub published_before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub order_by: Option<ArticleOrderBy>,
}

/// Ordering options for article queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArticleOrderBy {
    /// Order by creation date (newest first)
    CreatedAtDesc,
    /// Order by creation date (oldest first)
    CreatedAtAsc,
    /// Order by published date (newest first)
    PublishedAtDesc,
    /// Order by total revenue (highest first)
    RevenueDesc,
    /// Order by total purchases (highest first)
    PurchasesDesc,
    /// Order by price (highest first)
    PriceDesc,
    /// Order by price (lowest first)
    PriceAsc,
    /// Order by title alphabetically
    TitleAsc,
}

impl Default for ArticleOrderBy {
    fn default() -> Self {
        Self::CreatedAtDesc
    }
}

impl ArticleQuery {
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

    pub fn with_status(mut self, status: ArticleStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn active_only(mut self) -> Self {
        self.status = Some(ArticleStatus::Active);
        self
    }

    pub fn with_title_filter(mut self, title: String) -> Self {
        self.title_contains = Some(title);
        self
    }

    pub fn with_url_filter(mut self, url: String) -> Self {
        self.url_contains = Some(url);
        self
    }

    pub fn with_price_range(mut self, min_cents: u64, max_cents: u64) -> Self {
        self.min_price_cents = Some(min_cents);
        self.max_price_cents = Some(max_cents);
        self
    }

    pub fn with_min_purchases(mut self, min_purchases: u64) -> Self {
        self.min_purchases = Some(min_purchases);
        self
    }

    pub fn with_min_revenue(mut self, min_cents: u64) -> Self {
        self.min_revenue_cents = Some(min_cents);
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

    pub fn published_after(mut self, date: DateTime<Utc>) -> Self {
        self.published_after = Some(date);
        self
    }

    pub fn published_before(mut self, date: DateTime<Utc>) -> Self {
        self.published_before = Some(date);
        self
    }

    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }

    pub fn order_by(mut self, order: ArticleOrderBy) -> Self {
        self.order_by = Some(order);
        self
    }

    pub fn order_by_revenue(mut self) -> Self {
        self.order_by = Some(ArticleOrderBy::RevenueDesc);
        self
    }

    pub fn order_by_purchases(mut self) -> Self {
        self.order_by = Some(ArticleOrderBy::PurchasesDesc);
        self
    }

    pub fn order_by_recent(mut self) -> Self {
        self.order_by = Some(ArticleOrderBy::PublishedAtDesc);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_query_builder() {
        let query = ArticleQuery::new().active_only().with_pagination(10, 0);

        assert_eq!(query.status, Some(ArticleStatus::Active));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_article_query_for_creator() {
        let creator_id = CreatorId::new();

        let query = ArticleQuery::new()
            .for_creator(creator_id)
            .active_only()
            .order_by_revenue();

        assert!(query.creator_id.is_some());
        assert_eq!(query.status, Some(ArticleStatus::Active));
        assert_eq!(query.order_by, Some(ArticleOrderBy::RevenueDesc));
    }

    #[test]
    fn test_article_query_with_dates() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);

        let query = ArticleQuery::new()
            .published_after(yesterday)
            .published_before(now);

        assert!(query.published_after.is_some());
        assert!(query.published_before.is_some());
    }

    #[test]
    fn test_article_query_with_price_range() {
        let query = ArticleQuery::new().with_price_range(50, 500);

        assert_eq!(query.min_price_cents, Some(50));
        assert_eq!(query.max_price_cents, Some(500));
    }

    #[test]
    fn test_article_order_by_default() {
        let order = ArticleOrderBy::default();
        assert_eq!(order, ArticleOrderBy::CreatedAtDesc);
    }
}
