use crate::domain::entities::{ArticleStatus, PaywallArticle};
use crate::domain::repositories::{ArticleOrderBy, ArticleQuery, ArticleRepository};
use crate::domain::value_objects::{ArticleId, CreatorId, TenantId};
use crate::error::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory implementation of ArticleRepository
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
/// use allsource_core::infrastructure::repositories::InMemoryArticleRepository;
///
/// let repo = InMemoryArticleRepository::new();
/// // Use repo with article use cases...
/// ```
pub struct InMemoryArticleRepository {
    articles: RwLock<HashMap<String, PaywallArticle>>,
}

impl InMemoryArticleRepository {
    /// Create a new empty in-memory article repository
    pub fn new() -> Self {
        Self {
            articles: RwLock::new(HashMap::new()),
        }
    }

    /// Get the number of articles stored
    pub fn len(&self) -> usize {
        self.articles.read().unwrap().len()
    }

    /// Check if the repository is empty
    pub fn is_empty(&self) -> bool {
        self.articles.read().unwrap().is_empty()
    }

    /// Clear all articles (for testing)
    pub fn clear(&self) {
        self.articles.write().unwrap().clear();
    }
}

impl Default for InMemoryArticleRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ArticleRepository for InMemoryArticleRepository {
    async fn save(&self, article: &PaywallArticle) -> Result<()> {
        let mut articles = self.articles.write().unwrap();
        articles.insert(article.id().to_string(), article.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &ArticleId) -> Result<Option<PaywallArticle>> {
        let articles = self.articles.read().unwrap();
        Ok(articles.get(&id.to_string()).cloned())
    }

    async fn find_by_url(&self, url: &str) -> Result<Option<PaywallArticle>> {
        let articles = self.articles.read().unwrap();
        Ok(articles.values().find(|a| a.url() == url).cloned())
    }

    async fn find_by_creator(
        &self,
        creator_id: &CreatorId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<PaywallArticle>> {
        let articles = self.articles.read().unwrap();
        let mut result: Vec<_> = articles
            .values()
            .filter(|a| a.creator_id() == creator_id)
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<PaywallArticle>> {
        let articles = self.articles.read().unwrap();
        let mut result: Vec<_> = articles
            .values()
            .filter(|a| a.tenant_id() == tenant_id)
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_active_by_creator(
        &self,
        creator_id: &CreatorId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<PaywallArticle>> {
        let articles = self.articles.read().unwrap();
        let mut result: Vec<_> = articles
            .values()
            .filter(|a| a.creator_id() == creator_id && a.is_purchasable())
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_by_status(
        &self,
        status: ArticleStatus,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<PaywallArticle>> {
        let articles = self.articles.read().unwrap();
        let mut result: Vec<_> = articles
            .values()
            .filter(|a| a.status() == status)
            .cloned()
            .collect();

        // Sort by created_at descending (newest first)
        result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.articles.read().unwrap().len())
    }

    async fn count_by_creator(&self, creator_id: &CreatorId) -> Result<usize> {
        let articles = self.articles.read().unwrap();
        Ok(articles
            .values()
            .filter(|a| a.creator_id() == creator_id)
            .count())
    }

    async fn count_by_status(&self, status: ArticleStatus) -> Result<usize> {
        let articles = self.articles.read().unwrap();
        Ok(articles.values().filter(|a| a.status() == status).count())
    }

    async fn delete(&self, id: &ArticleId) -> Result<bool> {
        let mut articles = self.articles.write().unwrap();
        Ok(articles.remove(&id.to_string()).is_some())
    }

    async fn query(&self, query: &ArticleQuery) -> Result<Vec<PaywallArticle>> {
        let articles = self.articles.read().unwrap();
        let mut result: Vec<_> = articles.values().cloned().collect();

        // Apply filters
        if let Some(ref tenant_id) = query.tenant_id {
            result.retain(|a| a.tenant_id() == tenant_id);
        }

        if let Some(ref creator_id) = query.creator_id {
            result.retain(|a| a.creator_id() == creator_id);
        }

        if let Some(status) = query.status {
            result.retain(|a| a.status() == status);
        }

        if let Some(ref title_pattern) = query.title_contains {
            result.retain(|a| {
                a.title()
                    .to_lowercase()
                    .contains(&title_pattern.to_lowercase())
            });
        }

        if let Some(ref url_pattern) = query.url_contains {
            result.retain(|a| a.url().to_lowercase().contains(&url_pattern.to_lowercase()));
        }

        if let Some(min_price) = query.min_price_cents {
            result.retain(|a| a.price_cents() >= min_price);
        }

        if let Some(max_price) = query.max_price_cents {
            result.retain(|a| a.price_cents() <= max_price);
        }

        if let Some(min_purchases) = query.min_purchases {
            result.retain(|a| a.stats().total_purchases >= min_purchases);
        }

        if let Some(min_revenue) = query.min_revenue_cents {
            result.retain(|a| a.stats().total_revenue_cents >= min_revenue);
        }

        if let Some(date) = query.created_after {
            result.retain(|a| a.created_at() > date);
        }

        if let Some(date) = query.created_before {
            result.retain(|a| a.created_at() < date);
        }

        if let Some(date) = query.published_after {
            result.retain(|a| a.published_at().map(|p| p > date).unwrap_or(false));
        }

        if let Some(date) = query.published_before {
            result.retain(|a| a.published_at().map(|p| p < date).unwrap_or(false));
        }

        // Apply ordering
        let order = query.order_by.unwrap_or_default();
        match order {
            ArticleOrderBy::CreatedAtDesc => {
                result.sort_by_key(|b| std::cmp::Reverse(b.created_at()));
            }
            ArticleOrderBy::CreatedAtAsc => {
                result.sort_by_key(|a| a.created_at());
            }
            ArticleOrderBy::PublishedAtDesc => {
                result.sort_by_key(|b| std::cmp::Reverse(b.published_at()));
            }
            ArticleOrderBy::RevenueDesc => {
                result.sort_by(|a, b| {
                    b.stats()
                        .total_revenue_cents
                        .cmp(&a.stats().total_revenue_cents)
                });
            }
            ArticleOrderBy::PurchasesDesc => {
                result.sort_by_key(|a| std::cmp::Reverse(a.stats().total_purchases));
            }
            ArticleOrderBy::PriceDesc => {
                result.sort_by_key(|b| std::cmp::Reverse(b.price_cents()));
            }
            ArticleOrderBy::PriceAsc => {
                result.sort_by_key(|a| a.price_cents());
            }
            ArticleOrderBy::TitleAsc => {
                result.sort_by(|a, b| a.title().cmp(b.title()));
            }
        }

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(result.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_top_by_revenue(
        &self,
        creator_id: Option<&CreatorId>,
        limit: usize,
    ) -> Result<Vec<PaywallArticle>> {
        let articles = self.articles.read().unwrap();
        let mut result: Vec<_> = articles
            .values()
            .filter(|a| creator_id.is_none() || a.creator_id() == creator_id.unwrap())
            .filter(|a| a.is_purchasable())
            .cloned()
            .collect();

        // Sort by revenue descending
        result.sort_by(|a, b| {
            b.stats()
                .total_revenue_cents
                .cmp(&a.stats().total_revenue_cents)
        });

        Ok(result.into_iter().take(limit).collect())
    }

    async fn find_recent(
        &self,
        creator_id: Option<&CreatorId>,
        limit: usize,
    ) -> Result<Vec<PaywallArticle>> {
        let articles = self.articles.read().unwrap();
        let mut result: Vec<_> = articles
            .values()
            .filter(|a| creator_id.is_none() || a.creator_id() == creator_id.unwrap())
            .filter(|a| a.published_at().is_some())
            .cloned()
            .collect();

        // Sort by published_at descending
        result.sort_by_key(|b| std::cmp::Reverse(b.published_at()));

        Ok(result.into_iter().take(limit).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::ArticleId;

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn test_article_id(slug: &str) -> ArticleId {
        ArticleId::new(slug.to_string()).unwrap()
    }

    fn test_creator_id() -> CreatorId {
        CreatorId::new()
    }

    fn create_test_article(slug: &str, title: &str, creator_id: CreatorId) -> PaywallArticle {
        PaywallArticle::new(
            test_article_id(slug),
            test_tenant_id(),
            creator_id,
            title.to_string(),
            format!("https://example.com/{}", slug),
            50, // $0.50
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();
        let article = create_test_article("test-article", "Test Article", creator_id);
        let article_id = article.id().clone();

        repo.save(&article).await.unwrap();

        let found = repo.find_by_id(&article_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title(), "Test Article");
    }

    #[tokio::test]
    async fn test_find_by_url() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();
        let article = create_test_article("my-article", "My Article", creator_id);

        repo.save(&article).await.unwrap();

        let found = repo
            .find_by_url("https://example.com/my-article")
            .await
            .unwrap();
        assert!(found.is_some());

        let not_found = repo
            .find_by_url("https://example.com/non-existent")
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_creator() {
        let repo = InMemoryArticleRepository::new();
        let creator1 = test_creator_id();
        let creator2 = test_creator_id();

        // Create articles for creator1
        for i in 0..3 {
            let article = create_test_article(
                &format!("article-{}", i),
                &format!("Article {}", i),
                creator1,
            );
            repo.save(&article).await.unwrap();
        }

        // Create article for creator2
        let other_article = create_test_article("other-article", "Other Article", creator2);
        repo.save(&other_article).await.unwrap();

        let articles = repo.find_by_creator(&creator1, 100, 0).await.unwrap();
        assert_eq!(articles.len(), 3);
    }

    #[tokio::test]
    async fn test_find_by_tenant() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        for i in 0..3 {
            let article = create_test_article(
                &format!("article-{}", i),
                &format!("Article {}", i),
                creator_id,
            );
            repo.save(&article).await.unwrap();
        }

        let articles = repo
            .find_by_tenant(&test_tenant_id(), 100, 0)
            .await
            .unwrap();
        assert_eq!(articles.len(), 3);
    }

    #[tokio::test]
    async fn test_find_active_by_creator() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        // Create active article
        let active = create_test_article("active-article", "Active Article", creator_id);
        repo.save(&active).await.unwrap();

        // Create and archive an article
        let mut archived = create_test_article("archived-article", "Archived Article", creator_id);
        archived.archive();
        repo.save(&archived).await.unwrap();

        let active_articles = repo
            .find_active_by_creator(&creator_id, 100, 0)
            .await
            .unwrap();
        assert_eq!(active_articles.len(), 1);
        assert_eq!(active_articles[0].title(), "Active Article");
    }

    #[tokio::test]
    async fn test_find_by_status() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        // Create active article
        let active = create_test_article("active-article", "Active Article", creator_id);
        repo.save(&active).await.unwrap();

        // Create draft article
        let draft = PaywallArticle::new_draft(
            test_article_id("draft-article"),
            test_tenant_id(),
            creator_id,
            "Draft Article".to_string(),
            "https://example.com/draft-article".to_string(),
            50,
        )
        .unwrap();
        repo.save(&draft).await.unwrap();

        let actives = repo
            .find_by_status(ArticleStatus::Active, 100, 0)
            .await
            .unwrap();
        assert_eq!(actives.len(), 1);

        let drafts = repo
            .find_by_status(ArticleStatus::Draft, 100, 0)
            .await
            .unwrap();
        assert_eq!(drafts.len(), 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();
        let article = create_test_article("test-article", "Test Article", creator_id);
        let article_id = article.id().clone();

        repo.save(&article).await.unwrap();
        assert_eq!(repo.len(), 1);

        let deleted = repo.delete(&article_id).await.unwrap();
        assert!(deleted);
        assert_eq!(repo.len(), 0);

        let deleted_again = repo.delete(&article_id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_query_with_status() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        let active = create_test_article("active", "Active", creator_id);
        repo.save(&active).await.unwrap();

        let draft = PaywallArticle::new_draft(
            test_article_id("draft"),
            test_tenant_id(),
            creator_id,
            "Draft".to_string(),
            "https://example.com/draft".to_string(),
            50,
        )
        .unwrap();
        repo.save(&draft).await.unwrap();

        let query = ArticleQuery::new().active_only();
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title(), "Active");
    }

    #[tokio::test]
    async fn test_query_with_price_range() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        // Create articles with different prices
        let cheap = PaywallArticle::new(
            test_article_id("cheap"),
            test_tenant_id(),
            creator_id,
            "Cheap Article".to_string(),
            "https://example.com/cheap".to_string(),
            20, // $0.20
        )
        .unwrap();
        repo.save(&cheap).await.unwrap();

        let mid = PaywallArticle::new(
            test_article_id("mid"),
            test_tenant_id(),
            creator_id,
            "Mid Article".to_string(),
            "https://example.com/mid".to_string(),
            100, // $1.00
        )
        .unwrap();
        repo.save(&mid).await.unwrap();

        let expensive = PaywallArticle::new(
            test_article_id("expensive"),
            test_tenant_id(),
            creator_id,
            "Expensive Article".to_string(),
            "https://example.com/expensive".to_string(),
            500, // $5.00
        )
        .unwrap();
        repo.save(&expensive).await.unwrap();

        let query = ArticleQuery::new().with_price_range(50, 200);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title(), "Mid Article");
    }

    #[tokio::test]
    async fn test_query_order_by_revenue() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        // Create article with no revenue
        let low = create_test_article("low", "Low Revenue", creator_id);
        repo.save(&low).await.unwrap();

        // Create article with revenue
        let mut high = create_test_article("high", "High Revenue", creator_id);
        high.record_purchase(100);
        high.record_purchase(200);
        repo.save(&high).await.unwrap();

        let query = ArticleQuery::new().order_by_revenue();
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title(), "High Revenue");
    }

    #[tokio::test]
    async fn test_query_with_pagination() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        for i in 0..5 {
            let article = create_test_article(
                &format!("article-{}", i),
                &format!("Article {}", i),
                creator_id,
            );
            repo.save(&article).await.unwrap();
        }

        let query = ArticleQuery::new().with_pagination(2, 1);
        let results = repo.query(&query).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_find_top_by_revenue() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        // Create articles with different revenues
        for i in 0..5 {
            let mut article = create_test_article(
                &format!("article-{}", i),
                &format!("Article {}", i),
                creator_id,
            );
            // Give each article different revenue
            for _ in 0..i {
                article.record_purchase(100);
            }
            repo.save(&article).await.unwrap();
        }

        let top = repo.find_top_by_revenue(None, 3).await.unwrap();
        assert_eq!(top.len(), 3);
        // Should be in descending order of revenue
        assert!(top[0].stats().total_revenue_cents >= top[1].stats().total_revenue_cents);
        assert!(top[1].stats().total_revenue_cents >= top[2].stats().total_revenue_cents);
    }

    #[tokio::test]
    async fn test_find_recent() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        // Create published article
        let published = create_test_article("published", "Published", creator_id);
        repo.save(&published).await.unwrap();

        // Create draft (not published)
        let draft = PaywallArticle::new_draft(
            test_article_id("draft"),
            test_tenant_id(),
            creator_id,
            "Draft".to_string(),
            "https://example.com/draft".to_string(),
            50,
        )
        .unwrap();
        repo.save(&draft).await.unwrap();

        let recent = repo.find_recent(None, 10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].title(), "Published");
    }

    #[tokio::test]
    async fn test_count_methods() {
        let repo = InMemoryArticleRepository::new();
        let creator1 = test_creator_id();
        let creator2 = test_creator_id();

        // Create articles for creator1
        for i in 0..3 {
            let article = create_test_article(
                &format!("article1-{}", i),
                &format!("Article1 {}", i),
                creator1,
            );
            repo.save(&article).await.unwrap();
        }

        // Create article for creator2
        let article2 = create_test_article("article2", "Article2", creator2);
        repo.save(&article2).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 4);
        assert_eq!(repo.count_by_creator(&creator1).await.unwrap(), 3);
        assert_eq!(repo.count_by_creator(&creator2).await.unwrap(), 1);
        assert_eq!(
            repo.count_by_status(ArticleStatus::Active).await.unwrap(),
            4
        );
    }

    #[tokio::test]
    async fn test_clear() {
        let repo = InMemoryArticleRepository::new();
        let creator_id = test_creator_id();

        for i in 0..3 {
            let article = create_test_article(
                &format!("article-{}", i),
                &format!("Article {}", i),
                creator_id,
            );
            repo.save(&article).await.unwrap();
        }

        assert_eq!(repo.len(), 3);
        repo.clear();
        assert!(repo.is_empty());
    }
}
