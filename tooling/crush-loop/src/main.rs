//! crush-loop — ralph-tui's core loop, minus the TUI.
//!
//! Loop: pick a ready child bead of --epic → claim → render prompt →
//! `crush run` (model comes from the repo's .crush.json, e.g. GLM-5.2 via
//! OpenRouter) → verify the bead's `- [ ]` checkboxes got marked → `cn done`.
//! When every child is done, run the epic's quality-gate commands; on failure
//! file a fix-up bead and keep looping.

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(version, about = "Drive chronis beads through non-interactive crush")]
struct Args {
    /// Epic whose ready children get executed
    #[arg(long)]
    epic: String,
    /// Total agent iterations before giving up (retries count)
    #[arg(long, default_value_t = 10)]
    max_iterations: u32,
    /// Attempts per bead before it's declared stuck
    #[arg(long, default_value_t = 2)]
    retries: u32,
    /// Render the next bead's prompt and exit — no claim, no agent
    #[arg(long)]
    dry_run: bool,
    /// Iteration logs directory
    #[arg(long, default_value = ".crush-loop")]
    log_dir: PathBuf,
    /// Prompt template file; placeholders {{id}} {{title}} {{description}} {{epic_id}} {{epic_title}}
    #[arg(long)]
    template: Option<PathBuf>,
    /// Agent command (override to wrap or fake crush in tests)
    #[arg(long, default_value = "crush")]
    agent: String,
}

#[derive(Deserialize, Clone)]
struct Task {
    id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    status: String,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    archived: bool,
}

const DEFAULT_TEMPLATE: &str = r#"You are an autonomous coding agent working ONE task in this repository.

## Task {{id}}: {{title}}
Epic: {{epic_id}} — {{epic_title}}

{{description}}

## Rules
- Touch only what this task needs. No refactors or features beyond the acceptance criteria.
- After every substantive edit, run the matching check for the code you touched (cargo check, bun typecheck, mix compile).
- Verify each acceptance criterion, then mark its checkbox [x] by rewriting the task description:
    cn show {{id}} --toon
    cn task edit {{id}} -d '<full description with completed items marked [x]>'
- When ALL criteria are [x], close the task: cn done {{id}} --toon
- If genuinely blocked, record why and stop:
    cn task edit {{id}} --append-description 'BLOCKED: <reason>'
"#;

fn cn_json(args: &[&str]) -> Result<Vec<Task>> {
    let out = Command::new("cn")
        .args(args)
        .arg("--format")
        .arg("json")
        .output()
        .context("spawning cn")?;
    if !out.status.success() {
        bail!("cn {:?} failed: {}", args, String::from_utf8_lossy(&out.stderr));
    }
    Ok(serde_json::from_slice(&out.stdout).context("parsing cn JSON")?)
}

fn cn_ok(args: &[&str]) -> Result<()> {
    let out = Command::new("cn").args(args).output().context("spawning cn")?;
    if !out.status.success() {
        bail!("cn {:?} failed: {}", args, String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

fn children(epic: &str) -> Result<Vec<Task>> {
    Ok(cn_json(&["list", "--parent", epic])?
        .into_iter()
        .filter(|t| !t.archived && t.id != epic)
        .collect())
}

fn ready_children(epic: &str) -> Result<Vec<Task>> {
    let mut ready: Vec<Task> =
        cn_json(&["list", "--parent", epic, "--status", "open", "--no-blockers", "--unclaimed"])?
            .into_iter()
            .filter(|t| !t.archived && t.id != epic)
            .collect();
    ready.sort_by_key(|t| t.priority.clone().unwrap_or_else(|| "p9".into()));
    Ok(ready)
}

fn unchecked(desc: &str) -> Vec<String> {
    desc.lines().filter(|l| l.trim_start().starts_with("- [ ]")).map(str::to_string).collect()
}

fn has_checkboxes(desc: &str) -> bool {
    desc.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("- [ ]") || t.starts_with("- [x]")
    })
}

/// Epic gates = backticked commands on the epic description's checkbox lines.
fn gate_commands(epic_desc: &str) -> Vec<String> {
    epic_desc
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("- [ ]") || t.starts_with("- [x]")
        })
        .filter_map(|l| {
            let start = l.find('`')?;
            let rest = &l[start + 1..];
            let end = rest.find('`')?;
            Some(rest[..end].to_string())
        })
        .collect()
}

fn render(template: &str, bead: &Task, epic: &Task) -> String {
    template
        .replace("{{id}}", &bead.id)
        .replace("{{title}}", &bead.title)
        .replace("{{description}}", bead.description.as_deref().unwrap_or("(no description)"))
        .replace("{{epic_id}}", &epic.id)
        .replace("{{epic_title}}", &epic.title)
}

fn run_agent(agent: &str, prompt: &str, log_path: &PathBuf) -> Result<bool> {
    let mut child = Command::new(agent)
        .args(["run", "--yolo"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawning `{agent} run`"))?;
    child.stdin.take().unwrap().write_all(prompt.as_bytes())?;
    let out = child.wait_with_output()?;
    let mut log = fs::File::create(log_path)?;
    log.write_all(&out.stdout)?;
    log.write_all(&out.stderr)?;
    let tail: Vec<&str> = std::str::from_utf8(&out.stdout)
        .unwrap_or("")
        .lines()
        .rev()
        .take(8)
        .collect();
    for line in tail.iter().rev() {
        println!("    | {line}");
    }
    Ok(out.status.success())
}

fn run_gate(cmd: &str) -> Result<(bool, String)> {
    let out = Command::new("sh").args(["-lc", cmd]).output()?;
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    let tail: String = combined
        .lines()
        .rev()
        .take(30)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Ok((out.status.success(), tail))
}

fn fetch(id: &str) -> Result<Task> {
    cn_json(&["list", "--all"])?
        .into_iter()
        .find(|t| t.id == id)
        .with_context(|| format!("task {id} not found"))
}

fn main() -> Result<()> {
    let args = Args::parse();
    let epic = fetch(&args.epic)?;
    let template = match &args.template {
        Some(p) => fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?,
        None => DEFAULT_TEMPLATE.to_string(),
    };
    fs::create_dir_all(&args.log_dir)?;

    let mut attempts: std::collections::HashMap<String, u32> = Default::default();

    for iteration in 1..=args.max_iterations {
        let ready: Vec<Task> = ready_children(&epic.id)?
            .into_iter()
            .filter(|t| attempts.get(&t.id).copied().unwrap_or(0) < args.retries)
            .collect();

        let Some(bead) = ready.first().cloned() else {
            let kids = children(&epic.id)?;
            let open: Vec<&Task> = kids.iter().filter(|t| t.status != "done").collect();
            if open.is_empty() {
                println!("All children done — running epic gates.");
                let desc = epic.description.clone().unwrap_or_default();
                let gates = gate_commands(&desc);
                let mut failed = Vec::new();
                for g in &gates {
                    print!("  gate `{g}` ... ");
                    let (ok, tail) = run_gate(g)?;
                    println!("{}", if ok { "PASS" } else { "FAIL" });
                    if !ok {
                        failed.push((g.clone(), tail));
                    }
                }
                if failed.is_empty() {
                    cn_ok(&["done", &epic.id, "--toon"])?;
                    println!("<promise>COMPLETE</promise> epic {} closed.", epic.id);
                    return Ok(());
                }
                for (g, tail) in &failed {
                    let desc = format!(
                        "Epic gate failed: `{g}`\n\n## Acceptance Criteria\n- [ ] `{g}` passes\n\nLast output:\n```\n{tail}\n```"
                    );
                    cn_ok(&[
                        "task", "create", &format!("Fix epic gate: {g}"),
                        "--parent", &epic.id, "-p", "p1", "-d", &desc, "--toon",
                    ])?;
                }
                println!("Filed {} fix-up bead(s); continuing loop.", failed.len());
                continue;
            }
            let stuck: Vec<String> = open.iter().map(|t| format!("{} ({})", t.id, t.status)).collect();
            bail!("no ready beads but epic not done — stuck/blocked: {}", stuck.join(", "));
        };

        println!("[{iteration}/{}] {} — {}", args.max_iterations, bead.id, bead.title);
        let mut prompt = render(&template, &bead, &epic);
        if let Some(n) = attempts.get(&bead.id) {
            let left = unchecked(bead.description.as_deref().unwrap_or(""));
            prompt = format!(
                "RETRY (attempt {}): a previous run left these criteria unchecked:\n{}\n\n{prompt}",
                n + 1,
                left.join("\n")
            );
        }

        if args.dry_run {
            println!("--- prompt (dry run, nothing claimed) ---\n{prompt}");
            return Ok(());
        }

        cn_ok(&["claim", &bead.id, "--toon"])?;
        *attempts.entry(bead.id.clone()).or_insert(0) += 1;
        let log_path = args.log_dir.join(format!("iter{iteration:03}-{}.log", bead.id));
        let agent_ok = run_agent(&args.agent, &prompt, &log_path)?;

        let after = fetch(&bead.id)?;
        let desc = after.description.unwrap_or_default();
        if after.status == "done" {
            println!("  ✓ agent closed {}", bead.id);
        } else if unchecked(&desc).is_empty() && (has_checkboxes(&desc) || agent_ok) {
            cn_ok(&["done", &bead.id, "--toon"])?;
            println!("  ✓ criteria all [x] — closed {}", bead.id);
        } else {
            println!(
                "  ✗ {} unchecked criteria remain (attempt {}/{}), log: {}",
                unchecked(&desc).len(),
                attempts[&bead.id],
                args.retries,
                log_path.display()
            );
        }
    }

    bail!("max iterations reached without closing epic {}", epic.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_commands_only_from_checkbox_lines() {
        let desc = "Intro text with `not a gate`\n- [ ] `task ci` passes\n- [x] `cargo test -p core` green\n- plain bullet `also not a gate`\n";
        assert_eq!(gate_commands(desc), vec!["task ci", "cargo test -p core"]);
    }

    #[test]
    fn unchecked_counts_only_open_boxes() {
        let desc = "- [ ] one\n- [x] two\n  - [ ] indented three\ntext\n";
        assert_eq!(unchecked(desc).len(), 2);
        assert!(has_checkboxes(desc));
        assert!(!has_checkboxes("no boxes here"));
    }

    #[test]
    fn render_fills_placeholders() {
        let bead = Task {
            id: "t-1".into(),
            title: "T".into(),
            description: None,
            status: "open".into(),
            priority: None,
            archived: false,
        };
        let epic = Task { id: "t-0".into(), title: "E".into(), ..bead.clone() };
        let out = render(DEFAULT_TEMPLATE, &bead, &epic);
        assert!(out.contains("Task t-1: T"));
        assert!(out.contains("t-0 — E"));
        assert!(out.contains("(no description)"));
        assert!(!out.contains("{{"));
    }
}
