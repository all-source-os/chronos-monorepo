//! Tenant-isolation gate (two checks).
//!
//! **Check 1 — QS PubSub topics (`events:`/`projections:`).**
//! Cross-tenant data spill on the Query Service WebSocket path came from GLOBAL
//! PubSub topics (`events:all`, `events:<entity>`, `events:type:<type>`,
//! `projections:<name>`): any authenticated user could subscribe and receive
//! every tenant's events. The fix made every user-facing topic tenant-scoped
//! (`events:<tenant>:...`). This gate keeps it that way. It scans the Query
//! Service production source (`apps/query-service/lib`) for
//! `Phoenix.PubSub.broadcast` / `subscribe` calls whose topic is an `events:` /
//! `projections:` topic that is NOT tenant-scoped (i.e. not `events:#{...}:`).
//! Such a call must either be fixed or carry an inline `ISOLATION_OK: <reason>`
//! justification on a nearby line.
//!
//! **Check 2 — per-tenant projection compute must NOT live in Core.**
//! Per-tenant projections (epic t-822210) are a Query-Service concern: QS folds
//! a tenant's event stream into read-models. Core IS the durable event store;
//! it may *store* the enabled set as OPAQUE tenant metadata, but it must never
//! *compute or serve* per-tenant projection state — that would rewire Core's
//! ingest hot path and break the Core/QS role split (CLAUDE.md; see
//! docs/proposals/PER_TENANT_PROJECTIONS.md "Why not Core"). This check scans
//! `apps/core/src` and FAILS on identifiers/strings that signal per-tenant
//! projection compute (e.g. `list_projections_for_tenant`, a `tenant`-keyed
//! projection state, folding of `metadata.projections.enabled`). Core's GLOBAL
//! engine projections (`entity_snapshots`, `event_counters`, Prime's 9, the
//! embedded demo set) are internal database read-models and are NOT flagged.
//! A genuine exception must carry an inline `CORE_PROJECTION_OK: <reason>`
//! comment (mirrors `ISOLATION_OK`) — overridable, never silent.
//!
//! Run from the repo root: `cargo run --manifest-path tooling/tenant-isolation-check/Cargo.toml`

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SCAN_DIR: &str = "apps/query-service/lib";
const CORE_SCAN_DIR: &str = "apps/core/src";
const WINDOW: usize = 200; // chars after a call to look for the topic literal
const JUSTIFY_RADIUS: usize = 6; // lines around a call to look for ISOLATION_OK
const MARKER: &str = "ISOLATION_OK";
const CORE_MARKER: &str = "CORE_PROJECTION_OK";

struct Finding {
    file: String,
    line: usize,
    topic: String,
    justified: bool,
}

/// A hit in Core source that looks like per-tenant projection compute.
struct CoreFinding {
    file: String,
    line: usize,
    snippet: String,
    rule: &'static str,
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
        println!("tenant-isolation-check: {} documented PubSub exception(s) (review periodically):", exceptions.len());
        for f in &exceptions {
            println!("  ~ {}:{}  topic {:?}  [{}]", f.file, f.line, f.topic, MARKER);
        }
    }

    // Check 2: per-tenant projection compute must not enter Core.
    let core_scan = root.join(CORE_SCAN_DIR);
    let mut core_files = Vec::new();
    if core_scan.is_dir() {
        collect_rs_files(&core_scan, &mut core_files);
    } else {
        eprintln!("tenant-isolation-check: core scan dir not found: {}", core_scan.display());
        return ExitCode::FAILURE;
    }
    let mut core_findings = Vec::new();
    for path in &core_files {
        let Ok(src) = fs::read_to_string(path) else { continue };
        scan_core_source(&src, path, &root, &mut core_findings);
    }
    let core_violations: Vec<&CoreFinding> = core_findings.iter().filter(|f| !f.justified).collect();
    let core_exceptions: Vec<&CoreFinding> = core_findings.iter().filter(|f| f.justified).collect();

    if !core_exceptions.is_empty() {
        println!("tenant-isolation-check: {} documented Core-projection exception(s) (review periodically):", core_exceptions.len());
        for f in &core_exceptions {
            println!("  ~ {}:{}  [{}] {} — {:?}", f.file, f.line, CORE_MARKER, f.rule, f.snippet);
        }
    }

    let pubsub_ok = violations.is_empty();
    let core_ok = core_violations.is_empty();

    if pubsub_ok {
        println!(
            "tenant-isolation-check: OK — no un-justified global event/projection topics in {SCAN_DIR}"
        );
    } else {
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
    }

    if core_ok {
        println!(
            "tenant-isolation-check: OK — no per-tenant projection compute in {CORE_SCAN_DIR}"
        );
    } else {
        eprintln!("\ntenant-isolation-check: FAILED — {} per-tenant projection-compute hit(s) in Core:", core_violations.len());
        for f in &core_violations {
            eprintln!("  ✘ {}:{}  [{}] {:?}", f.file, f.line, f.rule, f.snippet);
        }
        eprintln!(
            "\nPer-tenant projection compute belongs in the Query Service, NOT Core (CLAUDE.md role split;\n\
             docs/proposals/PER_TENANT_PROJECTIONS.md \"Why not Core\"). Core may STORE the enabled set as\n\
             opaque tenant metadata, but must not COMPUTE/SERVE per-tenant projection state.\n\
             If a hit is a genuine exception, add an inline `{CORE_MARKER}: <reason>` comment nearby."
        );
    }

    if pubsub_ok && core_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Patterns that signal per-tenant projection COMPUTE/SERVE in Core. These are
/// deliberately narrow: they target the *intersection* of "projection" and
/// "tenant", not either word alone (Core legitimately uses both — global engine
/// projections, and tenant metadata storage). The aim is to catch the specific
/// mistake of folding/serving per-tenant projection state in the engine.
fn scan_core_source(src: &str, path: &Path, root: &Path, out: &mut Vec<CoreFinding>) {
    let rel = path.strip_prefix(root).unwrap_or(path).display().to_string();

    for (lineno, raw) in src.lines().enumerate() {
        let line = raw.trim();
        // Skip the gate's own self-reference and doc comments that merely
        // describe the rule (lines that mention the marker are explanatory).
        if line.contains(CORE_MARKER) {
            continue;
        }
        let lower = line.to_lowercase();

        let rule = core_rule(&lower);
        let Some(rule) = rule else { continue };

        let justified = has_core_marker_near(src, lineno, JUSTIFY_RADIUS);
        out.push(CoreFinding {
            file: rel.clone(),
            line: lineno + 1,
            snippet: line.chars().take(120).collect(),
            rule,
            justified,
        });
    }
}

/// Returns the matched rule name if `line` (lowercased) signals per-tenant
/// projection compute, else None.
fn core_rule(line: &str) -> Option<&'static str> {
    // 1. The explicit forbidden API name.
    if line.contains("projections_for_tenant") || line.contains("projection_for_tenant") {
        return Some("per-tenant projection accessor (list_projections_for_tenant)");
    }
    // 2. Folding the enabled SET in Core (storing the opaque blob is fine; the
    //    word `enabled` here means Core is interpreting it).
    if line.contains("projections.enabled") || line.contains("projections_enabled") {
        return Some("Core folding metadata.projections.enabled");
    }
    // 3. A projection-state key / cache that carries a tenant dimension. We only
    //    flag when BOTH a projection-state token AND a tenant token appear on the
    //    same line — i.e. a tenant-keyed projection state. Global state keys
    //    (`"{name}:{entity_id}"`) never mention tenant, so they don't trip this.
    let projection_state = line.contains("projection_state")
        || line.contains("projection state")
        || (line.contains("projection") && (line.contains("get_state") || line.contains("state_cache")));
    let tenant = line.contains("tenant_id") || line.contains("tenant.id") || line.contains("by_tenant");
    if projection_state && tenant {
        return Some("tenant-keyed projection state");
    }
    None
}

fn has_core_marker_near(src: &str, line_idx: usize, radius: usize) -> bool {
    let lines: Vec<&str> = src.lines().collect();
    let lo = line_idx.saturating_sub(radius);
    let hi = (line_idx + radius).min(lines.len().saturating_sub(1));
    (lo..=hi).any(|n| lines.get(n).is_some_and(|l| l.contains(CORE_MARKER)))
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

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            if p.file_name().and_then(|n| n.to_str()) == Some("target") {
                continue;
            }
            collect_rs_files(&p, out);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
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
