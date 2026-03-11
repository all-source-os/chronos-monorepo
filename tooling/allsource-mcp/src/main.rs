use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod protocol;
mod tools;
mod transport;

use transport::StdioTransport;

#[derive(Parser)]
#[command(
    name = "allsource-mcp",
    about = "MCP server for local AllSource debugging (stdio transport)"
)]
struct Cli {
    /// Path to AllSource data directory containing storage/ and wal/
    #[arg(long, env = "ALLSOURCE_DATA_DIR")]
    data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout is reserved for MCP JSON-RPC
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    tracing::info!("Opening AllSource data at {:?}", cli.data_dir);

    let core = allsource_core::embedded::EmbeddedCore::open(
        allsource_core::embedded::Config::builder()
            .data_dir(&cli.data_dir)
            .build()?,
    )
    .await?;

    tracing::info!("AllSource Core opened: {} events", core.event_count());

    let mut transport = StdioTransport::new(core);
    transport.run().await?;

    Ok(())
}
