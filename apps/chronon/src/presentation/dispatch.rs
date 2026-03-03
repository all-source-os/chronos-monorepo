use crate::{
    application::{approve_task, claim_task, complete_task, create_task, get_task, list_tasks},
    domain::{error::ChronError, repository::TaskRepository},
    infrastructure::workspace,
    presentation::{
        cli::{Command, TaskCommands},
        output::print_task_table,
    },
};

pub fn dispatch_init() -> Result<(), ChronError> {
    let cwd = std::env::current_dir()?;
    workspace::init_workspace(&cwd)
}

pub async fn dispatch(cmd: &Command, repo: &impl TaskRepository) -> Result<(), ChronError> {
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
                    },
                )
                .await?;
                println!("Created task {}: {}", output.id, create_args.title);
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
            println!("Title: {}", task.title);
            println!("Priority: {}", task.priority);
            println!("Status: {}", task.status);
            if let Some(ref claimed) = task.claimed_by {
                println!("Claimed by: {claimed}");
            }
            if !task.blocked_by.is_empty() {
                println!("Blocked by: {}", task.blocked_by.join(", "));
            }
            if let Some(ref reason) = task.done_reason {
                println!("Done reason: {reason}");
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
            let agent = std::env::var("CN_AGENT_ID").unwrap_or_else(|_| "human".to_string());
            claim_task::claim_task(repo, &args.id, &agent).await?;
            println!("Claimed task {} (agent: {agent})", args.id);
        }
        Command::Done(args) => {
            complete_task::complete_task(repo, &args.id, args.reason.as_deref()).await?;
            println!("Completed task {}", args.id);
        }
        Command::Approve(args) => {
            approve_task::approve_task(repo, &args.id).await?;
            println!("Approved task {}", args.id);
        }
        Command::Tui | Command::Serve(_) => {
            unreachable!("handled in main.rs")
        }
    }
    Ok(())
}
