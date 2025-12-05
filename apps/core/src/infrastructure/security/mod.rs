// Security infrastructure layer
// Contains authentication, authorization, rate limiting, and IP filtering

pub mod auth;
pub mod ip_filter;
pub mod middleware;
pub mod rate_limit;

// Re-exports for convenience
pub use auth::{ApiKey, AuthManager, Claims, Permission, Role, User};
pub use ip_filter::{FilterAction, FilterResult, IpFilter, IpFilterStats};
pub use middleware::{
    auth_middleware, rate_limit_middleware, Admin, AuthContext, AuthState, Authenticated,
    OptionalAuth, RateLimitState, RequestId, SecurityConfig, TenantContext,
};
pub use rate_limit::{RateLimitResult, RateLimiter};
