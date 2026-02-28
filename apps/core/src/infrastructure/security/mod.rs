// Security infrastructure layer
// Contains authentication, authorization, rate limiting, and IP filtering

#[cfg(feature = "server")]
pub mod auth;
pub mod ip_filter;
#[cfg(feature = "server")]
pub mod middleware;
pub mod rate_limit;

// Re-exports for convenience
#[cfg(feature = "server")]
pub use auth::{ApiKey, AuthManager, Claims, Permission, Role, User};
pub use ip_filter::{FilterAction, FilterResult, IpFilter, IpFilterStats};
#[cfg(feature = "server")]
pub use middleware::{
    Admin, AuthContext, AuthState, Authenticated, OptionalAuth, RateLimitState, RequestId,
    SecurityConfig, TenantContext, auth_middleware, rate_limit_middleware,
};
pub use rate_limit::{RateLimitResult, RateLimiter};
