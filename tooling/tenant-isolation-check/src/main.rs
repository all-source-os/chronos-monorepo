//! Tenant-isolation gate.
//!
//! Cross-tenant data spill on the Query Service WebSocket path came from GLOBAL
//! PubSub topics (`events:all`, `events:<entity>`, `events:type:<type>`,
//! `projections:<name>`): any authenticated user could subscribe and receive
//! every tenant's events. The fix made every user-facing topic tenant-scoped
//! (`events:<tenant>:...`). This gate keeps it that way.
//!
//! It scans the Query Service production source (`apps/query-service/lib`) for
//! `Phoenix.PubSub.broadcast` / `subscribe` calls whose topic is an `events:` /
//! `projections:` topic that is NOT tenant-scoped (i.e. not `events:#{...}:`).
//! Such a call must either be fixed or carry an inline `ISOLATION_OK: <reason>`
//! justification on a nearby line. The tool prints every justified exception
//! (the audit surface) and FAILS (exit 1) on any unjustified one.
//!
//! Run from the repo root: `cargo run --manifest-path tooling/tenant-isolation-check/Cargo.toml`

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SCAN_DIR: &str = "apps/query-service/lib";
const WINDOW: usize = 200; // chars after a call to look for the topic literal
const JUSTIFY_RADIUS: usize = 6; // lines around a call to look for ISOLATION_OK
const MARKER: &str = "ISOLATION_OK";

struct Finding {
    file: String,
    line: usize,
    topic: String,
    justified: bool,
}

fn main() -> ExitCode {
    let root = repo_root();
    let scan = root.join(SCAN_DIR);
    if !scan.is_dir() {
        eprintln!("tenant-isolation-check: scan dir not found: {}", scan.display());
        return ExitCode::FAILURE;
    }

    let mut files = Vec::new();
    collect_ex_files(&scan, &mut files);

    let mut findings = Vec::new();
    for path in &files {
        let Ok(src) = fs::read_to_string(path) else { continue };
        scan_source(&src, path, &root, &mut findings);
    }

    let violations: Vec<&Finding> = findings.iter().filter(|f| !f.justified).collect();
    let exceptions: Vec<&Finding> = findings.iter().filter(|f| f.justified).collect();

    if !exceptions.is_empty() {
        println!("tenant-isolation-check: {} documented exception(s) (review periodically):", exceptions.len());
        for f in &exceptions {
            println!("  ~ {}:{}  topic {:?}  [{}]", f.file, f.line, f.topic, MARKER);
        }
    }

    if violations.is_empty() {
        println!(
            "tenant-isolation-check: OK — no un-justified global event/projection topics in {SCAN_DIR}"
        );
        return ExitCode::SUCCESS;
    }

    eprintln!("\ntenant-isolation-check: FAILED — {} un-justified global topic(s):", violations.len());
    for f in &violations {
        eprintln!(
            "  ✘ {}:{}  broadcasts/subscribes to a NON-tenant-scoped topic {:?}",
            f.file, f.line, f.topic
        );
    }
    eprintln!(
        "\nEvery user-facing event/projection topic must be tenant-scoped (events:#{{tenant}}:...).\n\
         If a call is a genuine internal/admin exception, add an inline `{MARKER}: <reason>` comment nearby."
    );
    ExitCode::FAILURE
}

fn scan_source(src: &str, path: &Path, root: &Path, out: &mut Vec<Finding>) {
    let rel = path.strip_prefix(root).unwrap_or(path).display().to_string();
    let bytes = src.as_bytes();

    for (idx, _) in match_indices(src, &["PubSub.broadcast(", "PubSub.subscribe("]) {
        let window = &src[idx..(idx + WINDOW).min(src.len())];
        let Some(topic) = first_topic(window) else { continue };
        if is_tenant_scoped(&topic) {
            continue;
        }
        let line = 1 + bytecount(&bytes[..idx], b'\n');
        let justified = has_marker_near(src, idx, JUSTIFY_RADIUS);
        out.push(Finding { file: rel.clone(), line, topic, justified });
    }
}

/// First `"events:..."` / `"projections:..."` string literal in `window`.
fn first_topic(window: &str) -> Option<String> {
    let chars: Vec<char> = window.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' {
            let mut j = i + 1;
            let mut s = String::new();
            while j < chars.len() && chars[j] != '"' {
                s.push(chars[j]);
                j += 1;
            }
            if s.starts_with("events:") || s.starts_with("projections:") {
                return Some(s);
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    None
}

/// Tenant-scoped iff the namespace is immediately followed by an interpolation
/// and another colon: `events:#{<tenant>}:...` / `projections:#{<tenant>}:...`.
fn is_tenant_scoped(topic: &str) -> bool {
    for ns in ["events:", "projections:"] {
        if let Some(rest) = topic.strip_prefix(ns) {
            return rest.starts_with("#{") && rest.contains("}:");
        }
    }
    false
}

fn has_marker_near(src: &str, byte_idx: usize, radius: usize) -> bool {
    let lines: Vec<&str> = src.lines().collect();
    let call_line = bytecount(&src.as_bytes()[..byte_idx], b'\n');
    let lo = call_line.saturating_sub(radius);
    let hi = (call_line + radius).min(lines.len().saturating_sub(1));
    (lo..=hi).any(|n| lines.get(n).is_some_and(|l| l.contains(MARKER)))
}

fn match_indices(hay: &str, needles: &[&str]) -> Vec<(usize, ())> {
    let mut out = Vec::new();
    for n in needles {
        let mut start = 0;
        while let Some(pos) = hay[start..].find(n) {
            out.push((start + pos, ()));
            start += pos + n.len();
        }
    }
    out.sort_by_key(|(i, _)| *i);
    out
}

fn bytecount(bytes: &[u8], b: u8) -> usize {
    bytes.iter().filter(|&&x| x == b).count()
}

fn collect_ex_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_ex_files(&p, out);
        } else if p.extension().and_then(|e| e.to_str()) == Some("ex") {
            out.push(p);
        }
    }
}

fn repo_root() -> PathBuf {
    // The binary lives at tooling/tenant-isolation-check; the repo root is two up
    // from CARGO_MANIFEST_DIR. Fall back to the current dir.
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest);
        if let Some(root) = p.parent().and_then(|x| x.parent()) {
            return root.to_path_buf();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
