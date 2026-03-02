use clap::{Parser, Subcommand};

use crate::domain::task::Priority;

#[derive(Parser)]
#[command(name = "cn", about = "Chronon — event-sourced task CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new .chronon workspace
    Init,

    /// Task management
    Task(TaskArgs),

    /// List tasks
    #[command(visible_alias = "ls")]
    List(ListArgs),

    /// Show task details and timeline
    #[command(visible_alias = "s")]
    Show(ShowArgs),

    /// Show tasks that are ready to work on (open + unblocked)
    #[command(visible_alias = "r")]
    Ready,

    /// Claim a task
    #[command(visible_alias = "c")]
    Claim(ClaimArgs),

    /// Mark a task as done
    #[command(visible_alias = "d")]
    Done(DoneArgs),

    /// Approve a task
    Approve(ApproveArgs),

    /// Launch interactive TUI dashboard
    Tui,

    /// Start embedded web viewer
    Serve(ServeArgs),
}

#[derive(clap::Args)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub subcommand: TaskCommands,
}

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Create a new task
    Create(CreateArgs),
}

#[derive(clap::Args)]
pub struct CreateArgs {
    /// Task title
    pub title: String,

    /// Priority (p0-p3)
    #[arg(short, long, default_value = "p2")]
    pub priority: Priority,

    /// Task IDs that block this task
    #[arg(long, value_delimiter = ',')]
    pub blocked_by: Vec<String>,
}

#[derive(clap::Args)]
pub struct ListArgs {
    /// Filter by status (open, in-progress, done)
    #[arg(short, long)]
    pub status: Option<String>,
}

#[derive(clap::Args)]
pub struct ShowArgs {
    /// Task ID
    pub id: String,
}

#[derive(clap::Args)]
pub struct ClaimArgs {
    /// Task ID
    pub id: String,
}

#[derive(clap::Args)]
pub struct DoneArgs {
    /// Task ID
    pub id: String,

    /// Reason or summary for completion
    #[arg(long)]
    pub reason: Option<String>,
}

#[derive(clap::Args)]
pub struct ApproveArgs {
    /// Task ID
    pub id: String,
}

#[derive(clap::Args)]
pub struct ServeArgs {
    /// Port to bind the web viewer
    #[arg(short, long, default_value = "3905")]
    pub port: u16,

    /// Auto-open browser after starting
    #[arg(long)]
    pub open: bool,
}
