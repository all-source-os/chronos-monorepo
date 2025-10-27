use crate::auth::{AuthManager, Claims, Permission};
use crate::error::AllSourceError;
use crate::rate_limit::RateLimiter;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Authentication state shared across requests
#[derive(Clone)]
pub struct AuthState {
    pub auth_manager: Arc<AuthManager>,
}

/// Rate limiting state
#[derive(Clone)]
pub struct RateLimitState {
    pub rate_limiter: Arc<RateLimiter>,
}

/// Authenticated request context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub claims: Claims,
}

impl AuthContext {
    /// Check if user has required permission
    pub fn require_permission(&self, permission: Permission) -> Result<(), AllSourceError> {
        if self.claims.has_permission(permission) {
            Ok(())
        } else {
            Err(AllSourceError::ValidationError(
                "Insufficient permissions".to_string(),
            ))
        }
    }

    /// Get tenant ID from context
    pub fn tenant_id(&self) -> &str {
        &self.claims.tenant_id
    }

    /// Get user ID from context
    pub fn user_id(&self) -> &str {
        &self.claims.sub
    }
}

/// Extract token from Authorization header
fn extract_token(headers: &HeaderMap) -> Result<String, AllSourceError> {
    let auth_header = headers
        .get("authorization")
        .ok_or_else(|| AllSourceError::ValidationError("Missing authorization header".to_string()))?
        .to_str()
        .map_err(|_| AllSourceError::ValidationError("Invalid authorization header".to_string()))?;

    // Support both "Bearer <token>" and "<token>" formats
    let token = if auth_header.starts_with("Bearer ") {
        auth_header.trim_start_matches("Bearer ").trim()
    } else if auth_header.starts_with("bearer ") {
        auth_header.trim_start_matches("bearer ").trim()
    } else {
        auth_header.trim()
    };

    if token.is_empty() {
        return Err(AllSourceError::ValidationError(
            "Empty authorization token".to_string(),
        ));
    }

    Ok(token.to_string())
}

/// Authentication middleware
pub async fn auth_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let headers = request.headers();

    // Extract and validate token (JWT or API key)
    let token = extract_token(headers)?;

    let claims = if token.starts_with("ask_") {
        // API Key authentication
        auth_state.auth_manager.validate_api_key(&token)?
    } else {
        // JWT authentication
        auth_state.auth_manager.validate_token(&token)?
    };

    // Insert auth context into request extensions
    request.extensions_mut().insert(AuthContext { claims });

    Ok(next.run(request).await)
}

/// Optional authentication middleware (allows unauthenticated requests)
pub async fn optional_auth_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();

    if let Ok(token) = extract_token(headers) {
        // Try to authenticate, but don't fail if invalid
        let claims = if token.starts_with("ask_") {
            auth_state.auth_manager.validate_api_key(&token).ok()
        } else {
            auth_state.auth_manager.validate_token(&token).ok()
        };

        if let Some(claims) = claims {
            request.extensions_mut().insert(AuthContext { claims });
        }
    }

    next.run(request).await
}

/// Error type for authentication failures
#[derive(Debug)]
pub struct AuthError(AllSourceError);

impl From<AllSourceError> for AuthError {
    fn from(err: AllSourceError) -> Self {
        AuthError(err)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            AllSourceError::ValidationError(msg) => (StatusCode::UNAUTHORIZED, msg),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        (status, message).into_response()
    }
}

/// Axum extractor for authenticated requests
pub struct Authenticated(pub AuthContext);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .map(Authenticated)
            .ok_or((StatusCode::UNAUTHORIZED, "Unauthorized"))
    }
}

/// Axum extractor for admin-only requests
pub struct Admin(pub AuthContext);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for Admin
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth_ctx = parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or((StatusCode::UNAUTHORIZED, "Unauthorized"))?;

        auth_ctx
            .require_permission(Permission::Admin)
            .map_err(|_| (StatusCode::FORBIDDEN, "Admin permission required"))?;

        Ok(Admin(auth_ctx))
    }
}

/// Rate limiting middleware
/// Checks rate limits based on tenant_id from auth context
pub async fn rate_limit_middleware(
    State(rate_limit_state): State<RateLimitState>,
    request: Request,
    next: Next,
) -> Result<Response, RateLimitError> {
    // Extract auth context from request
    let auth_ctx = request
        .extensions()
        .get::<AuthContext>()
        .ok_or_else(|| RateLimitError::Unauthorized)?;

    // Check rate limit for this tenant
    let result = rate_limit_state
        .rate_limiter
        .check_rate_limit(auth_ctx.tenant_id());

    if !result.allowed {
        return Err(RateLimitError::RateLimitExceeded {
            retry_after: result.retry_after.unwrap_or_default().as_secs(),
            limit: result.limit,
        });
    }

    // Add rate limit headers to response
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert("X-RateLimit-Limit", result.limit.to_string().parse().unwrap());
    headers.insert("X-RateLimit-Remaining", result.remaining.to_string().parse().unwrap());

    Ok(response)
}

/// Error type for rate limiting failures
#[derive(Debug)]
pub enum RateLimitError {
    RateLimitExceeded { retry_after: u64, limit: u32 },
    Unauthorized,
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        match self {
            RateLimitError::RateLimitExceeded { retry_after, limit } => {
                let mut response = (
                    StatusCode::TOO_MANY_REQUESTS,
                    format!("Rate limit exceeded. Limit: {} requests/min", limit),
                )
                    .into_response();

                if retry_after > 0 {
                    response.headers_mut().insert(
                        "Retry-After",
                        retry_after.to_string().parse().unwrap(),
                    );
                }

                response
            }
            RateLimitError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Authentication required for rate limiting",
            )
                .into_response(),
        }
    }
}

/// Helper macro to require specific permission
#[macro_export]
macro_rules! require_permission {
    ($auth:expr, $perm:expr) => {
        $auth
            .0
            .require_permission($perm)
            .map_err(|_| (axum::http::StatusCode::FORBIDDEN, "Insufficient permissions"))?
    };
}

// ============================================================================
// Tenant Isolation Middleware (Phase 5B)
// ============================================================================

use crate::domain::entities::Tenant;
use crate::domain::repositories::TenantRepository;
use crate::domain::value_objects::TenantId;

/// Tenant isolation state for middleware
#[derive(Clone)]
pub struct TenantState<R: TenantRepository> {
    pub tenant_repository: Arc<R>,
}

/// Validated tenant context injected into requests
///
/// This context is created by the tenant_isolation_middleware after
/// validating that the tenant exists and is active.
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant: Tenant,
}

impl TenantContext {
    /// Get the tenant ID
    pub fn tenant_id(&self) -> &TenantId {
        self.tenant.id()
    }

    /// Check if tenant is active
    pub fn is_active(&self) -> bool {
        self.tenant.is_active()
    }
}

/// Tenant isolation middleware
///
/// Validates that the authenticated tenant exists and is active.
/// Injects TenantContext into the request for use by handlers.
///
/// # Phase 5B: Tenant Isolation
/// This middleware enforces tenant boundaries by:
/// 1. Extracting tenant_id from AuthContext
/// 2. Loading tenant from repository
/// 3. Validating tenant is active
/// 4. Injecting TenantContext into request extensions
///
/// Must be applied after auth_middleware.
pub async fn tenant_isolation_middleware<R: TenantRepository + 'static>(
    State(tenant_state): State<TenantState<R>>,
    mut request: Request,
    next: Next,
) -> Result<Response, TenantError> {
    // Extract auth context (must be authenticated)
    let auth_ctx = request
        .extensions()
        .get::<AuthContext>()
        .ok_or(TenantError::Unauthorized)?
        .clone();

    // Parse tenant ID
    let tenant_id = TenantId::new(auth_ctx.tenant_id().to_string())
        .map_err(|_| TenantError::InvalidTenant)?;

    // Load tenant from repository
    let tenant = tenant_state
        .tenant_repository
        .find_by_id(&tenant_id)
        .await
        .map_err(|e| TenantError::RepositoryError(e.to_string()))?
        .ok_or(TenantError::TenantNotFound)?;

    // Validate tenant is active
    if !tenant.is_active() {
        return Err(TenantError::TenantInactive);
    }

    // Inject tenant context into request
    request.extensions_mut().insert(TenantContext { tenant });

    // Continue to next middleware/handler
    Ok(next.run(request).await)
}

/// Error type for tenant isolation failures
#[derive(Debug)]
pub enum TenantError {
    Unauthorized,
    InvalidTenant,
    TenantNotFound,
    TenantInactive,
    RepositoryError(String),
}

impl IntoResponse for TenantError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            TenantError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Authentication required for tenant access",
            ),
            TenantError::InvalidTenant => (
                StatusCode::BAD_REQUEST,
                "Invalid tenant identifier",
            ),
            TenantError::TenantNotFound => (
                StatusCode::NOT_FOUND,
                "Tenant not found",
            ),
            TenantError::TenantInactive => (
                StatusCode::FORBIDDEN,
                "Tenant is inactive",
            ),
            TenantError::RepositoryError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to validate tenant",
            ),
        };

        (status, message).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Role, User};

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test_token_123".parse().unwrap());

        let token = extract_token(&headers).unwrap();
        assert_eq!(token, "test_token_123");
    }

    #[test]
    fn test_extract_lowercase_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "bearer test_token_123".parse().unwrap());

        let token = extract_token(&headers).unwrap();
        assert_eq!(token, "test_token_123");
    }

    #[test]
    fn test_extract_plain_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "test_token_123".parse().unwrap());

        let token = extract_token(&headers).unwrap();
        assert_eq!(token, "test_token_123");
    }

    #[test]
    fn test_missing_auth_header() {
        let headers = HeaderMap::new();
        assert!(extract_token(&headers).is_err());
    }

    #[test]
    fn test_auth_context_permissions() {
        let claims = Claims::new(
            "user1".to_string(),
            "tenant1".to_string(),
            Role::Developer,
            chrono::Duration::hours(1),
        );

        let ctx = AuthContext { claims };

        assert!(ctx.require_permission(Permission::Read).is_ok());
        assert!(ctx.require_permission(Permission::Write).is_ok());
        assert!(ctx.require_permission(Permission::Admin).is_err());
    }
}
