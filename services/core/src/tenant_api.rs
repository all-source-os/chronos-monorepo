use crate::middleware::{Admin, Authenticated};
use crate::tenant::{Tenant, TenantManager, TenantQuotas};
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// AppState is defined in api_v1.rs
use crate::api_v1::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub quota_preset: Option<String>, // "free", "professional", "unlimited"
    pub quotas: Option<TenantQuotas>,
}

#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub quotas: TenantQuotas,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub active: bool,
}

impl From<Tenant> for TenantResponse {
    fn from(tenant: Tenant) -> Self {
        Self {
            id: tenant.id,
            name: tenant.name,
            description: tenant.description,
            quotas: tenant.quotas,
            created_at: tenant.created_at,
            updated_at: tenant.updated_at,
            active: tenant.active,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuotasRequest {
    pub quotas: TenantQuotas,
}

// ============================================================================
// Handlers
// ============================================================================

/// Create tenant (admin only)
/// POST /api/v1/tenants
pub async fn create_tenant_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    Json(req): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>), (StatusCode, String)> {
    // Determine quotas
    let quotas = if let Some(quotas) = req.quotas {
        quotas
    } else if let Some(preset) = req.quota_preset {
        match preset.as_str() {
            "free" => TenantQuotas::free_tier(),
            "professional" => TenantQuotas::professional(),
            "unlimited" => TenantQuotas::unlimited(),
            _ => TenantQuotas::default(),
        }
    } else {
        TenantQuotas::default()
    };

    let mut tenant = state
        .tenant_manager
        .create_tenant(req.id, req.name, quotas)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    if let Some(desc) = req.description {
        tenant.description = Some(desc);
    }

    Ok((StatusCode::CREATED, Json(tenant.into())))
}

/// Get tenant
/// GET /api/v1/tenants/:id
pub async fn get_tenant_handler(
    State(state): State<AppState>,
    Authenticated(auth_ctx): Authenticated,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Result<Json<TenantResponse>, (StatusCode, String)> {
    // Users can only view their own tenant, admins can view any
    if tenant_id != auth_ctx.tenant_id() {
        auth_ctx
            .require_permission(crate::auth::Permission::Admin)
            .map_err(|_| (StatusCode::FORBIDDEN, "Can only view own tenant".to_string()))?;
    }

    let tenant = state
        .tenant_manager
        .get_tenant(&tenant_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(tenant.into()))
}

/// List all tenants (admin only)
/// GET /api/v1/tenants
pub async fn list_tenants_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
) -> Json<Vec<TenantResponse>> {
    let tenants = state.tenant_manager.list_tenants();
    Json(tenants.into_iter().map(TenantResponse::from).collect())
}

/// Get tenant statistics
/// GET /api/v1/tenants/:id/stats
pub async fn get_tenant_stats_handler(
    State(state): State<AppState>,
    Authenticated(auth_ctx): Authenticated,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Users can only view their own tenant stats
    if tenant_id != auth_ctx.tenant_id() {
        auth_ctx
            .require_permission(crate::auth::Permission::Admin)
            .map_err(|_| (StatusCode::FORBIDDEN, "Can only view own tenant stats".to_string()))?;
    }

    let stats = state
        .tenant_manager
        .get_stats(&tenant_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(stats))
}

/// Update tenant quotas (admin only)
/// PUT /api/v1/tenants/:id/quotas
pub async fn update_quotas_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
    Json(req): Json<UpdateQuotasRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .tenant_manager
        .update_quotas(&tenant_id, req.quotas)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Deactivate tenant (admin only)
/// POST /api/v1/tenants/:id/deactivate
pub async fn deactivate_tenant_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .tenant_manager
        .deactivate_tenant(&tenant_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Activate tenant (admin only)
/// POST /api/v1/tenants/:id/activate
pub async fn activate_tenant_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .tenant_manager
        .activate_tenant(&tenant_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Delete tenant (admin only)
/// DELETE /api/v1/tenants/:id
pub async fn delete_tenant_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .tenant_manager
        .delete_tenant(&tenant_id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
