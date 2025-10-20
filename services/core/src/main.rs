mod analytics;
mod compaction;
mod error;
mod event;
mod store;
mod api;
mod index;
mod projection;
mod snapshot;
mod storage;
mod wal;
mod websocket;

use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::store::EventStore;

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

    tracing::info!("🌟 AllSource Core starting...");
    tracing::info!("   Version: {}", env!("CARGO_PKG_VERSION"));
    tracing::info!("   High-performance event store with columnar storage");

    // Initialize event store
    let store = Arc::new(EventStore::new());

    // Start API server
    let addr = "0.0.0.0:8080";
    tracing::info!("🚀 AllSource Core listening on {}", addr);

    api::serve(store, addr).await?;

    Ok(())
}
