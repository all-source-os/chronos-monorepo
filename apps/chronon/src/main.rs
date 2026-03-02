use clap::Parser;
use chronon::infrastructure::workspace::Workspace;
use chronon::presentation::cli::{Cli, Command};
use chronon::presentation::dispatch;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match &cli.command {
        Command::Init => {
            dispatch::dispatch_init()?;
        }
        Command::Tui => {
            let ws = Workspace::open().await?;
            let repo = ws.repo();
            chronon::presentation::tui::run(repo).await?;
            // repo is moved into tui::run, no need to drop
            ws.shutdown().await?;
        }
        Command::Serve(args) => {
            let ws = Workspace::open().await?;
            let repo = ws.repo();
            chronon::presentation::web::run(repo, args.port, args.open).await?;
            ws.shutdown().await?;
        }
        _ => {
            let ws = Workspace::open().await?;
            let repo = ws.repo();
            let result = dispatch::dispatch(&cli.command, &repo).await;
            drop(repo);
            ws.shutdown().await?;
            result?;
        }
    }

    Ok(())
}
