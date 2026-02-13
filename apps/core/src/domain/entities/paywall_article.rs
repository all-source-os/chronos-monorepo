use crate::{
    domain::value_objects::{ArticleId, CreatorId, Money, TenantId},
    error::Result,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Article status in the paywall system
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArticleStatus {
    /// Article is a draft, not yet published
    Draft,
    /// Article is active and available for purchase
    #[default]
    Active,
    /// Article has been archived (no new purchases)
    Archived,
    /// Article has been deleted
    Deleted,
}

/// Article statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArticleStats {
    /// Total number of purchases
    pub total_purchases: u64,
    /// Total revenue in cents
    pub total_revenue_cents: u64,
    /// Total number of unique readers
    pub unique_readers: u64,
    /// Average read duration in seconds
    pub avg_read_duration_seconds: u64,
    /// Average scroll depth percentage
    pub avg_scroll_depth: u8,
    /// Conversion rate (purchases / views * 100)
    pub conversion_rate: f32,
}

impl ArticleStats {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Domain Entity: PaywallArticle
///
/// Represents a paywalled article in the system.
/// Articles are created by creators and can be purchased by readers.
///
/// Domain Rules:
/// - Article must have a valid ID (slug)
/// - Price must be within allowed range ($0.10 - $10.00)
/// - Article belongs to exactly one creator
/// - Only active articles can be purchased
/// - Article URL must be valid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaywallArticle {
    id: ArticleId,
    tenant_id: TenantId,
    creator_id: CreatorId,
    title: String,
    url: String,
    price: Money,
    description: Option<String>,
    estimated_reading_time_minutes: Option<u16>,
    preview_content: Option<String>,
    status: ArticleStatus,
    stats: ArticleStats,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    published_at: Option<DateTime<Utc>>,
    metadata: serde_json::Value,
}

/// Minimum price: $0.10 (10 cents)
const MIN_PRICE_CENTS: u64 = 10;
/// Maximum price: $10.00 (1000 cents)
const MAX_PRICE_CENTS: u64 = 1000;

impl PaywallArticle {
    /// Create a new paywall article with validation
    pub fn new(
        id: ArticleId,
        tenant_id: TenantId,
        creator_id: CreatorId,
        title: String,
        url: String,
        price_cents: u64,
    ) -> Result<Self> {
        Self::validate_title(&title)?;
        Self::validate_url(&url)?;
        Self::validate_price(price_cents)?;

        let now = Utc::now();
        Ok(Self {
            id,
            tenant_id,
            creator_id,
            title,
            url,
            price: Money::usd_cents(price_cents),
            description: None,
            estimated_reading_time_minutes: None,
            preview_content: None,
            status: ArticleStatus::Active,
            stats: ArticleStats::new(),
            created_at: now,
            updated_at: now,
            published_at: Some(now),
            metadata: serde_json::json!({}),
        })
    }

    /// Create a draft article (not yet published)
    pub fn new_draft(
        id: ArticleId,
        tenant_id: TenantId,
        creator_id: CreatorId,
        title: String,
        url: String,
        price_cents: u64,
    ) -> Result<Self> {
        Self::validate_title(&title)?;
        Self::validate_url(&url)?;
        Self::validate_price(price_cents)?;

        let now = Utc::now();
        Ok(Self {
            id,
            tenant_id,
            creator_id,
            title,
            url,
            price: Money::usd_cents(price_cents),
            description: None,
            estimated_reading_time_minutes: None,
            preview_content: None,
            status: ArticleStatus::Draft,
            stats: ArticleStats::new(),
            created_at: now,
            updated_at: now,
            published_at: None,
            metadata: serde_json::json!({}),
        })
    }

    /// Reconstruct article from storage (bypasses validation)
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: ArticleId,
        tenant_id: TenantId,
        creator_id: CreatorId,
        title: String,
        url: String,
        price: Money,
        description: Option<String>,
        estimated_reading_time_minutes: Option<u16>,
        preview_content: Option<String>,
        status: ArticleStatus,
        stats: ArticleStats,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        published_at: Option<DateTime<Utc>>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            tenant_id,
            creator_id,
            title,
            url,
            price,
            description,
            estimated_reading_time_minutes,
            preview_content,
            status,
            stats,
            created_at,
            updated_at,
            published_at,
            metadata,
        }
    }

    // Getters

    pub fn id(&self) -> &ArticleId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn creator_id(&self) -> &CreatorId {
        &self.creator_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn price(&self) -> &Money {
        &self.price
    }

    pub fn price_cents(&self) -> u64 {
        self.price.amount()
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn estimated_reading_time_minutes(&self) -> Option<u16> {
        self.estimated_reading_time_minutes
    }

    pub fn preview_content(&self) -> Option<&str> {
        self.preview_content.as_deref()
    }

    pub fn status(&self) -> ArticleStatus {
        self.status
    }

    pub fn stats(&self) -> &ArticleStats {
        &self.stats
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn published_at(&self) -> Option<DateTime<Utc>> {
        self.published_at
    }

    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    // Domain behavior methods

    /// Check if article is active and can be purchased
    pub fn is_purchasable(&self) -> bool {
        self.status == ArticleStatus::Active
    }

    /// Check if this article belongs to a specific creator
    pub fn belongs_to(&self, creator_id: &CreatorId) -> bool {
        &self.creator_id == creator_id
    }

    /// Update the title
    pub fn update_title(&mut self, title: String) -> Result<()> {
        Self::validate_title(&title)?;
        self.title = title;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update the URL
    pub fn update_url(&mut self, url: String) -> Result<()> {
        Self::validate_url(&url)?;
        self.url = url;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update the price
    pub fn update_price(&mut self, price_cents: u64) -> Result<()> {
        Self::validate_price(price_cents)?;
        self.price = Money::usd_cents(price_cents);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update the description
    pub fn update_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    /// Update the estimated reading time
    pub fn update_reading_time(&mut self, minutes: Option<u16>) {
        self.estimated_reading_time_minutes = minutes;
        self.updated_at = Utc::now();
    }

    /// Update the preview content
    pub fn update_preview(&mut self, preview: Option<String>) -> Result<()> {
        if let Some(ref p) = preview
            && p.len() > 1000
        {
            return Err(crate::error::AllSourceError::ValidationError(
                "Preview content cannot exceed 1000 characters".to_string(),
            ));
        }
        self.preview_content = preview;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Publish a draft article
    pub fn publish(&mut self) -> Result<()> {
        if self.status != ArticleStatus::Draft {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Can only publish draft articles, current status: {:?}",
                self.status
            )));
        }

        self.status = ArticleStatus::Active;
        self.published_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Archive the article (no new purchases allowed)
    pub fn archive(&mut self) {
        self.status = ArticleStatus::Archived;
        self.updated_at = Utc::now();
    }

    /// Restore an archived article
    pub fn restore(&mut self) -> Result<()> {
        if self.status != ArticleStatus::Archived {
            return Err(crate::error::AllSourceError::ValidationError(
                "Can only restore archived articles".to_string(),
            ));
        }

        self.status = ArticleStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark the article as deleted
    pub fn delete(&mut self) {
        self.status = ArticleStatus::Deleted;
        self.updated_at = Utc::now();
    }

    /// Record a purchase for this article
    pub fn record_purchase(&mut self, amount_cents: u64) {
        self.stats.total_purchases += 1;
        self.stats.total_revenue_cents += amount_cents;
        self.updated_at = Utc::now();
    }

    /// Record a unique reader
    pub fn record_reader(&mut self) {
        self.stats.unique_readers += 1;
        self.updated_at = Utc::now();
    }

    /// Update reading analytics
    pub fn update_reading_analytics(&mut self, duration_seconds: u64, scroll_depth: u8) {
        // Simple moving average approximation
        let total = self.stats.total_purchases.max(1);
        self.stats.avg_read_duration_seconds =
            ((self.stats.avg_read_duration_seconds * (total - 1)) + duration_seconds) / total;
        self.stats.avg_scroll_depth = (((self.stats.avg_scroll_depth as u64 * (total - 1))
            + scroll_depth as u64)
            / total) as u8;
        self.updated_at = Utc::now();
    }

    /// Update conversion rate
    pub fn update_conversion_rate(&mut self, views: u64) {
        if views > 0 {
            self.stats.conversion_rate = (self.stats.total_purchases as f32 / views as f32) * 100.0;
        }
        self.updated_at = Utc::now();
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }

    // Validation

    fn validate_title(title: &str) -> Result<()> {
        if title.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Article title cannot be empty".to_string(),
            ));
        }

        if title.len() > 500 {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Article title cannot exceed 500 characters".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_url(url: &str) -> Result<()> {
        if url.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Article URL cannot be empty".to_string(),
            ));
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Article URL must start with http:// or https://".to_string(),
            ));
        }

        if url.len() > 2048 {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Article URL cannot exceed 2048 characters".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_price(price_cents: u64) -> Result<()> {
        if price_cents < MIN_PRICE_CENTS {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Price must be at least ${:.2}",
                MIN_PRICE_CENTS as f64 / 100.0
            )));
        }

        if price_cents > MAX_PRICE_CENTS {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Price cannot exceed ${:.2}",
                MAX_PRICE_CENTS as f64 / 100.0
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn test_article_id() -> ArticleId {
        ArticleId::new("test-article".to_string()).unwrap()
    }

    fn test_creator_id() -> CreatorId {
        CreatorId::new()
    }

    #[test]
    fn test_create_article() {
        let article = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "My Awesome Article".to_string(),
            "https://blog.example.com/article".to_string(),
            50, // $0.50
        );

        assert!(article.is_ok());
        let article = article.unwrap();
        assert_eq!(article.title(), "My Awesome Article");
        assert_eq!(article.price_cents(), 50);
        assert_eq!(article.status(), ArticleStatus::Active);
        assert!(article.is_purchasable());
    }

    #[test]
    fn test_create_draft_article() {
        let article = PaywallArticle::new_draft(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Draft Article".to_string(),
            "https://blog.example.com/draft".to_string(),
            100,
        );

        assert!(article.is_ok());
        let article = article.unwrap();
        assert_eq!(article.status(), ArticleStatus::Draft);
        assert!(!article.is_purchasable());
        assert!(article.published_at().is_none());
    }

    #[test]
    fn test_reject_empty_title() {
        let result = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "".to_string(),
            "https://blog.example.com/article".to_string(),
            50,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_invalid_url() {
        let result = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Title".to_string(),
            "not-a-url".to_string(),
            50,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_price_too_low() {
        let result = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Title".to_string(),
            "https://example.com".to_string(),
            5, // $0.05 - below minimum
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_price_too_high() {
        let result = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Title".to_string(),
            "https://example.com".to_string(),
            1500, // $15.00 - above maximum
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_publish_draft() {
        let mut article = PaywallArticle::new_draft(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Draft Article".to_string(),
            "https://example.com".to_string(),
            50,
        )
        .unwrap();

        assert!(!article.is_purchasable());

        let result = article.publish();
        assert!(result.is_ok());
        assert!(article.is_purchasable());
        assert!(article.published_at().is_some());
    }

    #[test]
    fn test_cannot_publish_active_article() {
        let mut article = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Active Article".to_string(),
            "https://example.com".to_string(),
            50,
        )
        .unwrap();

        let result = article.publish();
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_and_restore() {
        let mut article = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Article".to_string(),
            "https://example.com".to_string(),
            50,
        )
        .unwrap();

        assert!(article.is_purchasable());

        article.archive();
        assert_eq!(article.status(), ArticleStatus::Archived);
        assert!(!article.is_purchasable());

        article.restore().unwrap();
        assert_eq!(article.status(), ArticleStatus::Active);
        assert!(article.is_purchasable());
    }

    #[test]
    fn test_record_purchase() {
        let mut article = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Article".to_string(),
            "https://example.com".to_string(),
            50,
        )
        .unwrap();

        assert_eq!(article.stats().total_purchases, 0);
        assert_eq!(article.stats().total_revenue_cents, 0);

        article.record_purchase(50);
        assert_eq!(article.stats().total_purchases, 1);
        assert_eq!(article.stats().total_revenue_cents, 50);

        article.record_purchase(50);
        assert_eq!(article.stats().total_purchases, 2);
        assert_eq!(article.stats().total_revenue_cents, 100);
    }

    #[test]
    fn test_update_price() {
        let mut article = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Article".to_string(),
            "https://example.com".to_string(),
            50,
        )
        .unwrap();

        assert_eq!(article.price_cents(), 50);

        article.update_price(100).unwrap();
        assert_eq!(article.price_cents(), 100);

        // Should fail for invalid price
        assert!(article.update_price(5).is_err());
    }

    #[test]
    fn test_belongs_to() {
        let creator_id = test_creator_id();
        let other_creator_id = CreatorId::new();

        let article = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            creator_id,
            "Article".to_string(),
            "https://example.com".to_string(),
            50,
        )
        .unwrap();

        assert!(article.belongs_to(&creator_id));
        assert!(!article.belongs_to(&other_creator_id));
    }

    #[test]
    fn test_serde_serialization() {
        let article = PaywallArticle::new(
            test_article_id(),
            test_tenant_id(),
            test_creator_id(),
            "Test Article".to_string(),
            "https://example.com".to_string(),
            50,
        )
        .unwrap();

        let json = serde_json::to_string(&article);
        assert!(json.is_ok());

        let deserialized: PaywallArticle = serde_json::from_str(&json.unwrap()).unwrap();
        assert_eq!(deserialized.title(), "Test Article");
    }
}
