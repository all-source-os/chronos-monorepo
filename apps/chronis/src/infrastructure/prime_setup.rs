//! Wire AllSource Prime into Claude Code for the current project.
//!
//! Writes a project-scoped `.mcp.json` at the workspace root pointing at the
//! locally-installed `allsource-prime` binary. The data dir lives under
//! `.chronis/prime/` so Prime memory is colocated with the chronis event log
//! and follows the same gitignore pattern.
//!
//! Why a separate `cn prime setup` command instead of folding it into `cn init`:
//! Prime is opt-in. Forcing every chronis workspace to register an MCP server
//! it never uses is noisy, and would also fail for users who haven't run
//! `cargo install allsource-prime` yet.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{Map, Value, json};

use crate::domain::error::ChronError;

/// Default name for the MCP server entry written into `.mcp.json`.
const SERVER_NAME: &str = "prime";

/// Default Prime binary name (must be on $PATH or be an absolute path).
const DEFAULT_BIN: &str = "allsource-prime";

/// Relative path (from workspace root) where Prime's WAL/Parquet/embeddings live.
const RELATIVE_DATA_DIR: &str = ".chronis/prime";

#[derive(Debug)]
pub struct PrimeSetupReport {
    pub mcp_json_path: PathBuf,
    pub data_dir: PathBuf,
    pub bin_path: String,
    pub bin_version: String,
    /// True when the server entry already existed and was overwritten in place.
    pub replaced_existing: bool,
}

/// Locate the workspace root for the given starting directory.
///
/// We look for `.chronis/` walking upward (matching `Workspace::find_root`)
/// and fall back to `start` itself if no chronis workspace is found — that
/// way `cn prime setup` works in a fresh project that hasn't run `cn init`.
fn locate_root(start: &Path) -> PathBuf {
    let mut dir = start.to_path_buf();
    loop {
        if dir.join(".chronis").is_dir() {
            return dir;
        }
        if !dir.pop() {
            return start.to_path_buf();
        }
    }
}

/// Run `<bin> --version` to confirm the binary exists and capture its version
/// string. Returns a structured error with the remediation command on failure.
fn probe_binary(bin: &str) -> Result<String, ChronError> {
    let out = Command::new(bin).arg("--version").output().map_err(|e| {
        ChronError::Sync(format!(
            "{bin} not found on PATH ({e}). Install it with: cargo install allsource-prime"
        ))
    })?;
    if !out.status.success() {
        return Err(ChronError::Sync(format!(
            "{bin} --version exited with status {}. Reinstall with: cargo install allsource-prime --force",
            out.status
        )));
    }
    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if version.is_empty() {
        return Err(ChronError::Sync(format!(
            "{bin} --version produced no output. Reinstall with: cargo install allsource-prime --force"
        )));
    }
    Ok(version)
}

/// Read the existing `.mcp.json` at `path` if any, otherwise return an empty
/// object. We tolerate (and warn-via-result) unparseable files by surfacing
/// them as an error — the user should not silently lose unrelated MCP entries.
fn load_existing(path: &Path) -> Result<Value, ChronError> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let raw = std::fs::read_to_string(path)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(trimmed).map_err(|e| {
        ChronError::Sync(format!(
            "{} is not valid JSON ({e}). Fix or delete it and re-run `cn prime setup`.",
            path.display()
        ))
    })
}

/// Merge the prime entry into `doc.mcpServers`, returning whether an existing
/// entry was overwritten.
fn merge_prime_entry(doc: &mut Value, prime_entry: Value) -> bool {
    let obj = doc.as_object_mut().expect("doc is an object");
    let servers = obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let servers = servers
        .as_object_mut()
        .expect("mcpServers slot holds an object");
    let replaced = servers.contains_key(SERVER_NAME);
    servers.insert(SERVER_NAME.to_string(), prime_entry);
    replaced
}

/// Append `.chronis/prime/` to `.chronis/.gitignore` if it isn't already there.
/// No-op when there's no `.chronis/` directory or no gitignore — this is a
/// cosmetic nicety, not a correctness requirement.
fn ensure_gitignore_entry(workspace_root: &Path) -> Result<(), ChronError> {
    let chronis_dir = workspace_root.join(".chronis");
    if !chronis_dir.is_dir() {
        return Ok(());
    }
    let gitignore = chronis_dir.join(".gitignore");
    let existing = std::fs::read_to_string(&gitignore).unwrap_or_default();
    if existing.lines().any(|l| l.trim() == "prime/") {
        return Ok(());
    }
    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str("prime/\n");
    std::fs::write(&gitignore, updated)?;
    Ok(())
}

/// Public entry point: wire Prime MCP into the project rooted at `start`.
///
/// - `start` is typically the cwd; we walk up for `.chronis/`.
/// - `bin_override`: when `Some`, used verbatim as the MCP server `command`
///   value (escape hatch for testing against a debug-built binary).
/// - `data_dir_override`: when `Some`, used verbatim instead of the default
///   `<root>/.chronis/prime`. Useful for the verification step which writes
///   into a throwaway directory.
pub fn run_prime_setup(
    start: &Path,
    bin_override: Option<&str>,
    data_dir_override: Option<&Path>,
) -> Result<PrimeSetupReport, ChronError> {
    let bin = bin_override.unwrap_or(DEFAULT_BIN);
    let bin_version = probe_binary(bin)?;

    let workspace_root = locate_root(start);
    let data_dir = data_dir_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workspace_root.join(RELATIVE_DATA_DIR));

    // Make sure the data dir exists. Prime opens it on first run; pre-creating
    // it gives the user a stable filesystem state right after `cn prime setup`.
    std::fs::create_dir_all(&data_dir)?;

    let mcp_json_path = workspace_root.join(".mcp.json");
    let mut doc = load_existing(&mcp_json_path)?;

    let prime_entry = json!({
        "command": bin,
        "args": [
            "--data-dir",
            data_dir.to_string_lossy().to_string(),
        ],
        "env": {}
    });
    let replaced = merge_prime_entry(&mut doc, prime_entry);

    let serialized = serde_json::to_string_pretty(&doc)
        .map_err(|e| ChronError::Sync(format!("serialize .mcp.json: {e}")))?;
    std::fs::write(&mcp_json_path, format!("{serialized}\n"))?;

    ensure_gitignore_entry(&workspace_root)?;

    Ok(PrimeSetupReport {
        mcp_json_path,
        data_dir,
        bin_path: bin.to_string(),
        bin_version,
        replaced_existing: replaced,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fake_bin_script(dir: &Path, output: &str) -> PathBuf {
        let path = dir.join("fake-prime.sh");
        std::fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s\\n' '{output}'\nexit 0\n"),
        )
        .unwrap();
        std::fs::set_permissions(
            &path,
            <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o755),
        )
        .unwrap();
        path
    }

    #[test]
    fn fails_loudly_when_binary_missing() {
        let dir = tempdir().unwrap();
        let err = run_prime_setup(
            dir.path(),
            Some("/definitely/not/a/binary/allsource-prime-nope"),
            None,
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cargo install allsource-prime"), "{msg}");
    }

    #[test]
    fn writes_mcp_json_from_scratch_and_is_idempotent() {
        let dir = tempdir().unwrap();
        let bin = fake_bin_script(dir.path(), "allsource-prime 0.21.5");

        // first run
        let report = run_prime_setup(dir.path(), Some(bin.to_str().unwrap()), None).unwrap();
        assert!(!report.replaced_existing);
        assert_eq!(report.bin_version, "allsource-prime 0.21.5");

        let raw = std::fs::read_to_string(dir.path().join(".mcp.json")).unwrap();
        let doc: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            doc["mcpServers"]["prime"]["command"].as_str(),
            Some(bin.to_str().unwrap())
        );
        assert_eq!(doc["mcpServers"]["prime"]["args"][0], "--data-dir");

        // second run — entry should be replaced, not duplicated, and pre-existing
        // sibling entries should survive.
        let raw_with_sibling = {
            let mut doc: Value = serde_json::from_str(&raw).unwrap();
            doc["mcpServers"]["other"] = json!({"command": "other-bin", "args": []});
            serde_json::to_string_pretty(&doc).unwrap()
        };
        std::fs::write(dir.path().join(".mcp.json"), raw_with_sibling).unwrap();

        let report2 = run_prime_setup(dir.path(), Some(bin.to_str().unwrap()), None).unwrap();
        assert!(report2.replaced_existing);
        let raw2 = std::fs::read_to_string(dir.path().join(".mcp.json")).unwrap();
        let doc2: Value = serde_json::from_str(&raw2).unwrap();
        assert!(doc2["mcpServers"]["other"].is_object());
        assert_eq!(
            doc2["mcpServers"]["prime"]["command"].as_str(),
            Some(bin.to_str().unwrap())
        );
    }

    #[test]
    fn data_dir_override_is_respected_and_pre_created() {
        let dir = tempdir().unwrap();
        let bin = fake_bin_script(dir.path(), "allsource-prime 0.21.5");
        let override_dir = dir.path().join("custom-prime-dir");

        let report =
            run_prime_setup(dir.path(), Some(bin.to_str().unwrap()), Some(&override_dir)).unwrap();
        assert_eq!(report.data_dir, override_dir);
        assert!(override_dir.is_dir());
    }

    #[test]
    fn rejects_invalid_existing_mcp_json() {
        let dir = tempdir().unwrap();
        let bin = fake_bin_script(dir.path(), "allsource-prime 0.21.5");
        std::fs::write(dir.path().join(".mcp.json"), "{ not json").unwrap();
        let err = run_prime_setup(dir.path(), Some(bin.to_str().unwrap()), None).unwrap_err();
        assert!(err.to_string().contains("not valid JSON"));
    }
}
