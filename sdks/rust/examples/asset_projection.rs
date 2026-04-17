//! Asset projection worker example — mirrors the code in GitHub issue #155.
//!
//! Builds a `HashMap<Symbol, AssetState>` projection from three event types:
//! - `asset.registered` — registers a new asset
//! - `asset.updated` — patches altname / icon_url
//! - `exchange_mapping.set` — appends an exchange mapping
//!
//! # Running
//!
//! ```bash
//! # Against a local Docker Core at CORE_URL (default: http://localhost:3900):
//! cargo run -p allsource --example asset_projection --features projection-worker
//!
//! # Seed a few test events in another shell:
//! curl -X POST http://localhost:3900/api/v1/events \
//!   -H 'Content-Type: application/json' \
//!   -d '{"event_type":"asset.registered","entity_id":"BTC","payload":{"symbol":"BTC","altname":"Bitcoin"}}'
//! ```
//!
//! The worker will print state on every catch-up and then idle in live mode.
//! Ctrl+C to stop.

use std::{collections::HashMap, time::Duration};

use allsource::{CoreClient, Error, Event, ProjectionWorker};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct AssetState {
    symbol: String,
    altname: String,
    icon_url: Option<String>,
    exchange_mappings: Vec<ExchangeMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExchangeMapping {
    exchange: String,
    exchange_symbol: String,
}

#[derive(Deserialize)]
struct AssetRegistered {
    symbol: String,
    #[serde(default)]
    altname: String,
    #[serde(default)]
    icon_url: Option<String>,
}

#[derive(Deserialize)]
struct ExchangeMappingSet {
    symbol: String,
    exchange: String,
    exchange_symbol: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,allsource=debug".into()),
        )
        .init();

    let core_url = std::env::var("CORE_URL").unwrap_or_else(|_| "http://localhost:3900".into());
    let api_key = std::env::var("ALLSOURCE_API_KEY").unwrap_or_else(|_| "dev".into());
    let core = CoreClient::new(&core_url, &api_key)?;

    let worker = ProjectionWorker::<HashMap<String, AssetState>>::builder(core)
        .name("assets")
        .event_types(&["asset.registered", "asset.updated", "exchange_mapping.set"])
        .reducer(asset_reducer)
        .checkpoint_interval(100)
        .state_flush_every(50)
        .state_flush_entities(|state| {
            state
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::to_value(v).unwrap_or_default()))
                .collect()
        })
        .build()?;

    let handle = worker.start().await?;

    tracing::info!("worker started; waiting for replay to complete");
    while !handle.is_caught_up() {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    tracing::info!(position = handle.current_position(), "caught up");

    // Read the reduced state — the in-memory map survives as long as the worker runs.
    if let Some(btc) = handle.state().read().await.get("BTC") {
        println!("BTC → {btc:?}");
    } else {
        println!("no BTC asset registered yet");
    }

    // Idle; let the user Ctrl+C to stop.
    tokio::signal::ctrl_c().await.ok();
    tracing::info!("shutting down");
    handle.stop().await?;
    Ok(())
}

fn asset_reducer(state: &mut HashMap<String, AssetState>, event: &Event) -> Result<(), Error> {
    match event.event_type.as_str() {
        "asset.registered" => {
            let payload: AssetRegistered = serde_json::from_value(event.payload.clone())?;
            state.insert(
                payload.symbol.clone(),
                AssetState {
                    symbol: payload.symbol,
                    altname: payload.altname,
                    icon_url: payload.icon_url,
                    exchange_mappings: vec![],
                },
            );
        }
        "asset.updated" => {
            if let Ok(payload) = serde_json::from_value::<AssetRegistered>(event.payload.clone()) {
                if let Some(asset) = state.get_mut(&payload.symbol) {
                    asset.altname = payload.altname;
                    asset.icon_url = payload.icon_url;
                }
            }
        }
        "exchange_mapping.set" => {
            let payload: ExchangeMappingSet = serde_json::from_value(event.payload.clone())?;
            if let Some(asset) = state.get_mut(&payload.symbol) {
                asset.exchange_mappings.push(ExchangeMapping {
                    exchange: payload.exchange,
                    exchange_symbol: payload.exchange_symbol,
                });
            }
        }
        _ => {}
    }
    Ok(())
}
