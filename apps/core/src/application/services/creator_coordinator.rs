use crate::application::dto::{CreatorDto, RegisterCreatorRequest, RegisterCreatorResponse};
use crate::application::use_cases::RegisterCreatorUseCase;
use crate::domain::repositories::CreatorRepository;
use crate::error::Result;
use std::sync::Arc;

/// Coordinator service for creator operations.
///
/// This service orchestrates use cases to handle creator workflows
/// including registration.
pub struct CreatorCoordinator {
    register_creator: RegisterCreatorUseCase,
}

impl CreatorCoordinator {
    pub fn new(creator_repo: Arc<dyn CreatorRepository>) -> Self {
        Self {
            register_creator: RegisterCreatorUseCase::new(creator_repo),
        }
    }

    /// Register a new creator
    pub async fn register_creator(
        &self,
        request: RegisterCreatorRequest,
    ) -> Result<RegisterCreatorResponse> {
        self.register_creator.execute(request).await
    }
}

/// Creator dashboard data
#[derive(Debug)]
pub struct CreatorDashboard {
    pub creator: CreatorDto,
    pub total_revenue_cents: i64,
    pub total_articles: u64,
}

/// Article performance metrics
#[derive(Debug)]
pub struct ArticlePerformance {
    pub article_id: String,
    pub title: String,
    pub total_purchases: u64,
    pub total_revenue_cents: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creator_dashboard_struct() {
        let dashboard = CreatorDashboard {
            creator: CreatorDto {
                id: uuid::Uuid::new_v4(),
                tenant_id: "default".to_string(),
                email: "test@test.com".to_string(),
                name: Some("Test".to_string()),
                wallet_address: "0x123".to_string(),
                blog_url: None,
                status: crate::application::dto::CreatorStatusDto::Active,
                tier: crate::application::dto::CreatorTierDto::Free,
                settings: crate::application::dto::CreatorSettingsDto {
                    default_price_cents: Some(100),
                    show_reading_time: Some(true),
                    brand_color: None,
                    unlock_button_text: None,
                },
                email_verified: false,
                total_revenue_cents: 0,
                total_articles: 0,
                fee_percentage: 10,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            total_revenue_cents: 0,
            total_articles: 0,
        };
        assert_eq!(dashboard.total_articles, 0);
    }

    #[test]
    fn test_article_performance_struct() {
        let perf = ArticlePerformance {
            article_id: "article-1".to_string(),
            title: "Test Article".to_string(),
            total_purchases: 10,
            total_revenue_cents: 1000,
        };
        assert_eq!(perf.total_purchases, 10);
    }
}
