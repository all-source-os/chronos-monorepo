use allsource_core::{
    api_v1,
    auth::AuthManager,
    config::ServerConfig,
    rate_limit::{RateLimitConfig, RateLimiter},
    store::EventStore,
    tenant::TenantManager,
};
use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    tracing::info!(
        "🌟 AllSource Core v{} starting...",
        env!("CARGO_PKG_VERSION")
    );
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
    let config = ServerConfig::default();
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("🚀 AllSource Core listening on {}", addr);
    tracing::info!("📝 API Documentation: /health for health check");
    tracing::info!("🔒 Features: Auth, Multi-tenancy, Rate Limiting");

    api_v1::serve_v1(store, auth_manager, tenant_manager, rate_limiter, &addr).await?;

    Ok(())
}
