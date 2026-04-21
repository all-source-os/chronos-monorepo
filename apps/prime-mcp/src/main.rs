#[cfg(all(not(target_env = "msvc"), not(feature = "dhat-heap")))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

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
mod transport;

use transport::StdioTransport;

#[derive(Clone, Debug, clap::ValueEnum)]
enum Mode {
    /// MCP server over stdio (JSON-RPC with Content-Length framing)
    Mcp,
    /// HTTP REST API server
    Http,
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

    match cli.mode {
        Mode::Mcp => {
            tracing::info!("Starting MCP server (stdio transport)");
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
