mod analytics;
mod auth;
mod auth_api;
mod api;
mod api_v1;
mod backup;
mod compaction;
mod config;
mod error;
mod event;
mod store;
mod index;
mod metrics;
mod middleware;
mod pipeline;
mod projection;
mod rate_limit;
mod replay;
mod schema;
mod snapshot;
mod storage;
mod tenant;
mod tenant_api;
mod wal;
mod websocket;

use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::auth::AuthManager;
use crate::rate_limit::{RateLimiter, RateLimitConfig};
use crate::store::EventStore;
use crate::tenant::TenantManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "allsource_core=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🌟 AllSource Core v{} starting...", env!("CARGO_PKG_VERSION"));
    tracing::info!("   Production-ready event store with authentication & multi-tenancy");

    // Initialize components
    let store = Arc::new(EventStore::new());
    let auth_manager = Arc::new(AuthManager::default());
    let tenant_manager = Arc::new(TenantManager::new());
    let rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig::professional()));

    tracing::info!("✅ Event store initialized");
    tracing::info!("✅ Authentication manager initialized");
    tracing::info!("✅ Tenant manager initialized (default tenant created)");
    tracing::info!("✅ Rate limiter initialized (professional tier defaults)");

    // Start API server (v1.0 with auth & rate limiting)
    let addr = "0.0.0.0:8080";
    tracing::info!("🚀 AllSource Core listening on {}", addr);
    tracing::info!("📝 API Documentation: /health for health check");
    tracing::info!("🔒 Features: Auth, Multi-tenancy, Rate Limiting");

    api_v1::serve_v1(store, auth_manager, tenant_manager, rate_limiter, addr).await?;

    Ok(())
}
