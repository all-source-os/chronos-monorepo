// Creator HTTP handlers - delegate to CreatorCoordinator and use cases
// Clean Architecture: Infrastructure Layer (HTTP) -> Application Layer (Coordinator/Use Cases)

use crate::application::dto::{
    CreatorDto, ListCreatorsResponse, RegisterCreatorRequest, RegisterCreatorResponse,
    UpdateCreatorRequest, UpdateCreatorResponse, UpgradeCreatorTierRequest,
};
use crate::application::services::CreatorCoordinator;
use crate::application::use_cases::{
    DeactivateCreatorUseCase, ListCreatorsUseCase, ReactivateCreatorUseCase, SuspendCreatorUseCase,
    UpdateCreatorUseCase, UpgradeCreatorTierUseCase, VerifyCreatorEmailUseCase,
};
use crate::domain::repositories::CreatorRepository;
use crate::domain::value_objects::CreatorId;
use crate::error::Result;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

/// Application state for creator handlers
#[derive(Clone)]
pub struct CreatorHandlerState<R: CreatorRepository> {
    pub coordinator: Arc<CreatorCoordinator>,
    pub creator_repo: Arc<R>,
}

/// Query parameters for listing creators
#[derive(Debug, Deserialize)]
pub struct ListCreatorsParams {
    pub tenant_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Register a new creator
///
/// POST /api/v1/creators
pub async fn register_creator_handler<R: CreatorRepository + 'static>(
    State(state): State<CreatorHandlerState<R>>,
    Json(request): Json<RegisterCreatorRequest>,
) -> Result<Json<RegisterCreatorResponse>> {
    tracing::info!(
        email = %request.email,
        tenant_id = %request.tenant_id,
        "Registering new creator"
    );

    let response = state.coordinator.register_creator(request).await?;

    tracing::info!(
        creator_id = %response.creator.id,
        "Creator registered successfully"
    );

    Ok(Json(response))
}

/// List creators with optional filtering
///
/// GET /api/v1/creators
pub async fn list_creators_handler<R: CreatorRepository + 'static>(
    State(state): State<CreatorHandlerState<R>>,
    Query(params): Query<ListCreatorsParams>,
) -> Result<Json<ListCreatorsResponse>> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    let creators = if let Some(tenant_id) = params.tenant_id {
        let tenant = crate::domain::value_objects::TenantId::new(tenant_id)?;
        state
            .creator_repo
            .find_by_tenant(&tenant, limit, offset)
            .await?
    } else {
        state.creator_repo.find_active(limit, offset).await?
    };

    let response = ListCreatorsUseCase::execute(creators);

    tracing::debug!(count = response.count, "Listed creators");

    Ok(Json(response))
}

/// Get a specific creator by ID
///
/// GET /api/v1/creators/{creator_id}
pub async fn get_creator_handler<R: CreatorRepository + 'static>(
    State(state): State<CreatorHandlerState<R>>,
    Path(creator_id): Path<String>,
) -> Result<Json<CreatorDto>> {
    let id = CreatorId::parse(&creator_id).map_err(|_| {
        crate::error::AllSourceError::InvalidInput("Invalid creator ID".to_string())
    })?;

    let creator = state.creator_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Creator not found: {creator_id}"))
    })?;

    Ok(Json(CreatorDto::from(&creator)))
}

/// Update a creator
///
/// PUT /api/v1/creators/{creator_id}
pub async fn update_creator_handler<R: CreatorRepository + 'static>(
    State(state): State<CreatorHandlerState<R>>,
    Path(creator_id): Path<String>,
    Json(request): Json<UpdateCreatorRequest>,
) -> Result<Json<UpdateCreatorResponse>> {
    let id = CreatorId::parse(&creator_id).map_err(|_| {
        crate::error::AllSourceError::InvalidInput("Invalid creator ID".to_string())
    })?;

    let creator = state.creator_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Creator not found: {creator_id}"))
    })?;

    let use_case = UpdateCreatorUseCase::new(state.creator_repo.clone());
    let response = use_case.execute(creator, request).await?;

    tracing::info!(
        creator_id = %creator_id,
        "Creator updated successfully"
    );

    Ok(Json(response))
}

/// Upgrade a creator's tier
///
/// POST /api/v1/creators/{creator_id}/upgrade
pub async fn upgrade_creator_tier_handler<R: CreatorRepository + 'static>(
    State(state): State<CreatorHandlerState<R>>,
    Path(creator_id): Path<String>,
    Json(request): Json<UpgradeCreatorTierRequest>,
) -> Result<Json<UpdateCreatorResponse>> {
    let id = CreatorId::parse(&creator_id).map_err(|_| {
        crate::error::AllSourceError::InvalidInput("Invalid creator ID".to_string())
    })?;

    let creator = state.creator_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Creator not found: {creator_id}"))
    })?;

    let dto = UpgradeCreatorTierUseCase::execute(creator, request.tier)?;

    // Save the updated creator
    let updated_creator = {
        let id = CreatorId::parse(&creator_id).unwrap();
        state.creator_repo.find_by_id(&id).await?.unwrap()
    };
    state.creator_repo.save(&updated_creator).await?;

    tracing::info!(
        creator_id = %creator_id,
        tier = ?request.tier,
        "Creator tier upgraded"
    );

    Ok(Json(UpdateCreatorResponse { creator: dto }))
}

/// Verify a creator's email
///
/// POST /api/v1/creators/{creator_id}/verify-email
pub async fn verify_creator_email_handler<R: CreatorRepository + 'static>(
    State(state): State<CreatorHandlerState<R>>,
    Path(creator_id): Path<String>,
) -> Result<Json<UpdateCreatorResponse>> {
    let id = CreatorId::parse(&creator_id).map_err(|_| {
        crate::error::AllSourceError::InvalidInput("Invalid creator ID".to_string())
    })?;

    let creator = state.creator_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Creator not found: {creator_id}"))
    })?;

    let dto = VerifyCreatorEmailUseCase::execute(creator)?;

    tracing::info!(
        creator_id = %creator_id,
        "Creator email verified"
    );

    Ok(Json(UpdateCreatorResponse { creator: dto }))
}

/// Suspend a creator
///
/// POST /api/v1/creators/{creator_id}/suspend
pub async fn suspend_creator_handler<R: CreatorRepository + 'static>(
    State(state): State<CreatorHandlerState<R>>,
    Path(creator_id): Path<String>,
) -> Result<Json<UpdateCreatorResponse>> {
    let id = CreatorId::parse(&creator_id).map_err(|_| {
        crate::error::AllSourceError::InvalidInput("Invalid creator ID".to_string())
    })?;

    let creator = state.creator_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Creator not found: {creator_id}"))
    })?;

    let dto = SuspendCreatorUseCase::execute(creator)?;

    tracing::info!(
        creator_id = %creator_id,
        "Creator suspended"
    );

    Ok(Json(UpdateCreatorResponse { creator: dto }))
}

/// Reactivate a suspended creator
///
/// POST /api/v1/creators/{creator_id}/reactivate
pub async fn reactivate_creator_handler<R: CreatorRepository + 'static>(
    State(state): State<CreatorHandlerState<R>>,
    Path(creator_id): Path<String>,
) -> Result<Json<UpdateCreatorResponse>> {
    let id = CreatorId::parse(&creator_id).map_err(|_| {
        crate::error::AllSourceError::InvalidInput("Invalid creator ID".to_string())
    })?;

    let creator = state.creator_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Creator not found: {creator_id}"))
    })?;

    let dto = ReactivateCreatorUseCase::execute(creator)?;

    tracing::info!(
        creator_id = %creator_id,
        "Creator reactivated"
    );

    Ok(Json(UpdateCreatorResponse { creator: dto }))
}

/// Deactivate a creator
///
/// POST /api/v1/creators/{creator_id}/deactivate
pub async fn deactivate_creator_handler<R: CreatorRepository + 'static>(
    State(state): State<CreatorHandlerState<R>>,
    Path(creator_id): Path<String>,
) -> Result<Json<UpdateCreatorResponse>> {
    let id = CreatorId::parse(&creator_id).map_err(|_| {
        crate::error::AllSourceError::InvalidInput("Invalid creator ID".to_string())
    })?;

    let creator = state.creator_repo.find_by_id(&id).await?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Creator not found: {creator_id}"))
    })?;

    let dto = DeactivateCreatorUseCase::execute(creator)?;

    tracing::info!(
        creator_id = %creator_id,
        "Creator deactivated"
    );

    Ok(Json(UpdateCreatorResponse { creator: dto }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_creators_params_default() {
        let json = r#"{}"#;
        let params: ListCreatorsParams = serde_json::from_str(json).unwrap();
        assert!(params.tenant_id.is_none());
        assert!(params.limit.is_none());
        assert!(params.offset.is_none());
    }

    #[test]
    fn test_list_creators_params_with_values() {
        let json = r#"{"tenant_id": "test", "limit": 10, "offset": 5}"#;
        let params: ListCreatorsParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.tenant_id, Some("test".to_string()));
        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(5));
    }
}
