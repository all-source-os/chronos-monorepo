/// v1.0 API router with authentication and multi-tenancy
use crate::infrastructure::di::ServiceContainer;
use crate::{
    application::services::tenant_service::TenantManager,
    infrastructure::{
        replication::{WalReceiver, WalShipper},
        security::{
            auth::AuthManager,
            middleware::{AuthState, RateLimitState, auth_middleware, rate_limit_middleware},
            rate_limit::RateLimiter,
        },
        web::{audit_api::*, auth_api::*, config_api::*, tenant_api::*},
    },
    store::EventStore,
};
use axum::{
    Json, Router,
    extract::State,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

/// Node role for leader-follower replication
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    Leader,
    Follower,
}

impl NodeRole {
    /// Detect role from environment variables.
    ///
    /// Checks `ALLSOURCE_ROLE` ("leader" or "follower") first,
    /// then falls back to `ALLSOURCE_READ_ONLY` ("true" → follower).
    /// Defaults to `Leader` if neither is set.
    pub fn from_env() -> Self {
        if let Ok(role) = std::env::var("ALLSOURCE_ROLE") {
            match role.to_lowercase().as_str() {
                "follower" => return NodeRole::Follower,
                "leader" => return NodeRole::Leader,
                other => {
                    tracing::warn!(
                        "Unknown ALLSOURCE_ROLE value '{}', defaulting to leader",
                        other
                    );
                    return NodeRole::Leader;
                }
            }
        }
        if let Ok(read_only) = std::env::var("ALLSOURCE_READ_ONLY")
            && (read_only == "true" || read_only == "1")
        {
            return NodeRole::Follower;
        }
        NodeRole::Leader
    }

    pub fn is_follower(self) -> bool {
        self == NodeRole::Follower
    }

    fn to_u8(self) -> u8 {
        match self {
            NodeRole::Leader => 0,
            NodeRole::Follower => 1,
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => NodeRole::Follower,
            _ => NodeRole::Leader,
        }
    }
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::Leader => write!(f, "leader"),
            NodeRole::Follower => write!(f, "follower"),
        }
    }
}

/// Thread-safe, runtime-mutable node role for failover support.
///
/// Wraps an `AtomicU8` so the read-only middleware and health endpoint
/// always see the current role, even after a sentinel-triggered promotion.
#[derive(Clone)]
pub struct AtomicNodeRole(Arc<std::sync::atomic::AtomicU8>);

impl AtomicNodeRole {
    pub fn new(role: NodeRole) -> Self {
        Self(Arc::new(std::sync::atomic::AtomicU8::new(role.to_u8())))
    }

    pub fn load(&self) -> NodeRole {
        NodeRole::from_u8(self.0.load(std::sync::atomic::Ordering::Relaxed))
    }

    pub fn store(&self, role: NodeRole) {
        self.0
            .store(role.to_u8(), std::sync::atomic::Ordering::Relaxed);
    }
}

/// Unified application state for all handlers
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<EventStore>,
    pub auth_manager: Arc<AuthManager>,
    pub tenant_manager: Arc<TenantManager>,
    /// Service container for paywall domain use cases (Creator, Article, Payment, etc.)
    pub service_container: ServiceContainer,
    /// Node role for leader-follower replication (runtime-mutable for failover)
    pub role: AtomicNodeRole,
    /// WAL shipper for replication status reporting (leader only).
    /// Wrapped in RwLock so a promoted follower can install a shipper at runtime.
    pub wal_shipper: Arc<tokio::sync::RwLock<Option<Arc<WalShipper>>>>,
    /// WAL receiver for replication status reporting (follower only)
    pub wal_receiver: Option<Arc<WalReceiver>>,
    /// Replication port used by the WAL shipper (needed for runtime promotion)
    pub replication_port: u16,
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
    role: NodeRole,
    wal_shipper: Option<Arc<WalShipper>>,
    wal_receiver: Option<Arc<WalReceiver>>,
    replication_port: u16,
) -> anyhow::Result<()> {
    let app_state = AppState {
        store,
        auth_manager: auth_manager.clone(),
        tenant_manager,
        service_container,
        role: AtomicNodeRole::new(role),
        wal_shipper: Arc::new(tokio::sync::RwLock::new(wal_shipper)),
        wal_receiver,
        replication_port,
    };

    let auth_state = AuthState {
        auth_manager: auth_manager.clone(),
    };

    let rate_limit_state = RateLimitState { rate_limiter };

    let app = Router::new()
        // Public routes (no auth)
        .route("/health", get(health_v1))
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
        // Audit endpoints (admin only)
        .route("/api/v1/audit/events", post(log_audit_event))
        .route("/api/v1/audit/events", get(query_audit_events))
        // Config endpoints (admin only)
        .route("/api/v1/config", get(list_configs))
        .route("/api/v1/config", post(set_config))
        .route("/api/v1/config/{key}", get(get_config))
        .route("/api/v1/config/{key}", put(update_config))
        .route("/api/v1/config/{key}", delete(delete_config))
        // Event and data routes (protected by auth)
        .route("/api/v1/events", post(super::api::ingest_event_v1))
        .route(
            "/api/v1/events/batch",
            post(super::api::ingest_events_batch_v1),
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
        // Internal endpoints for sentinel-driven failover (not exposed publicly)
        .route("/internal/promote", post(promote_handler))
        .route("/internal/repoint", post(repoint_handler))
        .with_state(app_state.clone())
        // IMPORTANT: Middleware layers execute from bottom to top in Tower/Axum
        // Read-only middleware runs after auth (applied before rate limit layer)
        .layer(middleware::from_fn_with_state(
            app_state,
            read_only_middleware,
        ))
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

/// Write paths that should be rejected when running as a follower.
const WRITE_PATHS: &[&str] = &[
    "/api/v1/events",
    "/api/v1/events/batch",
    "/api/v1/snapshots",
    "/api/v1/projections/",
    "/api/v1/schemas",
    "/api/v1/replay",
    "/api/v1/pipelines",
    "/api/v1/compaction/trigger",
    "/api/v1/audit/events",
    "/api/v1/config",
];

/// Returns true if this request is a write operation that should be blocked on followers.
fn is_write_request(method: &axum::http::Method, path: &str) -> bool {
    use axum::http::Method;
    // Only POST/PUT/DELETE are writes
    if method != Method::POST && method != Method::PUT && method != Method::DELETE {
        return false;
    }
    WRITE_PATHS
        .iter()
        .any(|write_path| path.starts_with(write_path))
}

/// Returns true if the request targets an internal endpoint (not subject to read-only checks).
fn is_internal_request(path: &str) -> bool {
    path.starts_with("/internal/")
}

/// Middleware that rejects write requests when the node is a follower.
///
/// Returns HTTP 409 Conflict with `{"error": "read_only", "message": "..."}`.
/// Internal endpoints (`/internal/*`) are exempt — they are used by the sentinel
/// to trigger promotion and repointing during failover.
async fn read_only_middleware(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = request.uri().path();
    if state.role.load().is_follower()
        && is_write_request(request.method(), path)
        && !is_internal_request(path)
    {
        return (
            axum::http::StatusCode::CONFLICT,
            axum::Json(serde_json::json!({
                "error": "read_only",
                "message": "This node is a read-only follower"
            })),
        )
            .into_response();
    }
    next.run(request).await
}

/// Health endpoint with system stream health reporting.
///
/// Reports overall health plus detailed system metadata health when
/// event-sourced system repositories are configured.
async fn health_v1(State(state): State<AppState>) -> impl IntoResponse {
    let has_system_repos = state.service_container.has_system_repositories();

    let system_streams = if has_system_repos {
        let (tenant_count, config_count, total_events) =
            if let Some(store) = state.service_container.system_store() {
                use crate::domain::value_objects::system_stream::SystemDomain;
                (
                    store.count_stream(SystemDomain::Tenant),
                    store.count_stream(SystemDomain::Config),
                    store.total_events(),
                )
            } else {
                (0, 0, 0)
            };

        serde_json::json!({
            "status": "healthy",
            "mode": "event-sourced",
            "total_events": total_events,
            "tenant_events": tenant_count,
            "config_events": config_count,
        })
    } else {
        serde_json::json!({
            "status": "disabled",
            "mode": "in-memory",
        })
    };

    let replication = {
        let shipper_guard = state.wal_shipper.read().await;
        if let Some(ref shipper) = *shipper_guard {
            serde_json::to_value(shipper.status()).unwrap_or_default()
        } else if let Some(ref receiver) = state.wal_receiver {
            serde_json::to_value(receiver.status()).unwrap_or_default()
        } else {
            serde_json::json!(null)
        }
    };

    let current_role = state.role.load();

    Json(serde_json::json!({
        "status": "healthy",
        "service": "allsource-core",
        "version": env!("CARGO_PKG_VERSION"),
        "role": current_role,
        "system_streams": system_streams,
        "replication": replication,
    }))
}

/// POST /internal/promote — Promote this follower to leader.
///
/// Called by the sentinel process during automated failover.
/// Switches the node role to leader, stops WAL receiving, and starts
/// a WAL shipper on the replication port so other followers can connect.
async fn promote_handler(State(state): State<AppState>) -> impl IntoResponse {
    let current_role = state.role.load();
    if current_role == NodeRole::Leader {
        return (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "status": "already_leader",
                "message": "This node is already the leader",
            })),
        );
    }

    tracing::info!("PROMOTE: Switching role from follower to leader");

    // 1. Switch role — the read-only middleware will immediately start accepting writes
    state.role.store(NodeRole::Leader);

    // 2. Signal the WAL receiver to stop (it will stop reconnecting)
    if let Some(ref receiver) = state.wal_receiver {
        receiver.shutdown();
        tracing::info!("PROMOTE: WAL receiver shutdown signalled");
    }

    // 3. Start a new WAL shipper so remaining followers can connect
    let replication_port = state.replication_port;
    let (mut shipper, tx) = WalShipper::new();
    state.store.enable_wal_replication(tx);
    shipper.set_store(Arc::clone(&state.store));
    shipper.set_metrics(state.store.metrics());
    let shipper = Arc::new(shipper);

    // Install into AppState so health endpoint reports shipper status
    {
        let mut shipper_guard = state.wal_shipper.write().await;
        *shipper_guard = Some(Arc::clone(&shipper));
    }

    // Spawn the shipper server
    let shipper_clone = Arc::clone(&shipper);
    tokio::spawn(async move {
        if let Err(e) = shipper_clone.serve(replication_port).await {
            tracing::error!("Promoted WAL shipper error: {}", e);
        }
    });

    tracing::info!(
        "PROMOTE: Now accepting writes. WAL shipper listening on port {}",
        replication_port,
    );

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "status": "promoted",
            "role": "leader",
            "replication_port": replication_port,
        })),
    )
}

/// POST /internal/repoint?leader=host:port — Switch replication target.
///
/// Called by the sentinel process to tell a follower to disconnect from
/// the old leader and reconnect to a newly promoted leader.
async fn repoint_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let current_role = state.role.load();
    if current_role != NodeRole::Follower {
        return (
            axum::http::StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "not_follower",
                "message": "Repoint only applies to follower nodes",
            })),
        );
    }

    let new_leader = match params.get("leader") {
        Some(l) if !l.is_empty() => l.clone(),
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "missing_leader",
                    "message": "Query parameter 'leader' is required (e.g. ?leader=new-leader:3910)",
                })),
            );
        }
    };

    tracing::info!("REPOINT: Switching replication target to {}", new_leader);

    if let Some(ref receiver) = state.wal_receiver {
        receiver.repoint(&new_leader);
        tracing::info!("REPOINT: WAL receiver repointed to {}", new_leader);
    } else {
        tracing::warn!("REPOINT: No WAL receiver to repoint");
    }

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "status": "repointed",
            "new_leader": new_leader,
        })),
    )
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
