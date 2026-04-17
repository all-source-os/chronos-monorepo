use clap::{Parser, Subcommand};

use crate::domain::task::{Priority, TaskType};

#[derive(Parser)]
#[command(
    name = "cn",
    about = "Chronis — the agent-native task CLI",
    long_about = "Chronis — the agent-native task CLI. Event-sourced, TOON-optimized.\n\n\
        Inspired by beads_rust. Every action is an immutable event.\n\
        State is derived from projections over the event stream.\n\n\
        TOON OUTPUT (the key differentiator):\n  \
          Pass --toon to ANY command for TOON (Token-Oriented Object Notation).\n  \
          ~50% fewer tokens than JSON — pipe-delimited, no braces, no quotes.\n  \
          Lists:    [id|type|title|pri|status|...]\\n val|val|val\n  \
          Details:  key:value lines\n  \
          Actions:  ok:verb:id (single-line ack)\n\n\
        AGENT WORKFLOW (4 commands, ~20 tokens overhead):\n  \
          cn ready --toon          → pick a task from TOON table\n  \
          cn claim <id>  --toon    → ok:claimed:<id>\n  \
          cn done <id>   --toon    → ok:done:<id>\n  \
          cn list --toon           → verify final state\n\n\
        QUICK REFERENCE:\n  \
          cn task create <title>   Create task (-p priority, -t type, --parent, --blocked-by, -d desc)\n  \
          cn list [--status=S]     List tasks (--archived, --all, --toon)\n  \
          cn ready                 Open + unblocked tasks\n  \
          cn show <id>             Task detail + event timeline\n  \
          cn claim <id>            Claim task (--cascade for epics)\n  \
          cn done <id>             Complete task (--reason, --cascade)\n  \
          cn approve <id>          Approve task\n  \
          cn archive [ids..]       Archive tasks (--all-done, --done-before <days>)\n  \
          cn unarchive [ids..]     Restore archived tasks\n  \
          cn dep add|remove        Manage blockers\n  \
          cn sync                  Sync events with remote Core\n  \
          cn tui                   Interactive TUI\n  \
          cn serve                 Web viewer\n\n\
        MIGRATE FROM BEADS:\n  \
          cn migrate-beads         Import .beads/issues.jsonl → chronis events"
)]
pub struct Cli {
    /// Output TOON format (Token-Oriented Object Notation) for agents/scripts — ~50% fewer tokens
    #[arg(long, global = true)]
    pub toon: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new .chronis workspace
    Init(InitArgs),

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

    /// Archive tasks (hide from default listings)
    Archive(ArchiveArgs),

    /// Unarchive a task
    Unarchive(UnarchiveArgs),

    /// Manage task dependencies
    Dep(DepArgs),

    /// Migrate issues from .beads/ to chronis
    MigrateBeads(MigrateBeadsArgs),

    /// Sync events with a remote Core (embedded mode only)
    Sync,

    /// Launch interactive TUI dashboard
    Tui,

    /// Start embedded web viewer
    Serve(ServeArgs),
}

#[derive(clap::Args)]
pub struct InitArgs {
    /// Remote AllSource URL for team sync (e.g. https://api.all-source.xyz)
    #[arg(long)]
    pub remote: Option<String>,

    /// API key for authenticating with AllSource
    #[arg(long)]
    pub api_key: Option<String>,
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

    /// Filter by type (task, epic, bug, feature)
    #[arg(short = 't', long = "type")]
    pub task_type: Option<String>,

    /// Show only archived tasks
    #[arg(long)]
    pub archived: bool,

    /// Show all tasks (including archived)
    #[arg(long)]
    pub all: bool,
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

    /// Also claim all children (for epics)
    #[arg(long)]
    pub cascade: bool,
}

#[derive(clap::Args)]
pub struct DoneArgs {
    /// Task ID
    pub id: String,

    /// Reason or summary for completion
    #[arg(long)]
    pub reason: Option<String>,

    /// Also mark all children as done (for epics)
    #[arg(long)]
    pub cascade: bool,
}

#[derive(clap::Args)]
pub struct ApproveArgs {
    /// Task ID
    pub id: String,
}

#[derive(clap::Args)]
pub struct ArchiveArgs {
    /// Task IDs to archive
    pub ids: Vec<String>,

    /// Archive all done tasks
    #[arg(long)]
    pub all_done: bool,

    /// Archive tasks done before N days ago (e.g., 30)
    #[arg(long, value_name = "DAYS")]
    pub done_before: Option<u64>,
}

#[derive(clap::Args)]
pub struct UnarchiveArgs {
    /// Task IDs to unarchive
    pub ids: Vec<String>,
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
pub struct ServeArgs {
    /// Port to bind the web viewer
    #[arg(short, long, default_value = "3905")]
    pub port: u16,

    /// Auto-open browser after starting
    #[arg(long)]
    pub open: bool,
}
