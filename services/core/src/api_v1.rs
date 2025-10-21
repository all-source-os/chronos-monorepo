/// v1.0 API router with authentication and multi-tenancy
use crate::auth::AuthManager;
use crate::auth_api::*;
use crate::middleware::{auth_middleware, rate_limit_middleware, AuthState, RateLimitState};
use crate::rate_limit::RateLimiter;
use crate::store::EventStore;
use crate::tenant::TenantManager;
use crate::tenant_api::*;
use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Unified application state for all handlers
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<EventStore>,
    pub auth_manager: Arc<AuthManager>,
    pub tenant_manager: Arc<TenantManager>,
}

// Enable extracting Arc<EventStore> from AppState
// This allows handlers that expect State<Arc<EventStore>> to work with AppState
impl axum::extract::FromRef<AppState> for Arc<EventStore> {
    fn from_ref(state: &AppState) -> Self {
        state.store.clone()
    }
}

pub async fn serve_v1(
    store: Arc<EventStore>,
    auth_manager: Arc<AuthManager>,
    tenant_manager: Arc<TenantManager>,
    rate_limiter: Arc<RateLimiter>,
    addr: &str,
) -> anyhow::Result<()> {
    let app_state = AppState {
        store,
        auth_manager: auth_manager.clone(),
        tenant_manager,
    };

    let auth_state = AuthState {
        auth_manager: auth_manager.clone(),
    };

    let rate_limit_state = RateLimitState {
        rate_limiter,
    };

    let app = Router::new()
        // Public routes (no auth)
        .route("/health", get(crate::api::health))
        .route("/metrics", get(crate::api::prometheus_metrics))
        // Auth routes
        .route("/api/v1/auth/register", post(register_handler))
        .route("/api/v1/auth/login", post(login_handler))
        .route("/api/v1/auth/me", get(me_handler))
        .route("/api/v1/auth/api-keys", post(create_api_key_handler))
        .route("/api/v1/auth/api-keys", get(list_api_keys_handler))
        .route("/api/v1/auth/api-keys/:id", delete(revoke_api_key_handler))
        .route("/api/v1/auth/users", get(list_users_handler))
        .route("/api/v1/auth/users/:id", delete(delete_user_handler))
        // Tenant routes (protected)
        .route("/api/v1/tenants", post(create_tenant_handler))
        .route("/api/v1/tenants", get(list_tenants_handler))
        .route("/api/v1/tenants/:id", get(get_tenant_handler))
        .route("/api/v1/tenants/:id/stats", get(get_tenant_stats_handler))
        .route("/api/v1/tenants/:id/quotas", put(update_quotas_handler))
        .route("/api/v1/tenants/:id/deactivate", post(deactivate_tenant_handler))
        .route("/api/v1/tenants/:id/activate", post(activate_tenant_handler))
        .route("/api/v1/tenants/:id", delete(delete_tenant_handler))
        // Event and data routes (protected by auth)
        .route("/api/v1/events", post(crate::api::ingest_event))
        .route("/api/v1/events/query", get(crate::api::query_events))
        .route("/api/v1/events/stream", get(crate::api::events_websocket))
        .route("/api/v1/entities/:entity_id/state", get(crate::api::get_entity_state))
        .route("/api/v1/entities/:entity_id/snapshot", get(crate::api::get_entity_snapshot))
        .route("/api/v1/stats", get(crate::api::get_stats))
        // Analytics
        .route("/api/v1/analytics/frequency", get(crate::api::analytics_frequency))
        .route("/api/v1/analytics/summary", get(crate::api::analytics_summary))
        .route("/api/v1/analytics/correlation", get(crate::api::analytics_correlation))
        // Snapshots
        .route("/api/v1/snapshots", post(crate::api::create_snapshot))
        .route("/api/v1/snapshots", get(crate::api::list_snapshots))
        .route("/api/v1/snapshots/:entity_id/latest", get(crate::api::get_latest_snapshot))
        // Compaction
        .route("/api/v1/compaction/trigger", post(crate::api::trigger_compaction))
        .route("/api/v1/compaction/stats", get(crate::api::compaction_stats))
        // Schemas
        .route("/api/v1/schemas", post(crate::api::register_schema))
        .route("/api/v1/schemas", get(crate::api::list_subjects))
        .route("/api/v1/schemas/:subject", get(crate::api::get_schema))
        .route("/api/v1/schemas/:subject/versions", get(crate::api::list_schema_versions))
        .route("/api/v1/schemas/validate", post(crate::api::validate_event_schema))
        .route("/api/v1/schemas/:subject/compatibility", put(crate::api::set_compatibility_mode))
        // Replay
        .route("/api/v1/replay", post(crate::api::start_replay))
        .route("/api/v1/replay", get(crate::api::list_replays))
        .route("/api/v1/replay/:replay_id", get(crate::api::get_replay_progress))
        .route("/api/v1/replay/:replay_id/cancel", post(crate::api::cancel_replay))
        .route("/api/v1/replay/:replay_id", delete(crate::api::delete_replay))
        // Pipelines
        .route("/api/v1/pipelines", post(crate::api::register_pipeline))
        .route("/api/v1/pipelines", get(crate::api::list_pipelines))
        .route("/api/v1/pipelines/stats", get(crate::api::all_pipeline_stats))
        .route("/api/v1/pipelines/:pipeline_id", get(crate::api::get_pipeline))
        .route("/api/v1/pipelines/:pipeline_id", delete(crate::api::remove_pipeline))
        .route("/api/v1/pipelines/:pipeline_id/stats", get(crate::api::get_pipeline_stats))
        .route("/api/v1/pipelines/:pipeline_id/reset", put(crate::api::reset_pipeline))
        .with_state(app_state)
        .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
        .layer(middleware::from_fn_with_state(rate_limit_state, rate_limit_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
