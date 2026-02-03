use crate::application::dto::{
    ArticleDto, CreateArticleRequest, CreateArticleResponse, ListArticlesResponse,
    UpdateArticleRequest, UpdateArticleResponse,
};
use crate::domain::entities::PaywallArticle;
use crate::domain::repositories::ArticleRepository;
use crate::domain::value_objects::{ArticleId, CreatorId, TenantId};
use crate::error::Result;
use std::sync::Arc;

/// Use Case: Create Article
///
/// Creates a new paywalled article for a creator.
///
/// Responsibilities:
/// - Validate input (DTO validation)
/// - Check for duplicate URL
/// - Create domain PaywallArticle entity (with domain validation)
/// - Persist via repository
/// - Return response DTO
pub struct CreateArticleUseCase {
    repository: Arc<dyn ArticleRepository>,
}

impl CreateArticleUseCase {
    pub fn new(repository: Arc<dyn ArticleRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: CreateArticleRequest) -> Result<CreateArticleResponse> {
        // Check for duplicate URL
        if self.repository.url_exists(&request.url).await? {
            return Err(crate::error::AllSourceError::ValidationError(
                "Article with this URL already exists".to_string(),
            ));
        }

        // Parse IDs
        let article_id = ArticleId::new(request.article_id)?;
        let tenant_id = TenantId::new(request.tenant_id)?;
        let creator_id = CreatorId::parse(&request.creator_id)?;

        // Create domain article entity
        let mut article = if request.is_draft.unwrap_or(false) {
            PaywallArticle::new_draft(
                article_id,
                tenant_id,
                creator_id,
                request.title,
                request.url,
                request.price_cents,
            )?
        } else {
            PaywallArticle::new(
                article_id,
                tenant_id,
                creator_id,
                request.title,
                request.url,
                request.price_cents,
            )?
        };

        // Set optional fields
        if let Some(description) = request.description {
            article.update_description(Some(description));
        }

        if let Some(reading_time) = request.estimated_reading_time_minutes {
            article.update_reading_time(Some(reading_time));
        }

        if let Some(preview) = request.preview_content {
            article.update_preview(Some(preview))?;
        }

        // Persist via repository
        self.repository.save(&article).await?;

        Ok(CreateArticleResponse {
            article: ArticleDto::from(&article),
        })
    }
}

/// Use Case: Update Article
///
/// Updates an existing article's information.
pub struct UpdateArticleUseCase {
    repository: Arc<dyn ArticleRepository>,
}

impl UpdateArticleUseCase {
    pub fn new(repository: Arc<dyn ArticleRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        mut article: PaywallArticle,
        request: UpdateArticleRequest,
    ) -> Result<UpdateArticleResponse> {
        // Update title if provided
        if let Some(title) = request.title {
            article.update_title(title)?;
        }

        // Update URL if provided
        if let Some(url) = request.url {
            // Check for duplicate URL (if different from current)
            if article.url() != url && self.repository.url_exists(&url).await? {
                return Err(crate::error::AllSourceError::ValidationError(
                    "Another article with this URL already exists".to_string(),
                ));
            }
            article.update_url(url)?;
        }

        // Update price if provided
        if let Some(price_cents) = request.price_cents {
            article.update_price(price_cents)?;
        }

        // Update description if provided
        if let Some(description) = request.description {
            article.update_description(Some(description));
        }

        // Update reading time if provided
        if let Some(reading_time) = request.estimated_reading_time_minutes {
            article.update_reading_time(Some(reading_time));
        }

        // Update preview if provided
        if let Some(preview) = request.preview_content {
            article.update_preview(Some(preview))?;
        }

        // Persist changes
        self.repository.save(&article).await?;

        Ok(UpdateArticleResponse {
            article: ArticleDto::from(&article),
        })
    }
}

/// Use Case: Publish Article
///
/// Publishes a draft article, making it available for purchase.
pub struct PublishArticleUseCase;

impl PublishArticleUseCase {
    pub fn execute(mut article: PaywallArticle) -> Result<ArticleDto> {
        article.publish()?;
        Ok(ArticleDto::from(&article))
    }
}

/// Use Case: Archive Article
///
/// Archives an article, preventing new purchases while preserving existing access.
pub struct ArchiveArticleUseCase;

impl ArchiveArticleUseCase {
    pub fn execute(mut article: PaywallArticle) -> Result<ArticleDto> {
        article.archive();
        Ok(ArticleDto::from(&article))
    }
}

/// Use Case: Restore Article
///
/// Restores an archived article, making it available for purchase again.
pub struct RestoreArticleUseCase;

impl RestoreArticleUseCase {
    pub fn execute(mut article: PaywallArticle) -> Result<ArticleDto> {
        article.restore()?;
        Ok(ArticleDto::from(&article))
    }
}

/// Use Case: Delete Article
///
/// Marks an article as deleted (soft delete).
pub struct DeleteArticleUseCase;

impl DeleteArticleUseCase {
    pub fn execute(mut article: PaywallArticle) -> Result<ArticleDto> {
        article.delete();
        Ok(ArticleDto::from(&article))
    }
}

/// Use Case: Record Article Purchase
///
/// Records a purchase for analytics tracking.
pub struct RecordArticlePurchaseUseCase;

impl RecordArticlePurchaseUseCase {
    pub fn execute(mut article: PaywallArticle, amount_cents: u64) -> Result<ArticleDto> {
        article.record_purchase(amount_cents);
        Ok(ArticleDto::from(&article))
    }
}

/// Use Case: List Articles
///
/// Returns a list of articles.
pub struct ListArticlesUseCase;

impl ListArticlesUseCase {
    pub fn execute(articles: Vec<PaywallArticle>) -> ListArticlesResponse {
        let article_dtos: Vec<ArticleDto> = articles.iter().map(ArticleDto::from).collect();
        let count = article_dtos.len();

        ListArticlesResponse {
            articles: article_dtos,
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ArticleStatus;
    use crate::domain::repositories::ArticleQuery;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockArticleRepository {
        articles: Mutex<Vec<PaywallArticle>>,
    }

    impl MockArticleRepository {
        fn new() -> Self {
            Self {
                articles: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ArticleRepository for MockArticleRepository {
        async fn save(&self, article: &PaywallArticle) -> Result<()> {
            let mut articles = self.articles.lock().unwrap();
            // Update if exists, otherwise insert
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

        async fn find_by_url(&self, url: &str) -> Result<Option<PaywallArticle>> {
            let articles = self.articles.lock().unwrap();
            Ok(articles.iter().find(|a| a.url() == url).cloned())
        }

        async fn find_by_creator(
            &self,
            creator_id: &CreatorId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<PaywallArticle>> {
            let articles = self.articles.lock().unwrap();
            Ok(articles
                .iter()
                .filter(|a| a.creator_id() == creator_id)
                .cloned()
                .collect())
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
            _status: ArticleStatus,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<PaywallArticle>> {
            Ok(Vec::new())
        }

        async fn count(&self) -> Result<usize> {
            Ok(self.articles.lock().unwrap().len())
        }

        async fn count_by_creator(&self, _creator_id: &CreatorId) -> Result<usize> {
            Ok(0)
        }

        async fn count_by_status(&self, _status: ArticleStatus) -> Result<usize> {
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

    #[tokio::test]
    async fn test_create_article() {
        let repo = Arc::new(MockArticleRepository::new());
        let use_case = CreateArticleUseCase::new(repo.clone());

        let request = CreateArticleRequest {
            article_id: "test-article".to_string(),
            tenant_id: "test-tenant".to_string(),
            creator_id: uuid::Uuid::new_v4().to_string(),
            title: "My Test Article".to_string(),
            url: "https://blog.example.com/article".to_string(),
            price_cents: 50,
            description: Some("A great article".to_string()),
            estimated_reading_time_minutes: Some(5),
            preview_content: None,
            is_draft: None,
        };

        let response = use_case.execute(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.article.title, "My Test Article");
        assert_eq!(response.article.price_cents, 50);
        assert!(response.article.is_purchasable);
    }

    #[tokio::test]
    async fn test_create_draft_article() {
        let repo = Arc::new(MockArticleRepository::new());
        let use_case = CreateArticleUseCase::new(repo.clone());

        let request = CreateArticleRequest {
            article_id: "draft-article".to_string(),
            tenant_id: "test-tenant".to_string(),
            creator_id: uuid::Uuid::new_v4().to_string(),
            title: "Draft Article".to_string(),
            url: "https://blog.example.com/draft".to_string(),
            price_cents: 100,
            description: None,
            estimated_reading_time_minutes: None,
            preview_content: None,
            is_draft: Some(true),
        };

        let response = use_case.execute(request).await.unwrap();
        assert!(!response.article.is_purchasable);
        assert_eq!(
            response.article.status,
            crate::application::dto::ArticleStatusDto::Draft
        );
    }

    #[tokio::test]
    async fn test_create_article_duplicate_url() {
        let repo = Arc::new(MockArticleRepository::new());
        let use_case = CreateArticleUseCase::new(repo.clone());

        // First article
        let request1 = CreateArticleRequest {
            article_id: "article-1".to_string(),
            tenant_id: "test-tenant".to_string(),
            creator_id: uuid::Uuid::new_v4().to_string(),
            title: "Article 1".to_string(),
            url: "https://blog.example.com/article".to_string(),
            price_cents: 50,
            description: None,
            estimated_reading_time_minutes: None,
            preview_content: None,
            is_draft: None,
        };
        use_case.execute(request1).await.unwrap();

        // Second article with same URL
        let request2 = CreateArticleRequest {
            article_id: "article-2".to_string(),
            tenant_id: "test-tenant".to_string(),
            creator_id: uuid::Uuid::new_v4().to_string(),
            title: "Article 2".to_string(),
            url: "https://blog.example.com/article".to_string(),
            price_cents: 50,
            description: None,
            estimated_reading_time_minutes: None,
            preview_content: None,
            is_draft: None,
        };

        let result = use_case.execute(request2).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_publish_article() {
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let creator_id = CreatorId::new();

        let article = PaywallArticle::new_draft(
            article_id,
            tenant_id,
            creator_id,
            "Draft".to_string(),
            "https://example.com".to_string(),
            50,
        )
        .unwrap();

        assert!(!article.is_purchasable());

        let result = PublishArticleUseCase::execute(article);
        assert!(result.is_ok());
        assert!(result.unwrap().is_purchasable);
    }

    #[test]
    fn test_archive_and_restore() {
        let article_id = ArticleId::new("test-article".to_string()).unwrap();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let creator_id = CreatorId::new();

        let article = PaywallArticle::new(
            article_id,
            tenant_id,
            creator_id,
            "Article".to_string(),
            "https://example.com".to_string(),
            50,
        )
        .unwrap();

        // Archive
        let archived = ArchiveArticleUseCase::execute(article).unwrap();
        assert_eq!(
            archived.status,
            crate::application::dto::ArticleStatusDto::Archived
        );
        assert!(!archived.is_purchasable);

        // Restore (need to recreate since we don't have the entity)
        let article_id = ArticleId::new("test-article-2".to_string()).unwrap();
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let creator_id = CreatorId::new();

        let mut article = PaywallArticle::new(
            article_id,
            tenant_id,
            creator_id,
            "Article".to_string(),
            "https://example.com/2".to_string(),
            50,
        )
        .unwrap();
        article.archive();

        let restored = RestoreArticleUseCase::execute(article).unwrap();
        assert_eq!(
            restored.status,
            crate::application::dto::ArticleStatusDto::Active
        );
        assert!(restored.is_purchasable);
    }

    #[test]
    fn test_list_articles() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let creator_id = CreatorId::new();

        let articles = vec![
            PaywallArticle::new(
                ArticleId::new("article-1".to_string()).unwrap(),
                tenant_id.clone(),
                creator_id,
                "Article 1".to_string(),
                "https://example.com/1".to_string(),
                50,
            )
            .unwrap(),
            PaywallArticle::new(
                ArticleId::new("article-2".to_string()).unwrap(),
                tenant_id,
                creator_id,
                "Article 2".to_string(),
                "https://example.com/2".to_string(),
                100,
            )
            .unwrap(),
        ];

        let response = ListArticlesUseCase::execute(articles);
        assert_eq!(response.count, 2);
        assert_eq!(response.articles.len(), 2);
    }
}
