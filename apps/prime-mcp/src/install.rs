//! `prime hound install` — write a Prime Hound usage skill into an AI coding
//! assistant's config, so the assistant knows to reach the code graph over MCP
//! instead of grepping.
//!
//! Mechanical, like Graphify's installer: each platform gets a skill/rules file
//! at its conventional path. We write *instructions* (including how to wire the
//! `allsource-prime` MCP server) — we never auto-edit the assistant's MCP config,
//! which is riskier and platform-specific.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The shared body: how an assistant should drive Prime Hound. Kept platform-
/// agnostic; each platform wraps it with its own frontmatter.
const GUIDE: &str = r#"Prime Hound turns this repository into a durable, queryable knowledge graph you
reach over MCP. Prefer it over grepping whenever you need *structure*: what calls
what, the blast radius of a change, an orientation report, or meaning-based
("find the auth code") search. The graph is durable and re-queryable across the
session, not a one-shot dump.

**Prerequisite:** the `allsource-prime` MCP server must be configured (stdio).
Point it at a data dir, e.g. run `allsource-prime --mode mcp --data-dir ~/.prime/memory`.
See https://www.all-source.xyz to connect.

**Workflow**

1. **Build / refresh the graph** — call `hound_ingest` with
   `{ "path": ".", "embed": true }`. Tree-sitter parses the code on-device (no
   LLM); `embed` adds vectors so semantic recall works. Run once per session, or
   after large changes.
2. **Orient on the codebase** — `hound_report` (add `{ "markdown": true }` for a
   readable report): node/edge counts, confidence tiers, the PageRank "god
   nodes" (architectural hubs), and communities.
3. **Before changing a function** — `hound_impact` `{ "target": "<fn name>" }`:
   everything that transitively calls it, i.e. what could break.
4. **Reviewing a PR** — `hound_pr_impact`
   `{ "files": [<output of `git diff --name-only main..HEAD`>] }`: the changed
   symbols ranked by blast radius. Review the top entries most carefully.
5. **Find code by meaning** — `prime_recall` `{ "text": "<description>" }`:
   hybrid vector + graph search (requires `embed: true` at ingest).

**Tools:** `hound_ingest`, `hound_report`, `hound_impact`, `hound_pr_impact`,
`prime_recall`, `prime_neighbors`, `prime_shortest_path`.
"#;

const DESCRIPTION: &str = "Query this repo's code as a durable knowledge graph (Prime Hound) over MCP — \
impact analysis, PR triage, structural reports, and semantic code search. Use \
instead of grepping when you need structure.";

/// A Claude-Code / generic Agent-Skills `SKILL.md` (YAML frontmatter + body).
fn skill_md() -> String {
    format!("---\nname: prime-hound\ndescription: {DESCRIPTION}\n---\n\n{GUIDE}")
}

/// A Cursor `.mdc` rule (frontmatter differs from a skill).
fn cursor_mdc() -> String {
    format!("---\ndescription: {DESCRIPTION}\nglobs:\nalwaysApply: false\n---\n\n{GUIDE}")
}

/// A platform: its key, the relative install path, and the content generator.
type Platform = (&'static str, &'static str, fn() -> String);

/// Supported platforms.
fn platforms() -> Vec<Platform> {
    vec![
        (
            "claude-code",
            ".claude/skills/prime-hound/SKILL.md",
            skill_md as fn() -> String,
        ),
        ("cursor", ".cursor/rules/prime-hound.mdc", cursor_mdc),
        ("agents", ".agents/skills/prime-hound/SKILL.md", skill_md),
    ]
}

/// Comma-separated list of installable platform keys (for help text / errors).
#[must_use]
pub fn platform_keys() -> String {
    "claude-code, cursor, agents, git-hook, all".to_string()
}

/// Write the Hound skill for `platform` (or "all") under `root`. Returns the
/// paths written. Creates parent directories as needed; overwrites an existing
/// skill file (re-install refreshes it).
pub fn run(root: &Path, platform: &str) -> Result<Vec<PathBuf>> {
    // Dedupe by path so the `cursor`/`.cursor` aliases don't double-write.
    let mut written: Vec<PathBuf> = Vec::new();
    for (key, rel, make) in platforms() {
        let want = platform == "all" || platform.eq_ignore_ascii_case(key);
        if !want {
            continue;
        }
        let path = root.join(rel);
        if written.contains(&path) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, make()).with_context(|| format!("write {}", path.display()))?;
        written.push(path);
    }

    if written.is_empty() {
        anyhow::bail!(
            "unknown platform '{platform}' — choose one of: {}",
            platform_keys()
        );
    }
    Ok(written)
}

/// Install a git `post-commit` hook (in the already-resolved `hooks_dir`) that
/// re-runs Hound in `--rebuild` mode after each commit, keeping the code graph
/// fresh for `repo_root`. `exe` is the `allsource-prime` binary and `data_dir`
/// its store. Returns the hook path.
///
/// The caller resolves `hooks_dir` (respecting `core.hooksPath`) and guards
/// against shared/global dirs. Won't clobber an unrelated existing hook: if a
/// `post-commit` is already there without our marker, it errors and prints the
/// line to add by hand. Re-running over our own hook refreshes it.
pub fn git_hook(
    hooks_dir: &Path,
    repo_root: &Path,
    exe: &Path,
    data_dir: &Path,
) -> Result<PathBuf> {
    std::fs::create_dir_all(hooks_dir)
        .with_context(|| format!("create {}", hooks_dir.display()))?;
    let path = hooks_dir.join("post-commit");

    let line = format!(
        "\"{}\" --mode hound --rebuild --data-dir \"{}\" \"{}\" >/dev/null 2>&1 || true",
        exe.display(),
        data_dir.display(),
        repo_root.display()
    );
    let body = format!(
        "#!/bin/sh\n# Prime Hound — refresh the code graph after each commit (idempotent).\n# Installed by `allsource-prime --mode install --platform git-hook`.\n{line}\n"
    );

    if path.exists() {
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if !current.contains("Prime Hound") {
            anyhow::bail!(
                "a post-commit hook already exists at {} — add this line to it yourself:\n  {line}",
                path.display()
            );
        }
    }

    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("chmod {}", path.display()))?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_claude_code_skill_with_frontmatter_and_tools() {
        let dir = tempfile::tempdir().unwrap();
        let written = run(dir.path(), "claude-code").unwrap();
        assert_eq!(written.len(), 1);
        let p = dir.path().join(".claude/skills/prime-hound/SKILL.md");
        assert_eq!(written[0], p);
        let body = std::fs::read_to_string(&p).unwrap();
        assert!(body.starts_with("---\nname: prime-hound\n"));
        assert!(body.contains("hound_pr_impact"));
        assert!(body.contains("hound_ingest"));
        assert!(body.contains("prime_recall"));
    }

    #[test]
    fn install_all_writes_each_platform_once() {
        let dir = tempfile::tempdir().unwrap();
        let written = run(dir.path(), "all").unwrap();
        // claude-code + cursor + agents = 3 distinct files (cursor alias dedup'd).
        assert_eq!(written.len(), 3);
        assert!(dir.path().join(".cursor/rules/prime-hound.mdc").exists());
        assert!(
            dir.path()
                .join(".agents/skills/prime-hound/SKILL.md")
                .exists()
        );
    }

    #[test]
    fn unknown_platform_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(run(dir.path(), "emacs").is_err());
    }

    #[test]
    fn cursor_rule_has_cursor_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        run(dir.path(), "cursor").unwrap();
        let body =
            std::fs::read_to_string(dir.path().join(".cursor/rules/prime-hound.mdc")).unwrap();
        assert!(body.contains("alwaysApply: false"));
        assert!(body.contains("hound_impact"));
    }

    #[test]
    fn git_hook_writes_executable_post_commit() {
        let dir = tempfile::tempdir().unwrap();
        let hooks = dir.path().join(".git/hooks"); // created by git_hook if absent
        let p = git_hook(
            &hooks,
            dir.path(),
            Path::new("/usr/local/bin/allsource-prime"),
            Path::new("/home/u/.prime/memory"),
        )
        .unwrap();
        assert!(p.ends_with("post-commit"));
        let body = std::fs::read_to_string(&p).unwrap();
        assert!(body.contains("--mode hound --rebuild"));
        assert!(body.contains("Prime Hound"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&p).unwrap().permissions().mode();
            assert!(mode & 0o111 != 0, "hook must be executable");
        }
        // Re-installing over our own hook is fine.
        assert!(git_hook(&hooks, dir.path(), Path::new("/x"), Path::new("/y")).is_ok());
    }

    #[test]
    fn git_hook_refuses_to_clobber_a_foreign_hook() {
        let dir = tempfile::tempdir().unwrap();
        let hooks = dir.path().join(".git/hooks");
        std::fs::create_dir_all(&hooks).unwrap();
        std::fs::write(
            hooks.join("post-commit"),
            "#!/bin/sh\necho someone-elses-hook\n",
        )
        .unwrap();
        assert!(git_hook(&hooks, dir.path(), Path::new("/x"), Path::new("/y")).is_err());
    }
}
