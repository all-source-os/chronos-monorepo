use chronis::{
    infrastructure::workspace::Workspace,
    presentation::{
        cli::{Cli, Command},
        dispatch,
    },
};
use clap::Parser;
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
            chronis::presentation::tui::run(repo).await?;
            // repo is moved into tui::run, no need to drop
            ws.shutdown().await?;
        }
        Command::Serve(args) => {
            let ws = Workspace::open().await?;
            let repo = ws.repo();
            chronis::presentation::web::run(repo, args.port, args.open).await?;
            ws.shutdown().await?;
        }
        _ => {
            let ws = Workspace::open().await?;
            let repo = ws.repo();
            let result = dispatch::dispatch(&cli.command, &repo, &ws.root, cli.toon).await;
            drop(repo);
            ws.shutdown().await?;
            result?;
        }
    }

    Ok(())
}
