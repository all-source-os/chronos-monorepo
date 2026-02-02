use crate::domain::entities::{ArticleStats, ArticleStatus, PaywallArticle};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// DTO for creating a new article
#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    pub article_id: String,
    pub tenant_id: String,
    pub creator_id: String,
    pub title: String,
    pub url: String,
    pub price_cents: u64,
    pub description: Option<String>,
    pub estimated_reading_time_minutes: Option<u16>,
    pub preview_content: Option<String>,
    /// If true, create as draft (not immediately published)
    pub is_draft: Option<bool>,
}

/// DTO for article creation response
#[derive(Debug, Serialize)]
pub struct CreateArticleResponse {
    pub article: ArticleDto,
}

/// DTO for updating an article
#[derive(Debug, Deserialize)]
pub struct UpdateArticleRequest {
    pub title: Option<String>,
    pub url: Option<String>,
    pub price_cents: Option<u64>,
    pub description: Option<String>,
    pub estimated_reading_time_minutes: Option<u16>,
    pub preview_content: Option<String>,
}

/// DTO for article update response
#[derive(Debug, Serialize)]
pub struct UpdateArticleResponse {
    pub article: ArticleDto,
}

/// DTO for listing articles response
#[derive(Debug, Serialize)]
pub struct ListArticlesResponse {
    pub articles: Vec<ArticleDto>,
    pub count: usize,
}

/// DTO for article status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArticleStatusDto {
    Draft,
    Active,
    Archived,
    Deleted,
}

impl From<ArticleStatus> for ArticleStatusDto {
    fn from(status: ArticleStatus) -> Self {
        match status {
            ArticleStatus::Draft => ArticleStatusDto::Draft,
            ArticleStatus::Active => ArticleStatusDto::Active,
            ArticleStatus::Archived => ArticleStatusDto::Archived,
            ArticleStatus::Deleted => ArticleStatusDto::Deleted,
        }
    }
}

/// DTO for article statistics
#[derive(Debug, Clone, Serialize)]
pub struct ArticleStatsDto {
    pub total_purchases: u64,
    pub total_revenue_cents: u64,
    pub unique_readers: u64,
    pub avg_read_duration_seconds: u64,
    pub avg_scroll_depth: u8,
    pub conversion_rate: f32,
}

impl From<&ArticleStats> for ArticleStatsDto {
    fn from(stats: &ArticleStats) -> Self {
        Self {
            total_purchases: stats.total_purchases,
            total_revenue_cents: stats.total_revenue_cents,
            unique_readers: stats.unique_readers,
            avg_read_duration_seconds: stats.avg_read_duration_seconds,
            avg_scroll_depth: stats.avg_scroll_depth,
            conversion_rate: stats.conversion_rate,
        }
    }
}

/// DTO for a paywall article in responses
#[derive(Debug, Clone, Serialize)]
pub struct ArticleDto {
    pub id: String,
    pub tenant_id: String,
    pub creator_id: String,
    pub title: String,
    pub url: String,
    pub price_cents: u64,
    pub description: Option<String>,
    pub estimated_reading_time_minutes: Option<u16>,
    pub preview_content: Option<String>,
    pub status: ArticleStatusDto,
    pub stats: ArticleStatsDto,
    pub is_purchasable: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

impl From<&PaywallArticle> for ArticleDto {
    fn from(article: &PaywallArticle) -> Self {
        Self {
            id: article.id().to_string(),
            tenant_id: article.tenant_id().to_string(),
            creator_id: article.creator_id().as_uuid().to_string(),
            title: article.title().to_string(),
            url: article.url().to_string(),
            price_cents: article.price_cents(),
            description: article.description().map(|s| s.to_string()),
            estimated_reading_time_minutes: article.estimated_reading_time_minutes(),
            preview_content: article.preview_content().map(|s| s.to_string()),
            status: article.status().into(),
            stats: article.stats().into(),
            is_purchasable: article.is_purchasable(),
            created_at: article.created_at(),
            updated_at: article.updated_at(),
            published_at: article.published_at(),
        }
    }
}

impl From<PaywallArticle> for ArticleDto {
    fn from(article: PaywallArticle) -> Self {
        ArticleDto::from(&article)
    }
}
