//! Prime Hound — Phase 1 (code) ingest wiring.
//!
//! Walks a source tree via the `hound-extract` crate (Tree-sitter, on-device,
//! no LLM) and folds the result into the local embedded Prime graph as ordinary
//! `prime.node.created` / `prime.edge.created` events. Confidence is carried on
//! every edge so downstream queries can tell AST-certain structure from inferred
//! links — the same EXTRACTED / INFERRED / AMBIGUOUS taxonomy a flat graph file
//! would give you, but durable and queryable:
//!
//!   * `defines` (file → symbol)             — EXTRACTED, weight 1.0
//!   * `calls`   (fn → fn, single match)     — INFERRED,  weight 0.6
//!   * `calls`   (fn → fn, multiple matches) — AMBIGUOUS, weight 0.3
//!
//! Call resolution is name-based within the scanned tree: a call whose name
//! matches exactly one extracted function is INFERRED; matching several is
//! AMBIGUOUS (edges to each); matching none is counted `unresolved` (external /
//! std / macro) and dropped.

use std::{collections::HashMap, path::Path};

use allsource_core::prime::Prime;
use hound_extract::{RefKind, SymbolKind, extract};
use serde_json::json;

#[derive(Debug, Default)]
pub struct HoundSummary {
    pub files: usize,
    pub nodes: usize,
    pub edges: usize,
    pub defines: usize,
    pub calls: usize,
    pub ambiguous: usize,
    pub unresolved: usize,
    /// Symbols embedded for hybrid (vector) recall — 0 unless `embed` was set.
    pub embedded: usize,
    /// Stale code nodes removed before re-ingest — 0 unless `rebuild` was set.
    pub deleted_stale: usize,
}

/// Embed `text` under a node's wire id so `prime_recall` can find it by meaning.
///
/// The vector is stored at `vec:{wire}`; recall strips the `vec:` prefix and
/// looks the node back up — so passing the node's wire id as the vector id is
/// what wires a symbol into hybrid search. Best-effort: a transient embed
/// failure is swallowed (the caller warms the embedder first, so "no model" is
/// caught before any writes), a store failure propagates.
async fn embed_node(prime: &Prime, wire: &str, text: &str) -> anyhow::Result<bool> {
    match prime.embed_text(text) {
        Ok(vector) => {
            prime.embed(wire, Some(text), vector).await?;
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}

/// Extract `root` and write the resulting graph into the embedded `prime` store.
///
/// A call site found in pass 1, resolved to an edge in pass 2 once every call
/// target is guaranteed live.
struct Pending {
    file: String,
    from_fn: Option<String>,
    target: String,
    line: usize,
    file_wire: String,
}

/// When `embed` is true, each file and symbol node is also embedded (in-process,
/// no LLM) so the code graph is searchable by meaning via `prime_recall` — the
/// hybrid-retrieval edge a flat graph file can't offer. Embedding is opt-in
/// because it runs the model once per node.
///
/// When `rebuild` is true, every existing code node (`properties.domain ==
/// "code"`) is deleted first (cascading its edges) so a re-ingest replaces the
/// graph instead of duplicating it — the idempotency a git post-commit hook
/// needs. Non-code nodes (agent memory) are left untouched.
pub async fn ingest(
    prime: &Prime,
    root: &Path,
    embed: bool,
    rebuild: bool,
) -> anyhow::Result<HoundSummary> {
    let result = extract(root)?;
    let mut s = HoundSummary {
        files: result.files.len(),
        ..Default::default()
    };

    if rebuild {
        // Snapshot first, then delete — the snapshot is a Vec so deleting from
        // the live projections mid-iteration is safe.
        let existing = prime.full_graph(None, None, None);
        for n in &existing.nodes {
            if n.properties.get("domain").and_then(|v| v.as_str()) == Some("code")
                && prime.delete_node(&n.id).await.is_ok()
            {
                s.deleted_stale += 1;
            }
        }
    }

    // Fail fast before any writes if embedding was requested but the embedder
    // can't load — rather than ingesting a half-embedded graph. Mirrors the
    // actionable error `--mode warm` surfaces.
    if embed {
        prime.embed_text("warm").map_err(|e| {
            anyhow::anyhow!("embedding requested but the embedder is unavailable: {e}")
        })?;
    }

    // function name → wire ids (cross-file call-target resolution)
    let mut fn_by_name: HashMap<String, Vec<String>> = HashMap::new();
    // (file, function name) → wire id (resolve the enclosing caller)
    let mut fn_in_file: HashMap<(String, String), String> = HashMap::new();

    let mut pending: Vec<Pending> = Vec::new();

    // Pass 1 — create every node and its `defines` edge first, so all call
    // targets are live before any `calls` edge is added (add_edge requires both
    // endpoints to exist in the node-state projection).
    for fg in &result.files {
        let file_uuid = prime
            .add_node(
                "file",
                json!({ "path": fg.path, "language": fg.language, "domain": "code" }),
            )
            .await?;
        let file_wire = format!("node:file:{}", file_uuid.as_str());
        s.nodes += 1;
        if embed && embed_node(prime, &file_wire, &format!("file {}", fg.path)).await? {
            s.embedded += 1;
        }

        for sym in &fg.symbols {
            let node_type = sym.kind.as_node_type();
            let uuid = prime
                .add_node(
                    node_type,
                    json!({ "name": sym.name, "file": fg.path, "line": sym.line, "domain": "code" }),
                )
                .await?;
            let wire = format!("node:{node_type}:{}", uuid.as_str());
            s.nodes += 1;

            prime
                .add_edge_weighted(
                    &file_wire,
                    &wire,
                    "defines",
                    1.0,
                    Some(json!({ "confidence": "EXTRACTED", "line": sym.line })),
                )
                .await?;
            s.edges += 1;
            s.defines += 1;

            // Embed the symbol as "{kind} {name} in {file}" so a meaning-based
            // query (e.g. "authentication") recalls it even when the words don't
            // match the identifier.
            if embed
                && embed_node(
                    prime,
                    &wire,
                    &format!("{node_type} {} in {}", sym.name, fg.path),
                )
                .await?
            {
                s.embedded += 1;
            }

            if sym.kind == SymbolKind::Function {
                fn_by_name
                    .entry(sym.name.clone())
                    .or_default()
                    .push(wire.clone());
                fn_in_file.insert((fg.path.clone(), sym.name.clone()), wire);
            }
        }

        for r in &fg.references {
            if r.kind == RefKind::Call {
                pending.push(Pending {
                    file: fg.path.clone(),
                    from_fn: r.from_fn.clone(),
                    target: r.name.clone(),
                    line: r.line,
                    file_wire: file_wire.clone(),
                });
            }
        }
    }

    // Pass 2 — resolve each call by name and link it.
    for p in pending {
        let Some(targets) = fn_by_name.get(&p.target) else {
            s.unresolved += 1; // external / std / macro — not defined in this tree
            continue;
        };
        let source_wire = p
            .from_fn
            .and_then(|name| fn_in_file.get(&(p.file.clone(), name)).cloned())
            .unwrap_or(p.file_wire);
        let (confidence, weight) = if targets.len() == 1 {
            ("INFERRED", 0.6)
        } else {
            ("AMBIGUOUS", 0.3)
        };
        for target in targets {
            if *target == source_wire {
                continue; // self / recursion — no edge value here
            }
            prime
                .add_edge_weighted(
                    &source_wire,
                    target,
                    "calls",
                    weight,
                    Some(json!({ "confidence": confidence, "line": p.line })),
                )
                .await?;
            s.edges += 1;
            s.calls += 1;
            if confidence == "AMBIGUOUS" {
                s.ambiguous += 1;
            }
        }
    }

    Ok(s)
}
