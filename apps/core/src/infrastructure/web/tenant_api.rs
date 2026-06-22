use crate::{
    domain::{
        entities::{SchemaEnforcement, TenantQuotas, UsageMeter},
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
    /// Operational metadata (subscription tier, quotas, billing). Persisted via
    /// PUT /tenants/{id}; the Control Plane reads subscription/entitlement state
    /// from here. Must be serialized back out or the plan reads as "free".
    pub metadata: serde_json::Value,
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
            metadata: tenant.metadata().clone(),
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

/// Body for `POST /api/v1/tenants/{id}/usage/increment`.
///
/// `type` is the [`UsageMeter`] discriminator and DEFAULTS to `events` when
/// absent — that keeps the original untyped `{"count": N}` body the Query
/// Service sent before this field existed valid during a rolling deploy.
#[derive(Debug, Deserialize)]
pub struct IncrementUsageRequest {
    /// How much to add to the counter. Must be > 0. A `u64` field also makes
    /// serde reject negative numbers with a 400 before the handler runs.
    pub count: u64,
    /// Which meter to bump: `events` → `events_used`, `queries` →
    /// `queries_used`. Defaults to `events`.
    #[serde(default, rename = "type")]
    pub meter: UsageMeter,
}

/// Returned by the usage-increment endpoint: the meter that was bumped and its
/// new value, so the caller (and tests) can confirm the write landed.
#[derive(Debug, Serialize)]
pub struct IncrementUsageResponse {
    pub tenant_id: String,
    /// `events` or `queries`.
    #[serde(rename = "type")]
    pub meter: UsageMeter,
    /// The post-increment counter value (`metadata.quotas.<field>`).
    pub used: u64,
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

/// Increment a tenant's forward usage counter (admin only).
/// POST /api/v1/tenants/:id/usage/increment
///
/// Write half of forward usage-metering. The Query Service POSTs one increment
/// per metered activity — events ingested (`type: "events"`) or queries run
/// (`type: "queries"`) — and Core atomically bumps the matching
/// `metadata.quotas.{events_used,queries_used}` counter the dashboard reads.
///
/// Body: `{ "count": <u64>, "type": "events" | "queries" }`. `type` defaults
/// to `"events"` so the pre-typing `{count}` body keeps working during a
/// rolling deploy. Returns the post-increment value. `404` if the tenant is
/// unknown; `400` (via serde) for a missing/negative `count`; `400` for an
/// explicit `count: 0` (a no-op increment is a client bug, not a write).
///
/// Atomicity is the repository's job — see
/// [`TenantRepository::increment_usage`]; concurrent batched increments for
/// the same tenant are serialized so no counts are lost.
pub async fn increment_usage_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
    Json(req): Json<IncrementUsageRequest>,
) -> Result<Json<IncrementUsageResponse>, (StatusCode, String)> {
    if req.count == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "count must be greater than zero".to_string(),
        ));
    }

    let tid =
        TenantId::new(tenant_id.clone()).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let new_value = state
        .tenant_repo
        .increment_usage(&tid, req.meter, req.count)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Tenant not found: {tenant_id}"),
            )
        })?;

    Ok(Json(IncrementUsageResponse {
        tenant_id,
        meter: req.meter,
        used: new_value,
    }))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increment_request_defaults_meter_to_events() {
        // Backward compat: the pre-typing Query Service sent just `{count}`.
        // That body must still parse and target the events meter, so an old
        // QS hitting a new Core during a rolling deploy keeps metering events.
        let req: IncrementUsageRequest = serde_json::from_str(r#"{"count": 7}"#).unwrap();
        assert_eq!(req.count, 7);
        assert_eq!(req.meter, UsageMeter::Events);
        assert_eq!(req.meter.quota_field(), "events_used");
    }

    #[test]
    fn increment_request_parses_typed_events_and_queries() {
        let ev: IncrementUsageRequest =
            serde_json::from_str(r#"{"count": 3, "type": "events"}"#).unwrap();
        assert_eq!(ev.meter, UsageMeter::Events);

        let qs: IncrementUsageRequest =
            serde_json::from_str(r#"{"count": 9, "type": "queries"}"#).unwrap();
        assert_eq!(qs.meter, UsageMeter::Queries);
        assert_eq!(qs.meter.quota_field(), "queries_used");
    }

    #[test]
    fn increment_request_rejects_negative_count() {
        // `count: u64` makes serde reject negatives before the handler runs,
        // which axum surfaces as a 400 — no separate validation needed.
        let err = serde_json::from_str::<IncrementUsageRequest>(r#"{"count": -1}"#);
        assert!(err.is_err(), "negative count must fail to deserialize");
    }

    #[test]
    fn increment_response_serializes_type_and_used() {
        let resp = IncrementUsageResponse {
            tenant_id: "acme".to_string(),
            meter: UsageMeter::Queries,
            used: 42,
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["tenant_id"], "acme");
        assert_eq!(v["type"], "queries");
        assert_eq!(v["used"], 42);
    }
}
