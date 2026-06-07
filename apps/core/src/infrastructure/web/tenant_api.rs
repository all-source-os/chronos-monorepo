use crate::{
    domain::{
        entities::{SchemaEnforcement, TenantQuotas},
        value_objects::TenantId,
    },
    infrastructure::security::middleware::{Admin, Authenticated},
};
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

// AppState is defined in api_v1.rs
use crate::infrastructure::web::api_v1::AppState;

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
    #[serde(default)]
    pub is_demo: bool,
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
    pub is_demo: bool,
    pub schema_enforcement: SchemaEnforcement,
}

impl TenantResponse {
    fn from_domain(tenant: &crate::domain::entities::Tenant) -> Self {
        Self {
            id: tenant.id().as_str().to_string(),
            name: tenant.name().to_string(),
            description: tenant.description().map(std::string::ToString::to_string),
            quotas: tenant.quotas().clone(),
            created_at: tenant.created_at(),
            updated_at: tenant.updated_at(),
            active: tenant.is_active(),
            is_demo: tenant.is_demo(),
            schema_enforcement: tenant.schema_enforcement(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_demo: Option<bool>,
    pub quotas: Option<TenantQuotas>,
    /// Operational metadata (subscription tier, quotas, billing). Replaces the
    /// tenant's metadata when present — callers send the full merged map. This
    /// is how the Control Plane persists subscription/entitlement state.
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuotasRequest {
    pub quotas: TenantQuotas,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSchemaEnforcementRequest {
    /// One of `permissive`, `warn`, `strict`.
    pub schema_enforcement: SchemaEnforcement,
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

    let tenant_id = TenantId::new(req.id).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let mut tenant = state
        .tenant_repo
        .create(tenant_id, req.name, quotas)
        .await
        .map_err(|e| {
            let status = match &e {
                crate::error::AllSourceError::TenantAlreadyExists(_) => StatusCode::CONFLICT,
                _ => StatusCode::BAD_REQUEST,
            };
            (status, e.to_string())
        })?;

    // Apply optional fields that aren't part of create()
    if let Some(desc) = req.description {
        tenant.update_description(Some(desc));
    }
    if req.is_demo {
        tenant.set_is_demo(true);
    }

    // Persist the updated fields
    state
        .tenant_repo
        .save(&tenant)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(TenantResponse::from_domain(&tenant)),
    ))
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
            .require_permission(crate::infrastructure::security::auth::Permission::Admin)
            .map_err(|_| {
                (
                    StatusCode::FORBIDDEN,
                    "Can only view own tenant".to_string(),
                )
            })?;
    }

    let tid =
        TenantId::new(tenant_id.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let tenant = state
        .tenant_repo
        .find_by_id(&tid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Tenant not found: {tenant_id}"),
            )
        })?;

    Ok(Json(TenantResponse::from_domain(&tenant)))
}

/// List all tenants (admin only)
/// GET /api/v1/tenants
pub async fn list_tenants_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
) -> Result<Json<Vec<TenantResponse>>, (StatusCode, String)> {
    let tenants = state
        .tenant_repo
        .find_all(10_000, 0)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        tenants.iter().map(TenantResponse::from_domain).collect(),
    ))
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
            .require_permission(crate::infrastructure::security::auth::Permission::Admin)
            .map_err(|_| {
                (
                    StatusCode::FORBIDDEN,
                    "Can only view own tenant stats".to_string(),
                )
            })?;
    }

    let tid =
        TenantId::new(tenant_id.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let tenant = state
        .tenant_repo
        .find_by_id(&tid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Tenant not found: {tenant_id}"),
            )
        })?;

    let stats = build_tenant_stats(&tenant);
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
    let tid =
        TenantId::new(tenant_id.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let updated = state
        .tenant_repo
        .update_quotas(&tid, req.quotas)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !updated {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Tenant not found: {tenant_id}"),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Set a tenant's schema-enforcement mode (admin only).
/// PUT /api/v1/tenants/:id/schema-enforcement
///
/// Gap 3 toggle: flips registered-schema validation on ingest between
/// `permissive` (default), `warn`, and `strict`. The gateway exposes this to a
/// tenant for their own tenant; Core keeps it admin-gated and internal.
pub async fn update_schema_enforcement_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
    Json(req): Json<UpdateSchemaEnforcementRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let tid =
        TenantId::new(tenant_id.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let updated = state
        .tenant_repo
        .update_schema_enforcement(&tid, req.schema_enforcement)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !updated {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Tenant not found: {tenant_id}"),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Deactivate tenant (admin only)
/// POST /api/v1/tenants/:id/deactivate
pub async fn deactivate_tenant_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let tid =
        TenantId::new(tenant_id.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let deactivated = state
        .tenant_repo
        .deactivate(&tid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !deactivated {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Tenant not found: {tenant_id}"),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Activate tenant (admin only)
/// POST /api/v1/tenants/:id/activate
pub async fn activate_tenant_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let tid =
        TenantId::new(tenant_id.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let activated = state
        .tenant_repo
        .activate(&tid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !activated {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Tenant not found: {tenant_id}"),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Update tenant (admin only)
/// PUT /api/v1/tenants/:id
pub async fn update_tenant_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
    Json(req): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>, (StatusCode, String)> {
    let tid =
        TenantId::new(tenant_id.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let mut tenant = state
        .tenant_repo
        .find_by_id(&tid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Tenant not found: {tenant_id}"),
            )
        })?;

    if let Some(name) = req.name {
        tenant
            .update_name(name)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }
    if let Some(desc) = req.description {
        tenant.update_description(Some(desc));
    }
    if let Some(is_demo) = req.is_demo {
        tenant.set_is_demo(is_demo);
    }
    if let Some(quotas) = req.quotas {
        tenant.update_quotas(quotas);
    }
    if let Some(metadata) = req.metadata {
        tenant.update_metadata(metadata);
    }

    state
        .tenant_repo
        .save(&tenant)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(TenantResponse::from_domain(&tenant)))
}

/// Delete tenant (admin only)
/// DELETE /api/v1/tenants/:id
pub async fn delete_tenant_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let tid =
        TenantId::new(tenant_id.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let deleted = state
        .tenant_repo
        .delete(&tid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !deleted {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Tenant not found: {tenant_id}"),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Helpers
// ============================================================================

/// Build tenant statistics JSON (presentation concern).
fn build_tenant_stats(tenant: &crate::domain::entities::Tenant) -> serde_json::Value {
    let quotas = tenant.quotas();
    let usage = tenant.usage();

    let events_pct = if quotas.max_events_per_day() > 0 {
        (usage.events_today() as f64 / quotas.max_events_per_day() as f64) * 100.0
    } else {
        0.0
    };

    let storage_pct = if quotas.max_storage_bytes() > 0 {
        (usage.storage_bytes() as f64 / quotas.max_storage_bytes() as f64) * 100.0
    } else {
        0.0
    };

    let queries_pct = if quotas.max_queries_per_hour() > 0 {
        (usage.queries_this_hour() as f64 / quotas.max_queries_per_hour() as f64) * 100.0
    } else {
        0.0
    };

    serde_json::json!({
        "tenant_id": tenant.id().as_str(),
        "name": tenant.name(),
        "active": tenant.is_active(),
        "is_demo": tenant.is_demo(),
        "usage": tenant.usage(),
        "quotas": tenant.quotas(),
        "utilization": {
            "events_today": {
                "used": usage.events_today(),
                "limit": quotas.max_events_per_day(),
                "percentage": events_pct.min(100.0)
            },
            "storage": {
                "used_bytes": usage.storage_bytes(),
                "limit_bytes": quotas.max_storage_bytes(),
                "percentage": storage_pct.min(100.0)
            },
            "queries_this_hour": {
                "used": usage.queries_this_hour(),
                "limit": quotas.max_queries_per_hour(),
                "percentage": queries_pct.min(100.0)
            }
        },
        "created_at": tenant.created_at(),
        "updated_at": tenant.updated_at()
    })
}
