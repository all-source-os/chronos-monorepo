use std::path::Path;

use crate::{
    application::{
        add_dependency, approve_task, claim_task, complete_task, create_task, get_task, list_tasks,
        migrate_beads, remove_dependency, sync_git,
    },
    domain::{error::ChronError, repository::TaskRepository},
    infrastructure::{core_task_repo::CoreTaskRepository, workspace},
    presentation::{
        cli::{Command, DepCommands, TaskCommands},
        output::print_task_table,
    },
};

/// Collect task IDs for a cascade operation: the target plus all descendants.
/// Without cascade, returns just the target ID.
fn collect_cascade_ids(
    repo: &CoreTaskRepository,
    root_id: &str,
    cascade: bool,
) -> Result<Vec<String>, ChronError> {
    if !cascade {
        return Ok(vec![root_id.to_string()]);
    }
    let mut ids = Vec::new();
    let mut stack = vec![root_id.to_string()];
    while let Some(id) = stack.pop() {
        let children = repo.children_of(&id)?;
        for child in &children {
            stack.push(child.id.clone());
        }
        ids.push(id);
    }
    // Children first, parent last (bottom-up order)
    ids.reverse();
    Ok(ids)
}

pub fn dispatch_init() -> Result<(), ChronError> {
    let cwd = std::env::current_dir()?;
    workspace::init_workspace(&cwd)
}

pub async fn dispatch(
    cmd: &Command,
    repo: &CoreTaskRepository,
    workspace_root: &Path,
) -> Result<(), ChronError> {
    match cmd {
        Command::Init => unreachable!(),
        Command::Task(args) => match &args.subcommand {
            TaskCommands::Create(create_args) => {
                let output = create_task::create_task(
                    repo,
                    create_task::CreateTaskInput {
                        title: &create_args.title,
                        priority: &create_args.priority.to_string(),
                        blocked_by: &create_args.blocked_by,
                        task_type: create_args.task_type,
                        parent: create_args.parent.as_deref(),
                        description: create_args.description.as_deref(),
                    },
                )
                .await?;
                println!(
                    "Created {} {}: {}",
                    create_args.task_type, output.id, create_args.title
                );
            }
        },
        Command::List(args) => {
            let tasks = list_tasks::list_tasks(repo, args.status.as_deref())?;
            print_task_table(&tasks);
        }
        Command::Show(args) => {
            let detail = get_task::get_task(repo, &args.id).await?;
            let task = &detail.task;
            println!("Task: {}", task.id);
            println!("Type: {}", task.task_type);
            println!("Title: {}", task.title);
            if let Some(ref desc) = task.description {
                println!("Description: {desc}");
            }
            println!("Priority: {}", task.priority);
            println!("Status: {}", task.status);
            if let Some(ref parent) = task.parent {
                println!("Parent: {parent}");
            }
            if let Some(ref claimed) = task.claimed_by {
                println!("Claimed by: {claimed}");
            }
            if !task.blocked_by.is_empty() {
                println!("Blocked by: {}", task.blocked_by.join(", "));
            }
            if let Some(ref reason) = task.done_reason {
                println!("Done reason: {reason}");
            }
            // Show children
            let children = repo.children_of(&args.id)?;
            if !children.is_empty() {
                println!("\nChildren:");
                for child in &children {
                    println!("  {} [{}] {}", child.id, child.status, child.title);
                }
            }
            if !detail.timeline.is_empty() {
                println!("\nTimeline:");
                for entry in &detail.timeline {
                    println!("  {} — {}", entry.timestamp, entry.event_type);
                }
            }
        }
        Command::Ready => {
            let tasks = list_tasks::ready_tasks(repo)?;
            print_task_table(&tasks);
        }
        Command::Claim(args) => {
            let agent = crate::infrastructure::agent_id();
            let ids = collect_cascade_ids(repo, &args.id, args.cascade)?;
            for id in &ids {
                let task = repo.get_task(id)?;
                if task.status != crate::domain::task::TaskStatus::Open {
                    continue;
                }
                claim_task::claim_task(repo, id, &agent).await?;
                println!("Claimed task {id} (agent: {agent})");
            }
        }
        Command::Done(args) => {
            let ids = collect_cascade_ids(repo, &args.id, args.cascade)?;
            for id in &ids {
                let task = repo.get_task(id)?;
                if task.status == crate::domain::task::TaskStatus::Done {
                    continue;
                }
                complete_task::complete_task(repo, id, args.reason.as_deref()).await?;
                println!("Completed task {id}");
            }
        }
        Command::Approve(args) => {
            approve_task::approve_task(repo, &args.id).await?;
            println!("Approved task {}", args.id);
        }
        Command::Dep(args) => match &args.subcommand {
            DepCommands::Add(a) => {
                add_dependency::add_dependency(repo, &a.task_id, &a.blocker_id).await?;
                println!(
                    "Added dependency: {} blocked by {}",
                    a.task_id, a.blocker_id
                );
            }
            DepCommands::Remove(a) => {
                remove_dependency::remove_dependency(repo, &a.task_id, &a.blocker_id).await?;
                println!(
                    "Removed dependency: {} no longer blocked by {}",
                    a.task_id, a.blocker_id
                );
            }
        },
        Command::MigrateBeads(args) => {
            let result = migrate_beads::migrate_beads(repo, &args.beads_dir).await?;
            println!(
                "Migration complete: {} migrated, {} skipped (already exist)",
                result.migrated, result.skipped
            );
        }
        Command::Sync(args) => {
            if args.git {
                sync_git::sync_git(repo.core(), workspace_root).await?;
            } else {
                println!("Use --git for git-based sync. CRDT sync is not yet implemented.");
            }
        }
        Command::Tui | Command::Serve(_) => {
            unreachable!("handled in main.rs")
        }
    }
    Ok(())
}
