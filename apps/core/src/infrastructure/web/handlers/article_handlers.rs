// Article HTTP handlers - delegate to article use cases
// Clean Architecture: Infrastructure Layer (HTTP) -> Application Layer (Use Cases)

use crate::{
    application::{
        dto::{
            ArticleDto, CreateArticleRequest, CreateArticleResponse, ListArticlesResponse,
            UpdateArticleRequest, UpdateArticleResponse,
        },
        use_cases::{
            ArchiveArticleUseCase, CreateArticleUseCase, DeleteArticleUseCase, ListArticlesUseCase,
            PublishArticleUseCase, RestoreArticleUseCase, UpdateArticleUseCase,
        },
    },
    domain::{
        entities::ArticleStatus,
        repositories::{ArticleRepository, CreatorRepository},
        value_objects::{ArticleId, CreatorId, TenantId},
    },
    error::Result,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use std::sync::Arc;

/// Application state for article handlers
#[derive(Clone)]
pub struct ArticleHandlerState<A: ArticleRepository, C: CreatorRepository> {
    pub article_repo: Arc<A>,
    pub creator_repo: Arc<C>,
}

/// Query parameters for listing articles
#[derive(Debug, Deserialize)]
pub struct ListArticlesParams {
    pub tenant_id: Option<String>,
    pub creator_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Create a new article
///
/// POST /api/v1/articles
pub async fn create_article_handler<A, C>(
    State(state): State<ArticleHandlerState<A, C>>,
    Json(request): Json<CreateArticleRequest>,
) -> Result<Json<CreateArticleResponse>>
where
    A: ArticleRepository + 'static,
    C: CreatorRepository + 'static,
{
    tracing::info!(
        article_id = %request.article_id,
        creator_id = %request.creator_id,
        title = %request.title,
        "Creating new article"
    );

    let use_case = CreateArticleUseCase::new(state.article_repo.clone());
    let response = use_case.execute(request).await?;

    tracing::info!(
        article_id = %response.article.id,
        "Article created successfully"
    );

    Ok(Json(response))
}

/// List articles with optional filtering
///
/// GET /api/v1/articles
pub async fn list_articles_handler<A, C>(
    State(state): State<ArticleHandlerState<A, C>>,
    Query(params): Query<ListArticlesParams>,
) -> Result<Json<ListArticlesResponse>>
where
    A: ArticleRepository + 'static,
    C: CreatorRepository + 'static,
{
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let articles = if let Some(creator_id_str) = params.creator_id {
        let creator_id = CreatorId::parse(&creator_id_str)?;
        state
            .article_repo
            .find_by_creator(&creator_id, limit, offset)
            .await?
    } else if let Some(tenant_id_str) = params.tenant_id {
        let tenant_id = TenantId::new(tenant_id_str)?;
        state
            .article_repo
            .find_by_tenant(&tenant_id, limit, offset)
            .await?
    } else if let Some(status_str) = params.status {
        let status = parse_article_status(&status_str)?;
        state
            .article_repo
            .find_by_status(status, limit, offset)
            .await?
    } else {
        // Default: return active articles
        state
            .article_repo
            .find_by_status(ArticleStatus::Active, limit, offset)
            .await?
    };

    let response = ListArticlesUseCase::execute(&articles);

    tracing::debug!(count = response.count, "Listed articles");

    Ok(Json(response))
}

fn parse_article_status(s: &str) -> Result<ArticleStatus> {
    match s.to_lowercase().as_str() {
        "draft" => Ok(ArticleStatus::Draft),
        "active" => Ok(ArticleStatus::Active),
        "archived" => Ok(ArticleStatus::Archived),
        "deleted" => Ok(ArticleStatus::Deleted),
        _ => Err(crate::error::AllSourceError::InvalidInput(format!(
            "Invalid article status: {s}. Valid values: draft, active, archived, deleted"
        ))),
    }
}

/// Get a specific article by ID
///
/// GET /api/v1/articles/{article_id}
pub async fn get_article_handler<A, C>(
    State(state): State<ArticleHandlerState<A, C>>,
    Path(article_id): Path<String>,
) -> Result<Json<ArticleDto>>
where
    A: ArticleRepository + 'static,
    C: CreatorRepository + 'static,
{
    let id = ArticleId::new(article_id.clone())?;

    let article = state.article_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Article not found: {article_id}"))
    })?;

    Ok(Json(ArticleDto::from(&article)))
}

/// Update an article
///
/// PUT /api/v1/articles/{article_id}
pub async fn update_article_handler<A, C>(
    State(state): State<ArticleHandlerState<A, C>>,
    Path(article_id): Path<String>,
    Json(request): Json<UpdateArticleRequest>,
) -> Result<Json<UpdateArticleResponse>>
where
    A: ArticleRepository + 'static,
    C: CreatorRepository + 'static,
{
    let id = ArticleId::new(article_id.clone())?;

    let article = state.article_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Article not found: {article_id}"))
    })?;

    let use_case = UpdateArticleUseCase::new(state.article_repo.clone());
    let response = use_case.execute(article, request).await?;

    tracing::info!(
        article_id = %article_id,
        "Article updated successfully"
    );

    Ok(Json(response))
}

/// Publish an article (change status from Draft to Active)
///
/// POST /api/v1/articles/{article_id}/publish
pub async fn publish_article_handler<A, C>(
    State(state): State<ArticleHandlerState<A, C>>,
    Path(article_id): Path<String>,
) -> Result<Json<UpdateArticleResponse>>
where
    A: ArticleRepository + 'static,
    C: CreatorRepository + 'static,
{
    let id = ArticleId::new(article_id.clone())?;

    let article = state.article_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Article not found: {article_id}"))
    })?;

    let dto = PublishArticleUseCase::execute(article)?;

    tracing::info!(
        article_id = %article_id,
        "Article published"
    );

    Ok(Json(UpdateArticleResponse { article: dto }))
}

/// Archive an article
///
/// POST /api/v1/articles/{article_id}/archive
pub async fn archive_article_handler<A, C>(
    State(state): State<ArticleHandlerState<A, C>>,
    Path(article_id): Path<String>,
) -> Result<Json<UpdateArticleResponse>>
where
    A: ArticleRepository + 'static,
    C: CreatorRepository + 'static,
{
    let id = ArticleId::new(article_id.clone())?;

    let article = state.article_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Article not found: {article_id}"))
    })?;

    let dto = ArchiveArticleUseCase::execute(article)?;

    tracing::info!(
        article_id = %article_id,
        "Article archived"
    );

    Ok(Json(UpdateArticleResponse { article: dto }))
}

/// Restore an archived article
///
/// POST /api/v1/articles/{article_id}/restore
pub async fn restore_article_handler<A, C>(
    State(state): State<ArticleHandlerState<A, C>>,
    Path(article_id): Path<String>,
) -> Result<Json<UpdateArticleResponse>>
where
    A: ArticleRepository + 'static,
    C: CreatorRepository + 'static,
{
    let id = ArticleId::new(article_id.clone())?;

    let article = state.article_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Article not found: {article_id}"))
    })?;

    let dto = RestoreArticleUseCase::execute(article)?;

    tracing::info!(
        article_id = %article_id,
        "Article restored"
    );

    Ok(Json(UpdateArticleResponse { article: dto }))
}

/// Delete an article (soft delete - marks as Deleted status)
///
/// DELETE /api/v1/articles/{article_id}
pub async fn delete_article_handler<A, C>(
    State(state): State<ArticleHandlerState<A, C>>,
    Path(article_id): Path<String>,
) -> Result<Json<serde_json::Value>>
where
    A: ArticleRepository + 'static,
    C: CreatorRepository + 'static,
{
    let id = ArticleId::new(article_id.clone())?;

    let article = state.article_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Article not found: {article_id}"))
    })?;

    let _dto = DeleteArticleUseCase::execute(article)?;

    tracing::info!(
        article_id = %article_id,
        "Article deleted"
    );

    Ok(Json(serde_json::json!({
        "deleted": true,
        "article_id": article_id
    })))
}

/// Get articles by creator
///
/// GET /api/v1/creators/{creator_id}/articles
pub async fn get_creator_articles_handler<A, C>(
    State(state): State<ArticleHandlerState<A, C>>,
    Path(creator_id): Path<String>,
    Query(params): Query<ListArticlesParams>,
) -> Result<Json<ListArticlesResponse>>
where
    A: ArticleRepository + 'static,
    C: CreatorRepository + 'static,
{
    let creator = CreatorId::parse(&creator_id)?;
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let articles = state
        .article_repo
        .find_by_creator(&creator, limit, offset)
        .await?;

    let response = ListArticlesUseCase::execute(&articles);

    tracing::debug!(
        creator_id = %creator_id,
        count = response.count,
        "Listed creator articles"
    );

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_articles_params_default() {
        let json = r"{}";
        let params: ListArticlesParams = serde_json::from_str(json).unwrap();
        assert!(params.tenant_id.is_none());
        assert!(params.creator_id.is_none());
        assert!(params.status.is_none());
    }

    #[test]
    fn test_list_articles_params_with_values() {
        let json =
            r#"{"tenant_id": "test", "creator_id": "creator-1", "status": "active", "limit": 10}"#;
        let params: ListArticlesParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.tenant_id, Some("test".to_string()));
        assert_eq!(params.creator_id, Some("creator-1".to_string()));
        assert_eq!(params.status, Some("active".to_string()));
        assert_eq!(params.limit, Some(10));
    }

    #[test]
    fn test_parse_article_status() {
        assert!(matches!(
            parse_article_status("draft"),
            Ok(ArticleStatus::Draft)
        ));
        assert!(matches!(
            parse_article_status("active"),
            Ok(ArticleStatus::Active)
        ));
        assert!(matches!(
            parse_article_status("archived"),
            Ok(ArticleStatus::Archived)
        ));
        assert!(matches!(
            parse_article_status("deleted"),
            Ok(ArticleStatus::Deleted)
        ));
        assert!(matches!(
            parse_article_status("ACTIVE"),
            Ok(ArticleStatus::Active)
        ));
        assert!(parse_article_status("invalid").is_err());
    }
}
