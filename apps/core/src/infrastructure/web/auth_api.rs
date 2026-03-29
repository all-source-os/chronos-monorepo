use crate::infrastructure::security::{
    auth::{Permission, Role, User},
    middleware::{Admin, Authenticated, OptionalAuth},
};
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// AppState is defined in api_v1.rs and re-exported
use crate::infrastructure::web::api_v1::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: Option<Role>,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub role: Role,
    pub tenant_id: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: Role,
    pub tenant_id: String,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role,
            tenant_id: user.tenant_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub role: Option<Role>,
    pub expires_in_days: Option<i64>,
    /// Admin override: create the key for a specific tenant instead of the caller's tenant.
    /// Only honoured when the caller has Admin permission; ignored otherwise.
    pub tenant_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub tenant_id: String,
    pub role: Role,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub active: bool,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

// ============================================================================
// Handlers
// ============================================================================

/// Register a new user
/// POST /api/v1/auth/register
pub async fn register_handler(
    State(state): State<AppState>,
    OptionalAuth(auth): OptionalAuth,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, String)> {
    // Only admins can register other users (or allow self-registration in dev mode)
    let is_admin_request = auth.is_some();
    if let Some(auth_ctx) = auth {
        auth_ctx
            .require_permission(Permission::Admin)
            .map_err(|_| {
                (
                    StatusCode::FORBIDDEN,
                    "Admin permission required".to_string(),
                )
            })?;
    }

    let role = req.role.unwrap_or(Role::Developer);

    // Tenant isolation: if no tenant_id is provided, create an isolated tenant
    // for this user. This prevents new users from seeing other users' data.
    // Admin-created users (with auth context) can specify a tenant_id explicitly.
    let tenant_id = if let Some(tid) = req.tenant_id {
        tid
    } else if is_admin_request {
        // Admin explicitly registering a user without tenant_id → use default
        "default".to_string()
    } else {
        // Self-registration: create an isolated tenant named after the user
        let tid = format!("tenant-{}", req.username.to_lowercase().replace(' ', "-"));
        tracing::info!("Auto-creating isolated tenant '{tid}' for new user '{}'", req.username);
        tid
    };

    let user = state
        .auth_manager
        .register_user(
            req.username,
            req.email,
            &req.password,
            role.clone(),
            tenant_id.clone(),
        )
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: user.id,
            username: user.username,
            email: user.email,
            role,
            tenant_id,
        }),
    ))
}

/// Login with username and password
/// POST /api/v1/auth/login
pub async fn login_handler(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let token = state
        .auth_manager
        .authenticate(&req.username, &req.password)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    // Get user info
    let user_id = state
        .auth_manager
        .validate_token(&token)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .sub
        .parse::<Uuid>()
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid user ID".to_string(),
            )
        })?;

    let user = state
        .auth_manager
        .get_user(&user_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    Ok(Json(LoginResponse {
        token,
        user: user.into(),
    }))
}

/// Get current user info
/// GET /api/v1/auth/me
///
/// Returns identity info for the authenticated caller. Works for:
/// - JWT-authenticated users (looks up full user record)
/// - API key authentication (returns identity from claims)
/// - Dev mode (returns synthetic dev identity from claims)
pub async fn me_handler(
    State(state): State<AppState>,
    Authenticated(auth_ctx): Authenticated,
) -> Result<Json<UserInfo>, (StatusCode, String)> {
    // Try to look up a full user record if the subject is a valid UUID
    if let Ok(user_id) = auth_ctx.user_id().parse::<Uuid>()
        && let Some(user) = state.auth_manager.get_user(&user_id)
    {
        return Ok(Json(user.into()));
    }

    // For API key auth and dev mode, synthesize identity from claims.
    // This is the path Query Service uses to validate API keys — it only
    // needs tenant_id and role, not a full user record.
    Ok(Json(UserInfo {
        id: auth_ctx.user_id().parse::<Uuid>().unwrap_or_default(),
        username: auth_ctx.user_id().to_string(),
        email: String::new(),
        role: auth_ctx.claims.role.clone(),
        tenant_id: auth_ctx.tenant_id().to_string(),
    }))
}

/// Create API key
/// POST /api/v1/auth/api-keys
///
/// Requires Admin permission. Service accounts and developers (including agent API keys)
/// cannot create new keys — only the control plane (which holds an admin service JWT)
/// may provision API keys on behalf of users.
pub async fn create_api_key_handler(
    State(state): State<AppState>,
    Authenticated(auth_ctx): Authenticated,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), (StatusCode, String)> {
    let role = req.role.unwrap_or(Role::ServiceAccount);

    // Only admins can create API keys.
    // Service accounts (agents) and developers are explicitly blocked so that
    // agent keys cannot self-replicate or escalate privileges.
    auth_ctx
        .require_permission(Permission::Admin)
        .map_err(|_| {
            (
                StatusCode::FORBIDDEN,
                "Admin permission required to create API keys".to_string(),
            )
        })?;

    let expires_at = req
        .expires_in_days
        .map(|days| chrono::Utc::now() + chrono::Duration::days(days));

    // Admins may specify a target tenant; everyone else is restricted to their own tenant.
    let effective_tenant_id = if let Some(ref tid) = req.tenant_id {
        if auth_ctx.require_permission(Permission::Admin).is_ok() {
            tid.clone()
        } else {
            auth_ctx.tenant_id().to_string()
        }
    } else {
        auth_ctx.tenant_id().to_string()
    };

    let (api_key, key) = state.auth_manager.create_api_key(
        req.name.clone(),
        effective_tenant_id,
        role,
        expires_at,
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateApiKeyResponse {
            id: api_key.id,
            name: req.name,
            key,
            expires_at,
        }),
    ))
}

/// List API keys
/// GET /api/v1/auth/api-keys
pub async fn list_api_keys_handler(
    State(state): State<AppState>,
    Authenticated(auth_ctx): Authenticated,
) -> Result<Json<Vec<ApiKeyInfo>>, (StatusCode, String)> {
    let keys = state.auth_manager.list_api_keys(auth_ctx.tenant_id());

    let key_infos: Vec<ApiKeyInfo> = keys
        .into_iter()
        .map(|k| ApiKeyInfo {
            id: k.id,
            name: k.name,
            tenant_id: k.tenant_id,
            role: k.role,
            created_at: k.created_at,
            expires_at: k.expires_at,
            active: k.active,
            last_used: k.last_used,
        })
        .collect();

    Ok(Json(key_infos))
}

/// Revoke API key
/// DELETE /api/v1/auth/api-keys/:id
pub async fn revoke_api_key_handler(
    State(state): State<AppState>,
    Authenticated(auth_ctx): Authenticated,
    axum::extract::Path(key_id): axum::extract::Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    auth_ctx
        .require_permission(Permission::Write)
        .map_err(|_| {
            (
                StatusCode::FORBIDDEN,
                "Write permission required".to_string(),
            )
        })?;

    state
        .auth_manager
        .revoke_api_key(&key_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// List all users (admin only)
/// GET /api/v1/auth/users
pub async fn list_users_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
) -> Json<Vec<UserInfo>> {
    let users = state.auth_manager.list_users();
    Json(users.into_iter().map(UserInfo::from).collect())
}

/// Delete user (admin only)
/// DELETE /api/v1/auth/users/:id
pub async fn delete_user_handler(
    State(state): State<AppState>,
    Admin(_): Admin,
    axum::extract::Path(user_id): axum::extract::Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .auth_manager
        .delete_user(&user_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
