// mimalloc chosen over jemallocator for Docker build simplicity — jemalloc's
// autoconf-based C build needs more tooling than the slim runtime image has.
// For this workload (stdio-dominated + occasional HTTP) the allocator choice
// is secondary; mimalloc is fine for long-running servers and builds cleanly.
#[cfg(not(feature = "dhat-heap"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod http;
mod profiling;
mod projection_registry;
mod protocol;
mod sync;
mod templates;
mod tools;
mod toon;
mod transport;

use transport::StdioTransport;

#[derive(Clone, Debug, clap::ValueEnum)]
enum Mode {
    /// MCP server over stdio (newline-delimited JSON-RPC, per the MCP spec)
    Mcp,
    /// HTTP REST API server
    Http,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum Format {
    /// Pretty-printed JSON (default — maximally compatible)
    Json,
    /// Token-Oriented Object Notation — compact, ~40-60% fewer tokens on
    /// uniform result arrays. Encodes the tool-result payload only; the
    /// JSON-RPC envelope is always standard JSON.
    Toon,
}

#[derive(Parser)]
#[command(
    name = "allsource-prime",
    about = "AllSource Prime — unified agent memory engine. Supports MCP (stdio) and HTTP modes.",
    version
)]
struct Cli {
    /// Path to Prime data directory (WAL, Parquet, projection checkpoints)
    #[arg(long, env = "PRIME_DATA_DIR")]
    data_dir: PathBuf,

    /// Server mode: mcp (stdio) or http
    #[arg(long, env = "PRIME_MODE", default_value = "mcp")]
    mode: Mode,

    /// HTTP port (only used in http mode)
    #[arg(long, env = "PRIME_PORT", default_value = "3905")]
    port: u16,

    /// Tool-result payload encoding: json or toon (mcp mode only)
    #[arg(long, env = "PRIME_RESULT_FORMAT", default_value = "json")]
    format: Format,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "PRIME_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Auto-inject compressed index as MCP resource (zer0dex-style pre-message injection)
    #[arg(long, env = "PRIME_AUTO_INJECT")]
    auto_inject: bool,

    /// Max tokens for auto-injected index (default: 1000)
    #[arg(long, env = "PRIME_AUTO_INJECT_MAX_TOKENS", default_value = "1000")]
    auto_inject_max_tokens: usize,

    /// Remote `AllSource` Core base URL to push `prime.*` events to.
    /// When set with `--api-key`, spawns a background sync loop that ships
    /// events to your tenant so the web panel's Memory tab can show them.
    #[arg(long, env = "PRIME_SYNC_TO")]
    sync_to: Option<String>,

    /// Tenant API key for sync. Required if `--sync-to` is set.
    #[arg(long, env = "PRIME_API_KEY")]
    api_key: Option<String>,

    /// Sync flush interval in milliseconds (default: 1000ms).
    #[arg(long, env = "PRIME_SYNC_INTERVAL_MS", default_value = "1000")]
    sync_interval_ms: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Log to stderr so stdout is reserved for MCP JSON-RPC
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cli.log_level)),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Opening Prime at {:?}", cli.data_dir);

    let prime = Arc::new(allsource_core::prime::Prime::open(&cli.data_dir).await?);

    tracing::info!(
        "Prime ready — {} nodes, {} edges",
        prime.stats().total_nodes,
        prime.stats().total_edges
    );

    let recall_config = allsource_core::prime::recall::IndexConfig::default();
    let recall =
        allsource_core::prime::recall::RecallEngine::with_deps(prime.recall_deps(), &recall_config);

    tools::set_result_format(match cli.format {
        Format::Json => tools::ResultFormat::Json,
        Format::Toon => tools::ResultFormat::Toon,
    });

    // Hydrate the projection registry cache from the durable event log.
    // The event log (prime.projection.defined events) is the source of
    // truth; this just primes the in-memory accelerator. Failure is
    // tolerable — agents can re-register projections — but a warn is
    // logged so the operator notices.
    match prime.load_projection_defs().await {
        Ok(defs) => {
            let count = defs.len();
            projection_registry::hydrate(defs);
            if count > 0 {
                tracing::info!(
                    "Hydrated {count} projection definition(s) from event log"
                );
            }
        }
        Err(e) => tracing::warn!(
            error = %e,
            "failed to hydrate projection registry from event log — agents will need to re-register"
        ),
    }

    // Spawn the push-only sync loop when both --sync-to and --api-key are set.
    // Mismatched flags are a user error worth surfacing rather than silently
    // dropping sync.
    //
    // Treat blank strings as "not set". The Claude Desktop DXT manifest always
    // passes `--sync-to ${user_config.sync_to}` / `--api-key ${...}`, so a user
    // who clears the sync URL in the extension settings hands us `--sync-to ""`
    // rather than omitting the flag. Without this guard that empty string would
    // be taken as a real remote and every push would fail against `/api/v1/events`.
    let sync_to = cli.sync_to.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let api_key = cli.api_key.as_deref().map(str::trim).filter(|s| !s.is_empty());
    match (sync_to, api_key) {
        (Some(url), Some(key)) => {
            tracing::info!(
                remote_url = %url,
                "Prime sync ENABLED — prime.* events will be shipped to your AllSource tenant \
                 and appear on the dashboard Memory tab (all-source.xyz/dashboard/memory)."
            );
            tools::set_sync_status(tools::SyncStatus {
                enabled: true,
                remote_url: Some(url.to_string()),
            });
            let sync_config = sync::SyncConfig {
                remote_url: url.to_string(),
                api_key: key.to_string(),
                interval: std::time::Duration::from_millis(cli.sync_interval_ms),
            };
            let sync_prime = Arc::clone(&prime);
            let data_dir = cli.data_dir.clone();
            tokio::spawn(sync::run_sync_loop(sync_prime, sync_config, data_dir));
        }
        (Some(_), None) | (None, Some(_)) => {
            anyhow::bail!(
                "--sync-to and --api-key must be supplied together (or neither). \
                 Get a tenant API key at https://www.all-source.xyz/connect."
            );
        }
        (None, None) => {
            tools::set_sync_status(tools::SyncStatus {
                enabled: false,
                remote_url: None,
            });
            // Loud, unmissable: a silent local-only mode is the exact failure
            // that makes the dashboard show "No memory yet" while writes look
            // like they're succeeding. Surface it at WARN on every startup.
            tracing::warn!(
                "Prime running LOCAL-ONLY — writes will NOT appear in your AllSource dashboard \
                 (all-source.xyz/dashboard/memory). Pass --sync-to https://api.all-source.xyz \
                 and --api-key <your tenant key> to sync. Get a key at \
                 https://www.all-source.xyz/connect. Call prime_stats to confirm sync state."
            );
        }
    }

    match cli.mode {
        Mode::Mcp => {
            tracing::info!(
                "Starting MCP server (stdio transport, {:?} payloads)",
                cli.format
            );
            let mut transport = StdioTransport::new(prime, recall);
            if cli.auto_inject {
                tracing::info!(
                    "Auto-inject enabled (max {} tokens)",
                    cli.auto_inject_max_tokens
                );
                transport = transport.with_auto_inject(cli.auto_inject_max_tokens);
            }
            transport.run().await?;
        }
        Mode::Http => {
            tracing::info!("Starting HTTP server on port {}", cli.port);
            let state = Arc::new(http::AppState { prime, recall });
            http::serve(state, cli.port).await?;
        }
    }

    Ok(())
}
