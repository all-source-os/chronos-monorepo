use crate::application::services::tenant_service::TenantManager;
/// v1.0 API router with authentication and multi-tenancy
use crate::infrastructure::di::ServiceContainer;
use crate::infrastructure::security::auth::AuthManager;
use crate::infrastructure::security::middleware::{
    auth_middleware, rate_limit_middleware, AuthState, RateLimitState,
};
use crate::infrastructure::security::rate_limit::RateLimiter;
use crate::infrastructure::web::auth_api::*;
use crate::infrastructure::web::tenant_api::*;
use crate::store::EventStore;
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
    /// Service container for paywall domain use cases (Creator, Article, Payment, etc.)
    pub service_container: ServiceContainer,
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
    service_container: ServiceContainer,
    addr: &str,
) -> anyhow::Result<()> {
    let app_state = AppState {
        store,
        auth_manager: auth_manager.clone(),
        tenant_manager,
        service_container,
    };

    let auth_state = AuthState {
        auth_manager: auth_manager.clone(),
    };

    let rate_limit_state = RateLimitState { rate_limiter };

    let app = Router::new()
        // Public routes (no auth)
        .route("/health", get(super::api::health))
        .route("/metrics", get(super::api::prometheus_metrics))
        // Auth routes
        .route("/api/v1/auth/register", post(register_handler))
        .route("/api/v1/auth/login", post(login_handler))
        .route("/api/v1/auth/me", get(me_handler))
        .route("/api/v1/auth/api-keys", post(create_api_key_handler))
        .route("/api/v1/auth/api-keys", get(list_api_keys_handler))
        .route("/api/v1/auth/api-keys/{id}", delete(revoke_api_key_handler))
        .route("/api/v1/auth/users", get(list_users_handler))
        .route("/api/v1/auth/users/{id}", delete(delete_user_handler))
        // Tenant routes (protected)
        .route("/api/v1/tenants", post(create_tenant_handler))
        .route("/api/v1/tenants", get(list_tenants_handler))
        .route("/api/v1/tenants/{id}", get(get_tenant_handler))
        .route("/api/v1/tenants/{id}/stats", get(get_tenant_stats_handler))
        .route("/api/v1/tenants/{id}/quotas", put(update_quotas_handler))
        .route(
            "/api/v1/tenants/{id}/deactivate",
            post(deactivate_tenant_handler),
        )
        .route(
            "/api/v1/tenants/{id}/activate",
            post(activate_tenant_handler),
        )
        .route("/api/v1/tenants/{id}", delete(delete_tenant_handler))
        // Event and data routes (protected by auth)
        .route("/api/v1/events", post(super::api::ingest_event))
        .route(
            "/api/v1/events/batch",
            post(super::api::ingest_events_batch),
        )
        .route("/api/v1/events/query", get(super::api::query_events))
        .route("/api/v1/events/stream", get(super::api::events_websocket))
        .route(
            "/api/v1/entities/{entity_id}/state",
            get(super::api::get_entity_state),
        )
        .route(
            "/api/v1/entities/{entity_id}/snapshot",
            get(super::api::get_entity_snapshot),
        )
        .route("/api/v1/stats", get(super::api::get_stats))
        // v0.10: Stream and event type discovery endpoints
        .route("/api/v1/streams", get(super::api::list_streams))
        .route("/api/v1/event-types", get(super::api::list_event_types))
        // Analytics
        .route(
            "/api/v1/analytics/frequency",
            get(super::api::analytics_frequency),
        )
        .route(
            "/api/v1/analytics/summary",
            get(super::api::analytics_summary),
        )
        .route(
            "/api/v1/analytics/correlation",
            get(super::api::analytics_correlation),
        )
        // Snapshots
        .route("/api/v1/snapshots", post(super::api::create_snapshot))
        .route("/api/v1/snapshots", get(super::api::list_snapshots))
        .route(
            "/api/v1/snapshots/{entity_id}/latest",
            get(super::api::get_latest_snapshot),
        )
        // Compaction
        .route(
            "/api/v1/compaction/trigger",
            post(super::api::trigger_compaction),
        )
        .route(
            "/api/v1/compaction/stats",
            get(super::api::compaction_stats),
        )
        // Schemas
        .route("/api/v1/schemas", post(super::api::register_schema))
        .route("/api/v1/schemas", get(super::api::list_subjects))
        .route("/api/v1/schemas/{subject}", get(super::api::get_schema))
        .route(
            "/api/v1/schemas/{subject}/versions",
            get(super::api::list_schema_versions),
        )
        .route(
            "/api/v1/schemas/validate",
            post(super::api::validate_event_schema),
        )
        .route(
            "/api/v1/schemas/{subject}/compatibility",
            put(super::api::set_compatibility_mode),
        )
        // Replay
        .route("/api/v1/replay", post(super::api::start_replay))
        .route("/api/v1/replay", get(super::api::list_replays))
        .route(
            "/api/v1/replay/{replay_id}",
            get(super::api::get_replay_progress),
        )
        .route(
            "/api/v1/replay/{replay_id}/cancel",
            post(super::api::cancel_replay),
        )
        .route(
            "/api/v1/replay/{replay_id}",
            delete(super::api::delete_replay),
        )
        // Pipelines
        .route("/api/v1/pipelines", post(super::api::register_pipeline))
        .route("/api/v1/pipelines", get(super::api::list_pipelines))
        .route(
            "/api/v1/pipelines/stats",
            get(super::api::all_pipeline_stats),
        )
        .route(
            "/api/v1/pipelines/{pipeline_id}",
            get(super::api::get_pipeline),
        )
        .route(
            "/api/v1/pipelines/{pipeline_id}",
            delete(super::api::remove_pipeline),
        )
        .route(
            "/api/v1/pipelines/{pipeline_id}/stats",
            get(super::api::get_pipeline_stats),
        )
        .route(
            "/api/v1/pipelines/{pipeline_id}/reset",
            put(super::api::reset_pipeline),
        )
        // v0.7: Projection State API for Query Service integration
        .route("/api/v1/projections", get(super::api::list_projections))
        .route(
            "/api/v1/projections/{name}",
            get(super::api::get_projection),
        )
        .route(
            "/api/v1/projections/{name}/{entity_id}/state",
            get(super::api::get_projection_state),
        )
        .route(
            "/api/v1/projections/{name}/{entity_id}/state",
            post(super::api::save_projection_state),
        )
        .route(
            "/api/v1/projections/{name}/{entity_id}/state",
            put(super::api::save_projection_state),
        )
        .route(
            "/api/v1/projections/{name}/bulk",
            post(super::api::bulk_get_projection_states),
        )
        .route(
            "/api/v1/projections/{name}/bulk/save",
            post(super::api::bulk_save_projection_states),
        )
        .with_state(app_state)
        // IMPORTANT: Middleware layers execute from bottom to top in Tower/Axum
        // Rate limit MUST come before auth so auth runs first and populates AuthContext
        .layer(middleware::from_fn_with_state(
            rate_limit_state,
            rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Graceful shutdown on SIGTERM (required for serverless platforms)
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("🛑 AllSource Core shutdown complete");
    Ok(())
}

/// Listen for shutdown signals (SIGTERM for serverless, SIGINT for local dev)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("📤 Received Ctrl+C, initiating graceful shutdown...");
        }
        _ = terminate => {
            tracing::info!("📤 Received SIGTERM, initiating graceful shutdown...");
        }
    }
}
