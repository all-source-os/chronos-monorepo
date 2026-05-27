use std::path::Path;

use chrono::Utc;

use crate::{
    application::{
        add_dependency, approve_task, archive_task, claim_task, complete_task, create_task,
        get_task, list_tasks, migrate_beads, remove_dependency, sync_http,
    },
    domain::{error::ChronError, repository::TaskRepository},
    infrastructure::{
        config::{ChronisConfig, CoreMode},
        core_task_repo::CoreTaskRepository,
        prime_setup, workspace,
    },
    presentation::{
        cli::{ArchiveArgs, Command, DepCommands, PrimeArgs, PrimeCommands, TaskCommands},
        output::print_task_table,
        toon,
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

/// Resolve which task IDs to archive based on CLI args.
fn resolve_archive_ids(
    repo: &CoreTaskRepository,
    args: &ArchiveArgs,
) -> Result<Vec<String>, ChronError> {
    use crate::domain::task::TaskStatus;

    if !args.ids.is_empty() {
        return Ok(args.ids.clone());
    }

    // Get all tasks including non-archived ones via list_tasks_all
    let all = repo.list_tasks_all(None)?;

    if args.all_done {
        return Ok(all
            .into_iter()
            .filter(|t| t.status == TaskStatus::Done && !t.archived)
            .map(|t| t.id)
            .collect());
    }

    if let Some(days) = args.done_before {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        return Ok(all
            .into_iter()
            .filter(|t| {
                t.status == TaskStatus::Done
                    && !t.archived
                    && t.done_at.is_some_and(|done_at| done_at < cutoff)
            })
            .map(|t| t.id)
            .collect());
    }

    Err(ChronError::Sync(
        "specify task IDs, --all-done, or --done-before".to_string(),
    ))
}

pub fn dispatch_init(args: &super::cli::InitArgs) -> Result<(), ChronError> {
    let cwd = std::env::current_dir()?;
    if let Some(ref remote_url) = args.remote {
        workspace::init_workspace_with_remote(&cwd, remote_url, args.api_key.as_deref())
    } else {
        workspace::init_workspace(&cwd)
    }
}

/// Wire Prime MCP into Claude Code for the current project.
///
/// Runs outside the `Workspace::open` path because (a) it can operate on
/// projects that haven't run `cn init` yet, and (b) it doesn't need the
/// embedded chronis event store — Prime has its own data dir.
pub fn dispatch_prime(args: &PrimeArgs, toon_mode: bool) -> Result<(), ChronError> {
    let cwd = std::env::current_dir()?;
    match &args.subcommand {
        PrimeCommands::Setup(setup_args) => {
            let report = prime_setup::run_prime_setup(
                &cwd,
                setup_args.bin.as_deref(),
                setup_args.data_dir.as_deref(),
            )?;
            if toon_mode {
                println!(
                    "ok:prime.setup:{}:{}",
                    report.mcp_json_path.display(),
                    if report.replaced_existing { "replaced" } else { "added" }
                );
            } else {
                let verb = if report.replaced_existing { "Updated" } else { "Added" };
                println!("Prime MCP wired into Claude Code.");
                println!("  Binary:       {} ({})", report.bin_path, report.bin_version);
                println!("  Data dir:     {}", report.data_dir.display());
                println!(
                    "  {} `prime` entry in {}",
                    verb,
                    report.mcp_json_path.display()
                );
                println!();
                println!(
                    "Next: open a NEW Claude Code session in this directory. \
                     `mcp__prime__*` tools will load on session start. \
                     Try `prime_add_node` then `prime_recall` to confirm."
                );
            }
            Ok(())
        }
    }
}

#[cfg_attr(feature = "hotpath", hotpath::measure)]
pub async fn dispatch(
    cmd: &Command,
    repo: &CoreTaskRepository,
    workspace_root: &Path,
    config: &ChronisConfig,
    toon_mode: bool,
) -> Result<(), ChronError> {
    match cmd {
        Command::Init(_) => unreachable!(),
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
                if toon_mode {
                    print!(
                        "{}",
                        toon::created(
                            &create_args.task_type.to_string(),
                            &output.id,
                            &create_args.title
                        )
                    );
                } else {
                    println!(
                        "Created {} {}: {}",
                        create_args.task_type, output.id, create_args.title
                    );
                }
            }
        },
        Command::List(args) => {
            let mut tasks = if args.archived {
                repo.list_tasks_archived()?
            } else if args.all {
                repo.list_tasks_all(args.status.as_deref())?
            } else {
                list_tasks::list_tasks(repo, args.status.as_deref())?
            };
            if let Some(ref type_filter) = args.task_type {
                let tt: crate::domain::task::TaskType = type_filter.parse()?;
                tasks.retain(|t| t.task_type == tt);
            }
            if toon_mode {
                print!("{}", toon::tasks(&tasks));
            } else {
                print_task_table(&tasks);
            }
        }
        Command::Show(args) => {
            let detail = get_task::get_task(repo, &args.id).await?;
            if toon_mode {
                print!("{}", toon::task_detail(&detail));
                let children = repo.children_of(&args.id)?;
                print!("{}", toon::children(&children));
                print!("{}", toon::timeline(&detail.timeline));
            } else {
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
        }
        Command::Ready => {
            let tasks = list_tasks::ready_tasks(repo)?;
            if toon_mode {
                print!("{}", toon::tasks(&tasks));
            } else {
                print_task_table(&tasks);
            }
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
                if toon_mode {
                    print!("{}", toon::action("claimed", id));
                } else {
                    println!("Claimed task {id} (agent: {agent})");
                }
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
                if toon_mode {
                    print!("{}", toon::action("done", id));
                } else {
                    println!("Completed task {id}");
                }
            }
        }
        Command::Approve(args) => {
            approve_task::approve_task(repo, &args.id).await?;
            if toon_mode {
                print!("{}", toon::action("approved", &args.id));
            } else {
                println!("Approved task {}", args.id);
            }
        }
        Command::Archive(args) => {
            let ids = resolve_archive_ids(repo, args)?;
            if ids.is_empty() && !toon_mode {
                println!("No tasks to archive.");
            }
            for id in &ids {
                archive_task::archive_task(repo, id).await?;
                if toon_mode {
                    print!("{}", toon::action("archived", id));
                } else {
                    println!("Archived task {id}");
                }
            }
        }
        Command::Unarchive(args) => {
            for id in &args.ids {
                archive_task::unarchive_task(repo, id).await?;
                if toon_mode {
                    print!("{}", toon::action("unarchived", id));
                } else {
                    println!("Unarchived task {id}");
                }
            }
        }
        Command::Dep(args) => match &args.subcommand {
            DepCommands::Add(a) => {
                add_dependency::add_dependency(repo, &a.task_id, &a.blocker_id).await?;
                if toon_mode {
                    println!("ok:dep.added:{}:{}", a.task_id, a.blocker_id);
                } else {
                    println!(
                        "Added dependency: {} blocked by {}",
                        a.task_id, a.blocker_id
                    );
                }
            }
            DepCommands::Remove(a) => {
                remove_dependency::remove_dependency(repo, &a.task_id, &a.blocker_id).await?;
                if toon_mode {
                    println!("ok:dep.removed:{}:{}", a.task_id, a.blocker_id);
                } else {
                    println!(
                        "Removed dependency: {} no longer blocked by {}",
                        a.task_id, a.blocker_id
                    );
                }
            }
        },
        Command::MigrateBeads(args) => {
            let result = migrate_beads::migrate_beads(repo, &args.beads_dir).await?;
            if toon_mode {
                println!("ok:migrate:{}:{}", result.migrated, result.skipped);
            } else {
                println!(
                    "Migration complete: {} migrated, {} skipped (already exist)",
                    result.migrated, result.skipped
                );
            }
        }
        Command::Sync => {
            if config.mode == CoreMode::Remote {
                if toon_mode {
                    println!("ok:sync:noop");
                } else {
                    println!("Remote mode — already connected to Core. Nothing to sync.");
                }
            } else {
                sync_http::sync_http(repo.backend(), config, workspace_root).await?;
                if toon_mode {
                    println!("ok:sync");
                }
            }
        }
        Command::Tui | Command::Serve(_) | Command::Prime(_) => {
            unreachable!("handled in main.rs")
        }
    }
    Ok(())
}
