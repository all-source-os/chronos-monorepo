use std::process::Command;

use crate::domain::error::ChronError;

pub fn sync_git() -> Result<(), ChronError> {
    // Check git is available
    let status = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|e| {
            ChronError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("git not found: {e}"),
            ))
        })?;

    if !status.status.success() {
        return Err(ChronError::Io(std::io::Error::other(
            "not inside a git repository",
        )));
    }

    // Stage .chronis/ directory
    let add = Command::new("git").args(["add", ".chronis/"]).output()?;
    if !add.status.success() {
        let stderr = String::from_utf8_lossy(&add.stderr);
        return Err(ChronError::Io(std::io::Error::other(format!(
            "git add failed: {stderr}"
        ))));
    }

    // Check if there are staged changes
    let diff = Command::new("git")
        .args(["diff", "--cached", "--quiet", ".chronis/"])
        .output()?;

    if diff.status.success() {
        println!("Nothing to sync — .chronis/ is up to date.");
        return Ok(());
    }

    // Commit
    let commit = Command::new("git")
        .args(["commit", "-m", "chronis: sync"])
        .output()?;
    if !commit.status.success() {
        let stderr = String::from_utf8_lossy(&commit.stderr);
        return Err(ChronError::Io(std::io::Error::other(format!(
            "git commit failed: {stderr}"
        ))));
    }
    println!("Committed .chronis/ changes.");

    // Push
    let push = Command::new("git").args(["push"]).output()?;
    if !push.status.success() {
        let stderr = String::from_utf8_lossy(&push.stderr);
        eprintln!("Warning: git push failed: {stderr}");
        eprintln!("Changes are committed locally. Push manually when ready.");
    } else {
        println!("Pushed to remote.");
    }

    Ok(())
}
