use allsource_core::{
    api_v1,
    api_v1::NodeRole,
    auth::AuthManager,
    config::ServerConfig,
    infrastructure::{di::ContainerBuilder, persistence::SystemBootstrap},
    rate_limit::{RateLimitConfig, RateLimiter},
    replication::{ReplicationMode, WalReceiver, WalShipper},
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

    // Detect node role from environment
    let role = NodeRole::from_env();

    tracing::info!(
        "🌟 AllSource Core v{} starting...",
        env!("CARGO_PKG_VERSION")
    );
    match role {
        NodeRole::Leader => tracing::info!("   Starting as LEADER"),
        NodeRole::Follower => tracing::info!("   Starting as FOLLOWER (read-only)"),
    }
    tracing::info!("   Production-ready event store with authentication & multi-tenancy");

    // Initialize components
    let store = EventStore::new();

    // Initialize WAL replication if this is a leader with replication enabled
    let replication_enabled = std::env::var("ALLSOURCE_REPLICATION_ENABLED")
        .map(|v| v == "true")
        .unwrap_or(false);
    let replication_port: u16 = std::env::var("ALLSOURCE_REPLICATION_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3910);

    // Read replication mode and ACK timeout
    let replication_mode = std::env::var("ALLSOURCE_REPLICATION_MODE")
        .map(|v| ReplicationMode::from_str_value(&v))
        .unwrap_or(ReplicationMode::Async);
    let ack_timeout_ms: u64 = std::env::var("ALLSOURCE_REPLICATION_ACK_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);

    // Create WAL shipper (but don't spawn yet — needs store Arc first for catch-up)
    let wal_shipper_raw = if replication_enabled && role == NodeRole::Leader {
        let (mut shipper, tx) = WalShipper::new();
        shipper.set_replication_mode(
            replication_mode,
            std::time::Duration::from_millis(ack_timeout_ms),
        );
        store.enable_wal_replication(tx);
        Some(shipper)
    } else {
        if replication_enabled && role == NodeRole::Follower {
            tracing::info!("   Replication enabled but this is a follower — shipper not started");
        }
        None
    };

    let store = Arc::new(store);

    // Now that store is in Arc, attach it to the shipper and spawn
    let wal_shipper = if let Some(mut shipper) = wal_shipper_raw {
        shipper.set_store(Arc::clone(&store));
        shipper.set_metrics(store.metrics());
        let shipper = Arc::new(shipper);
        let shipper_clone = Arc::clone(&shipper);
        tokio::spawn(async move {
            if let Err(e) = shipper_clone.serve(replication_port).await {
                tracing::error!("Replication server error: {}", e);
            }
        });
        tracing::info!(
            "✅ WAL replication enabled on port {} (mode: {}, ack_timeout: {}ms)",
            replication_port,
            replication_mode,
            ack_timeout_ms,
        );
        Some(shipper)
    } else {
        None
    };

    // Initialize WAL receiver if this is a follower with a leader URL configured
    let wal_receiver = if role == NodeRole::Follower {
        if let Ok(leader_url) = std::env::var("ALLSOURCE_LEADER_URL")
            && !leader_url.is_empty()
        {
            let follower_wal_dir = std::env::var("ALLSOURCE_DATA_DIR")
                .unwrap_or_else(|_| "/tmp/allsource".to_string());
            let wal_dir = std::path::PathBuf::from(&follower_wal_dir).join("follower-wal");

            match WalReceiver::new(leader_url.clone(), &wal_dir, Arc::clone(&store)) {
                Ok(mut receiver) => {
                    receiver.set_metrics(store.metrics());
                    let receiver = Arc::new(receiver);
                    let receiver_clone = Arc::clone(&receiver);
                    tokio::spawn(async move {
                        receiver_clone.run().await;
                    });
                    tracing::info!(
                        "✅ WAL receiver started, connecting to leader at {}",
                        leader_url,
                    );
                    Some(receiver)
                }
                Err(e) => {
                    tracing::error!("Failed to initialize WAL receiver: {}", e);
                    None
                }
            }
        } else {
            tracing::warn!("Follower mode but ALLSOURCE_LEADER_URL not set — replication disabled");
            None
        }
    } else {
        None
    };

    let auth_manager = Arc::new(AuthManager::default());
    let tenant_manager = Arc::new(TenantManager::new());
    let rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig::professional()));

    // Initialize system metadata (event-sourced repositories)
    // ALLSOURCE_SYSTEM_DATA_DIR: path to durable system metadata storage
    //   (defaults to {ALLSOURCE_DATA_DIR}/__system/ if not set)
    // ALLSOURCE_BOOTSTRAP_TENANT: name of default tenant to create on first boot
    let system_data_dir = std::env::var("ALLSOURCE_SYSTEM_DATA_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("ALLSOURCE_DATA_DIR")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|d| std::path::PathBuf::from(d).join("__system"))
        });
    let bootstrap_tenant = std::env::var("ALLSOURCE_BOOTSTRAP_TENANT")
        .ok()
        .filter(|s| !s.is_empty());
    let system_repos = SystemBootstrap::try_initialize(system_data_dir, bootstrap_tenant).await;

    // Initialize DI container for paywall domain + system repositories
    let mut builder = ContainerBuilder::new().with_in_memory_repositories();
    if let Some(repos) = system_repos {
        builder = builder.with_system_repositories(repos);
    }
    let service_container = builder.build();

    tracing::info!("✅ Event store initialized");
    tracing::info!("✅ Authentication manager initialized");
    tracing::info!("✅ Tenant manager initialized (default tenant created)");
    tracing::info!("✅ Rate limiter initialized (professional tier defaults)");
    if service_container.has_system_repositories() {
        tracing::info!("✅ Service container initialized (event-sourced system repositories)");
    } else {
        tracing::info!("✅ Service container initialized (in-memory repositories)");
    }

    // Register bootstrap API key if configured
    if let Ok(bootstrap_key) = std::env::var("ALLSOURCE_BOOTSTRAP_API_KEY")
        && !bootstrap_key.is_empty()
    {
        auth_manager.register_bootstrap_api_key(&bootstrap_key, "default");
        tracing::info!("✅ Bootstrap API key configured");
    }

    // Start API server (v1.0 with auth & rate limiting)
    let config = ServerConfig::default();
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("🚀 AllSource Core listening on {}", addr);
    tracing::info!("📝 API: /health, /api/v1/events, /api/v1/events/query");
    tracing::info!(
        "🔌 WebSocket: ws://{}:{}/api/v1/events/stream",
        config.host,
        config.port
    );
    tracing::info!("🔒 Features: Auth, Multi-tenancy, Rate Limiting");

    api_v1::serve_v1(
        store,
        auth_manager,
        tenant_manager,
        rate_limiter,
        service_container,
        &addr,
        role,
        wal_shipper,
        wal_receiver,
        replication_port,
    )
    .await?;

    Ok(())
}
