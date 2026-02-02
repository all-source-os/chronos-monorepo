// Fork HTTP handlers - delegate to fork use cases
// Clean Architecture: Infrastructure Layer (HTTP) -> Application Layer (Use Cases)

use crate::application::dto::{
    AppendForkEventRequest, AppendForkEventResponse, BranchForkRequest, BranchForkResponse,
    CleanupExpiredForksRequest, CleanupExpiredForksResponse, CreateForkRequest, CreateForkResponse,
    DiscardForkRequest, DiscardForkResponse, ForkDto, ListForksRequest, ListForksResponse,
    MergeForkRequest, MergeForkResponse, QueryForkEventsRequest, QueryForkEventsResponse,
    UpdateForkRequest, UpdateForkResponse,
};
use crate::application::use_cases::{
    AppendForkEventUseCase, BranchForkUseCase, CleanupExpiredForksUseCase, CreateForkUseCase,
    DiscardForkUseCase, GetForkUseCase, ListForksUseCase, MergeForkUseCase, QueryForkEventsUseCase,
    UpdateForkUseCase,
};
use crate::domain::repositories::ForkRepository;
use crate::domain::value_objects::ForkId;
use crate::error::Result;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

/// Application state for fork handlers
#[derive(Clone)]
pub struct ForkHandlerState<F: ForkRepository> {
    pub fork_repo: Arc<F>,
}

/// Query parameters for listing forks
#[derive(Debug, Deserialize)]
pub struct ListForksParams {
    pub tenant_id: Option<String>,
    pub status: Option<String>,
    pub created_by_agent: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Create a new fork
///
/// POST /api/v1/forks
pub async fn create_fork_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Json(request): Json<CreateForkRequest>,
) -> Result<Json<CreateForkResponse>>
where
    F: ForkRepository + 'static,
{
    tracing::info!(
        name = %request.name,
        tenant_id = %request.tenant_id,
        "Creating new fork"
    );

    let use_case = CreateForkUseCase::new(state.fork_repo.clone());
    let response = use_case.execute(request).await?;

    tracing::info!(
        fork_id = %response.fork.id,
        "Fork created successfully"
    );

    Ok(Json(response))
}

/// List forks with optional filtering
///
/// GET /api/v1/forks
pub async fn list_forks_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Query(params): Query<ListForksParams>,
) -> Result<Json<ListForksResponse>>
where
    F: ForkRepository + 'static,
{
    let use_case = ListForksUseCase::new(state.fork_repo.clone());

    let request = ListForksRequest {
        tenant_id: params.tenant_id,
        status: params.status.and_then(|s| serde_json::from_str(&format!("\"{s}\"")).ok()),
        created_by_agent: params.created_by_agent,
        limit: params.limit,
        offset: params.offset,
    };

    let response = use_case.execute(request).await?;

    tracing::debug!(count = response.count, "Listed forks");

    Ok(Json(response))
}

/// Get a specific fork by ID
///
/// GET /api/v1/forks/{fork_id}
pub async fn get_fork_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Path(fork_id): Path<String>,
) -> Result<Json<ForkDto>>
where
    F: ForkRepository + 'static,
{
    let use_case = GetForkUseCase::new(state.fork_repo.clone());

    let fork = use_case.execute(&fork_id).await?;

    Ok(Json(fork))
}

/// Update a fork
///
/// PUT /api/v1/forks/{fork_id}
pub async fn update_fork_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Path(fork_id): Path<String>,
    Json(request): Json<UpdateForkRequest>,
) -> Result<Json<UpdateForkResponse>>
where
    F: ForkRepository + 'static,
{
    let use_case = UpdateForkUseCase::new(state.fork_repo.clone());

    let response = use_case.execute(&fork_id, request).await?;

    tracing::info!(
        fork_id = %fork_id,
        "Fork updated successfully"
    );

    Ok(Json(response))
}

/// Branch a fork (create a child fork from an existing fork)
///
/// POST /api/v1/forks/{fork_id}/branch
pub async fn branch_fork_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Path(fork_id): Path<String>,
    Json(mut request): Json<BranchForkRequest>,
) -> Result<Json<BranchForkResponse>>
where
    F: ForkRepository + 'static,
{
    // Ensure the parent fork ID matches the path parameter
    request.parent_fork_id = fork_id.clone();

    let use_case = BranchForkUseCase::new(state.fork_repo.clone());
    let response = use_case.execute(request).await?;

    tracing::info!(
        parent_fork_id = %fork_id,
        new_fork_id = %response.fork.id,
        "Fork branched successfully"
    );

    Ok(Json(response))
}

/// Append an event to a fork
///
/// POST /api/v1/forks/{fork_id}/events
pub async fn append_fork_event_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Path(fork_id): Path<String>,
    Json(mut request): Json<AppendForkEventRequest>,
) -> Result<Json<AppendForkEventResponse>>
where
    F: ForkRepository + 'static,
{
    // Ensure the fork ID matches the path parameter
    request.fork_id = fork_id.clone();

    let use_case = AppendForkEventUseCase::new(state.fork_repo.clone());
    let response = use_case.execute(request).await?;

    tracing::info!(
        fork_id = %fork_id,
        event_id = %response.event_id,
        "Event appended to fork"
    );

    Ok(Json(response))
}

/// Query events in a fork
///
/// GET /api/v1/forks/{fork_id}/events
#[derive(Debug, Deserialize)]
pub struct QueryForkEventsParams {
    pub entity_id: Option<String>,
    pub event_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub async fn query_fork_events_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Path(fork_id): Path<String>,
    Query(params): Query<QueryForkEventsParams>,
) -> Result<Json<QueryForkEventsResponse>>
where
    F: ForkRepository + 'static,
{
    let use_case = QueryForkEventsUseCase::new(state.fork_repo.clone());
    let request = QueryForkEventsRequest {
        fork_id,
        entity_id: params.entity_id,
        event_type: params.event_type,
        limit: params.limit,
        offset: params.offset,
    };

    let response = use_case.execute(request).await?;

    tracing::debug!(count = response.count, "Queried fork events");

    Ok(Json(response))
}

/// Merge a fork back to its parent or main store
///
/// POST /api/v1/forks/{fork_id}/merge
pub async fn merge_fork_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Path(fork_id): Path<String>,
    Json(mut request): Json<MergeForkRequest>,
) -> Result<Json<MergeForkResponse>>
where
    F: ForkRepository + 'static,
{
    // Ensure the fork ID matches the path parameter
    request.fork_id = fork_id.clone();

    let use_case = MergeForkUseCase::new(state.fork_repo.clone());
    let response = use_case.execute(request).await?;

    tracing::info!(
        fork_id = %fork_id,
        events_committed = response.events_committed,
        "Fork merged successfully"
    );

    Ok(Json(response))
}

/// Discard a fork
///
/// POST /api/v1/forks/{fork_id}/discard
pub async fn discard_fork_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Path(fork_id): Path<String>,
) -> Result<Json<DiscardForkResponse>>
where
    F: ForkRepository + 'static,
{
    let use_case = DiscardForkUseCase::new(state.fork_repo.clone());

    let request = DiscardForkRequest {
        fork_id: fork_id.clone(),
    };

    let response = use_case.execute(request).await?;

    tracing::info!(
        fork_id = %fork_id,
        "Fork discarded"
    );

    Ok(Json(response))
}

/// Delete a fork (removes from storage)
///
/// DELETE /api/v1/forks/{fork_id}
pub async fn delete_fork_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Path(fork_id): Path<String>,
) -> Result<Json<serde_json::Value>>
where
    F: ForkRepository + 'static,
{
    let id = ForkId::parse(&fork_id)
        .map_err(|_| crate::error::AllSourceError::InvalidInput("Invalid fork ID".to_string()))?;

    state.fork_repo.delete(&id).await?;

    tracing::info!(
        fork_id = %fork_id,
        "Fork deleted"
    );

    Ok(Json(serde_json::json!({
        "deleted": true,
        "fork_id": fork_id
    })))
}

/// Cleanup expired forks
///
/// POST /api/v1/forks/cleanup
pub async fn cleanup_expired_forks_handler<F>(
    State(state): State<ForkHandlerState<F>>,
    Json(request): Json<CleanupExpiredForksRequest>,
) -> Result<Json<CleanupExpiredForksResponse>>
where
    F: ForkRepository + 'static,
{
    let use_case = CleanupExpiredForksUseCase::new(state.fork_repo.clone());

    let response = use_case.execute(request).await?;

    tracing::info!(
        forks_deleted = response.forks_deleted,
        "Expired forks cleaned up"
    );

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_forks_params_default() {
        let json = r#"{}"#;
        let params: ListForksParams = serde_json::from_str(json).unwrap();
        assert!(params.tenant_id.is_none());
        assert!(params.status.is_none());
    }

    #[test]
    fn test_list_forks_params_with_values() {
        let json = r#"{"tenant_id": "test", "status": "active", "limit": 10}"#;
        let params: ListForksParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.tenant_id, Some("test".to_string()));
        assert_eq!(params.status, Some("active".to_string()));
        assert_eq!(params.limit, Some(10));
    }

    #[test]
    fn test_query_fork_events_params() {
        let json = r#"{"entity_id": "entity-1", "event_type": "created", "limit": 50}"#;
        let params: QueryForkEventsParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.entity_id, Some("entity-1".to_string()));
        assert_eq!(params.event_type, Some("created".to_string()));
        assert_eq!(params.limit, Some(50));
    }
}
