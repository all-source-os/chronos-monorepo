use clap::{Parser, Subcommand};

use crate::domain::task::{Priority, TaskType};

#[derive(Parser)]
#[command(name = "cn", about = "Chronis — event-sourced task CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new .chronis workspace
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

    /// Manage task dependencies
    Dep(DepArgs),

    /// Migrate issues from .beads/ to chronis
    MigrateBeads(MigrateBeadsArgs),

    /// Sync chronis data
    Sync(SyncArgs),

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

    /// Task type (task, epic, bug, feature)
    #[arg(short = 't', long = "type", default_value = "task")]
    pub task_type: TaskType,

    /// Parent task ID (for hierarchy)
    #[arg(long)]
    pub parent: Option<String>,

    /// Description
    #[arg(short = 'd', long)]
    pub description: Option<String>,
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
pub struct DepArgs {
    #[command(subcommand)]
    pub subcommand: DepCommands,
}

#[derive(Subcommand)]
pub enum DepCommands {
    /// Add a dependency (blocker) to a task
    Add(DepAddArgs),
    /// Remove a dependency from a task
    Remove(DepRemoveArgs),
}

#[derive(clap::Args)]
pub struct DepAddArgs {
    /// Task ID that is blocked
    pub task_id: String,
    /// Task ID that blocks it
    pub blocker_id: String,
}

#[derive(clap::Args)]
pub struct DepRemoveArgs {
    /// Task ID to remove dependency from
    pub task_id: String,
    /// Blocker task ID to remove
    pub blocker_id: String,
}

#[derive(clap::Args)]
pub struct MigrateBeadsArgs {
    /// Path to .beads/ directory (default: .beads/ in current directory)
    #[arg(long, default_value = ".beads")]
    pub beads_dir: String,
}

#[derive(clap::Args)]
pub struct SyncArgs {
    /// Sync via git (add .chronis/, commit, push)
    #[arg(long)]
    pub git: bool,
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
