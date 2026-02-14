use crate::{
    domain::{
        entities::{Actor, AuditAction, AuditCategory, AuditEvent, AuditOutcome},
        repositories::audit_event_repository::AuditEventQuery,
        value_objects::TenantId,
    },
    error::{AllSourceError, Result},
    infrastructure::{security::middleware::Admin, web::api_v1::AppState},
};
use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LogAuditEventRequest {
    pub tenant_id: String,
    pub action: String,
    pub actor_type: String,
    pub actor_id: String,
    pub actor_name: String,
    pub outcome: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AuditQueryParams {
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub category: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub security_only: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct AuditEventResponse {
    pub id: String,
    pub tenant_id: String,
    pub timestamp: DateTime<Utc>,
    pub action: AuditAction,
    pub actor: Actor,
    pub outcome: AuditOutcome,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub ip_address: Option<String>,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl From<&AuditEvent> for AuditEventResponse {
    fn from(event: &AuditEvent) -> Self {
        Self {
            id: event.id().as_str(),
            tenant_id: event.tenant_id().as_str().to_string(),
            timestamp: *event.timestamp(),
            action: event.action().clone(),
            actor: event.actor().clone(),
            outcome: event.outcome().clone(),
            resource_type: event.resource_type().map(|s| s.to_string()),
            resource_id: event.resource_id().map(|s| s.to_string()),
            ip_address: event.ip_address().map(|s| s.to_string()),
            error_message: event.error_message().map(|s| s.to_string()),
            metadata: event.metadata().cloned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AuditEventsResponse {
    pub events: Vec<AuditEventResponse>,
    pub total: usize,
}

// ============================================================================
// Handlers
// ============================================================================

/// Log an audit event
/// POST /api/v1/audit/events
pub async fn log_audit_event(
    State(state): State<AppState>,
    Admin(_): Admin,
    Json(req): Json<LogAuditEventRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let audit_repo = state
        .service_container
        .audit_repository()
        .ok_or_else(|| AllSourceError::InternalError("Audit repository not configured".into()))?;

    let tenant_id =
        TenantId::new(req.tenant_id).map_err(|e| AllSourceError::InvalidInput(e.to_string()))?;

    let action = parse_audit_action(&req.action)
        .ok_or_else(|| AllSourceError::InvalidInput(format!("Unknown action: {}", req.action)))?;

    let actor = match req.actor_type.as_str() {
        "user" => Actor::user(req.actor_id, req.actor_name),
        "api_key" => Actor::api_key(req.actor_id, req.actor_name),
        "system" => Actor::system(req.actor_name),
        _ => {
            return Err(AllSourceError::InvalidInput(format!(
                "Unknown actor_type: {}",
                req.actor_type
            )));
        }
    };

    let outcome = match req.outcome.as_deref() {
        Some("failure") => AuditOutcome::Failure,
        Some("partial_success") => AuditOutcome::PartialSuccess,
        _ => AuditOutcome::Success,
    };

    let mut event = AuditEvent::new(tenant_id, action, actor, outcome);

    if let (Some(rt), Some(ri)) = (req.resource_type, req.resource_id) {
        event = event.with_resource(rt, ri);
    }
    if let Some(ip) = req.ip_address {
        event = event.with_ip_address(ip);
    }
    if let Some(ua) = req.user_agent {
        event = event.with_user_agent(ua);
    }
    if let Some(err) = req.error_message {
        event = event.with_error(err);
    }
    if let Some(meta) = req.metadata {
        event = event.with_metadata(meta);
    }

    let event_id = event.id().as_str();
    audit_repo.append(event).await?;

    tracing::debug!("Audit event logged: {}", event_id);

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "id": event_id,
            "saved": true,
        })),
    ))
}

/// Query audit events with filters
/// GET /api/v1/audit/events
pub async fn query_audit_events(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Query(params): axum::extract::Query<AuditQueryParams>,
) -> Result<Json<AuditEventsResponse>> {
    let audit_repo = state
        .service_container
        .audit_repository()
        .ok_or_else(|| AllSourceError::InternalError("Audit repository not configured".into()))?;

    let tenant_id_str = params.tenant_id.as_deref().unwrap_or("default");
    let tenant_id = TenantId::new(tenant_id_str.to_string())
        .map_err(|e| AllSourceError::InvalidInput(e.to_string()))?;

    let mut query = AuditEventQuery::new(tenant_id);

    if let (Some(start), Some(end)) = (params.start, params.end) {
        query = query.with_time_range(start, end);
    }
    if let Some(ref action_str) = params.action
        && let Some(action) = parse_audit_action(action_str)
    {
        query = query.with_action(action);
    }
    if let Some(ref category_str) = params.category
        && let Some(category) = parse_audit_category(category_str)
    {
        query = query.with_category(category);
    }
    if let Some(ref user_id) = params.user_id {
        query = query.with_actor(format!("user:{user_id}"));
    }
    if params.security_only.unwrap_or(false) {
        query = query.security_only();
    }

    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);
    query = query.with_pagination(limit, offset);

    let events = audit_repo.query(query.clone()).await?;
    let total = audit_repo.count(query).await?;

    let event_responses: Vec<AuditEventResponse> =
        events.iter().map(AuditEventResponse::from).collect();

    Ok(Json(AuditEventsResponse {
        events: event_responses,
        total,
    }))
}

// ============================================================================
// Helpers
// ============================================================================

fn parse_audit_action(s: &str) -> Option<AuditAction> {
    match s {
        "login" => Some(AuditAction::Login),
        "logout" => Some(AuditAction::Logout),
        "login_failed" => Some(AuditAction::LoginFailed),
        "token_refreshed" => Some(AuditAction::TokenRefreshed),
        "password_changed" => Some(AuditAction::PasswordChanged),
        "api_key_created" => Some(AuditAction::ApiKeyCreated),
        "api_key_revoked" => Some(AuditAction::ApiKeyRevoked),
        "api_key_used" => Some(AuditAction::ApiKeyUsed),
        "event_ingested" => Some(AuditAction::EventIngested),
        "event_queried" => Some(AuditAction::EventQueried),
        "event_stream_created" => Some(AuditAction::EventStreamCreated),
        "tenant_created" => Some(AuditAction::TenantCreated),
        "tenant_updated" => Some(AuditAction::TenantUpdated),
        "tenant_activated" => Some(AuditAction::TenantActivated),
        "tenant_deactivated" => Some(AuditAction::TenantDeactivated),
        "tenant_deleted" => Some(AuditAction::TenantDeleted),
        "schema_registered" => Some(AuditAction::SchemaRegistered),
        "schema_updated" => Some(AuditAction::SchemaUpdated),
        "schema_deleted" => Some(AuditAction::SchemaDeleted),
        "user_created" => Some(AuditAction::UserCreated),
        "user_updated" => Some(AuditAction::UserUpdated),
        "user_deleted" => Some(AuditAction::UserDeleted),
        "role_changed" => Some(AuditAction::RoleChanged),
        "permission_denied" => Some(AuditAction::PermissionDenied),
        "rate_limit_exceeded" => Some(AuditAction::RateLimitExceeded),
        "configuration_changed" => Some(AuditAction::ConfigurationChanged),
        "backup_created" => Some(AuditAction::BackupCreated),
        "backup_restored" => Some(AuditAction::BackupRestored),
        _ => None,
    }
}

fn parse_audit_category(s: &str) -> Option<AuditCategory> {
    match s {
        "authentication" => Some(AuditCategory::Authentication),
        "api_key" => Some(AuditCategory::ApiKey),
        "event" => Some(AuditCategory::Event),
        "tenant" => Some(AuditCategory::Tenant),
        "schema" => Some(AuditCategory::Schema),
        "projection" => Some(AuditCategory::Projection),
        "pipeline" => Some(AuditCategory::Pipeline),
        "user" => Some(AuditCategory::User),
        "security" => Some(AuditCategory::Security),
        "system" => Some(AuditCategory::System),
        _ => None,
    }
}
