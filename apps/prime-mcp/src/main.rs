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
mod protocol;
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

    let prime = allsource_core::prime::Prime::open(&cli.data_dir).await?;

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
