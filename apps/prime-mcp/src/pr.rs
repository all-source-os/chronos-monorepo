//! PR impact analysis for Prime Hound — Graphify's `prs` feature, built on the
//! code graph.
//!
//! Given the files a pull request changes (from `git diff --name-only`), map
//! each one onto the symbols it defines, then compute every symbol's **blast
//! radius**: the set of functions that transitively *call* it. A change to a
//! widely-called symbol is high-risk; one nothing depends on is safe. The output
//! is a review queue ranked by impact — review the most-depended-upon first.
//!
//! The git half (which files changed) is the caller's job; this is the pure,
//! testable graph half. It runs in the app over `Prime::full_graph`, never in
//! Core.

use std::collections::{HashMap, HashSet, VecDeque};

use allsource_core::prime::types::{FullGraph, GraphNode};
use serde_json::{json, Value};

/// One changed symbol and the code that depends on it.
pub struct ChangedSymbol {
    pub file: String,
    pub symbol: String,
    pub id: String,
    pub kind: String,
    /// Transitive caller count — the blast radius.
    pub impact: usize,
    /// A sample of caller labels (capped).
    pub callers: Vec<String>,
}

/// The PR impact report.
pub struct PrImpact {
    pub changed_files: usize,
    pub files_in_graph: usize,
    /// Changed files with no symbols in the graph (new files, non-code, or a
    /// stale graph — re-run `hound_ingest`).
    pub files_not_found: Vec<String>,
    /// Changed symbols, ranked by impact (most depended-upon first).
    pub symbols: Vec<ChangedSymbol>,
    /// Unique functions across the whole PR that could be affected.
    pub total_impacted: usize,
}

/// Does changed-file path `changed` (repo-relative) refer to graph-file path
/// `graph_file` (ingest-root-relative)? Matches exact, path-suffix, or basename.
fn path_match(changed: &str, graph_file: &str) -> bool {
    if changed == graph_file
        || changed.ends_with(&format!("/{graph_file}"))
        || graph_file.ends_with(&format!("/{changed}"))
    {
        return true;
    }
    let base = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
    base(changed) == base(graph_file)
}

fn label_of<'a>(props: &'a Value, id: &'a str) -> &'a str {
    props
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| props.get("path").and_then(Value::as_str))
        .unwrap_or(id)
}

/// BFS the incoming-`calls` graph from `start`, up to `depth` hops, returning
/// the count of unique callers (excluding `start`) and a capped sample of their
/// labels. Also folds every caller id into `affected` for the PR-wide total.
fn transitive_callers<'a>(
    incoming: &HashMap<&'a str, Vec<&'a str>>,
    label: &HashMap<&'a str, &'a str>,
    start: &'a str,
    depth: usize,
    affected: &mut HashSet<&'a str>,
) -> (usize, Vec<String>) {
    let mut visited: HashSet<&str> = HashSet::from([start]);
    let mut queue: VecDeque<(&str, usize)> = VecDeque::from([(start, 0usize)]);
    let mut callers: Vec<String> = Vec::new();

    while let Some((cur, d)) = queue.pop_front() {
        if d >= depth {
            continue;
        }
        if let Some(srcs) = incoming.get(cur) {
            for &src in srcs {
                if visited.insert(src) {
                    affected.insert(src);
                    if callers.len() < 12 {
                        callers.push(label.get(src).copied().unwrap_or(src).to_string());
                    }
                    queue.push_back((src, d + 1));
                }
            }
        }
    }
    (visited.len() - 1, callers)
}

/// Compute the PR impact of `changed_files` over `graph`. `depth` bounds the
/// transitive-caller search (2 is a sensible default).
#[must_use]
pub fn pr_impact(graph: &FullGraph, changed_files: &[String], depth: usize) -> PrImpact {
    let label: HashMap<&str, &str> = graph
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), label_of(&n.properties, n.id.as_str())))
        .collect();

    // Incoming-calls adjacency: callee id → caller ids.
    let mut incoming: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &graph.edges {
        if e.relation == "calls" {
            incoming.entry(e.target.as_str()).or_default().push(e.source.as_str());
        }
    }

    let mut affected: HashSet<&str> = HashSet::new();
    let mut symbols: Vec<ChangedSymbol> = Vec::new();
    let mut files_in_graph = 0;
    let mut files_not_found: Vec<String> = Vec::new();

    for cf in changed_files {
        // Symbols defined in this file carry properties.file == its graph path.
        let matched: Vec<&_> = graph
            .nodes
            .iter()
            .filter(|n| {
                n.properties
                    .get("file")
                    .and_then(Value::as_str)
                    .is_some_and(|f| path_match(cf, f))
            })
            .collect();

        if matched.is_empty() {
            files_not_found.push(cf.clone());
            continue;
        }
        files_in_graph += 1;

        for n in matched {
            let (impact, callers) =
                transitive_callers(&incoming, &label, n.id.as_str(), depth, &mut affected);
            symbols.push(ChangedSymbol {
                file: cf.clone(),
                symbol: label.get(n.id.as_str()).copied().unwrap_or(&n.id).to_string(),
                id: n.id.clone(),
                kind: n.node_type.clone(),
                impact,
                callers,
            });
        }
    }

    // Review the most depended-upon first; symbol name as a stable tiebreak.
    symbols.sort_by(|a, b| b.impact.cmp(&a.impact).then(a.symbol.cmp(&b.symbol)));

    PrImpact {
        changed_files: changed_files.len(),
        files_in_graph,
        files_not_found,
        symbols,
        total_impacted: affected.len(),
    }
}

impl PrImpact {
    /// The `hound_pr_impact` MCP payload.
    #[must_use]
    pub fn to_json(&self, top: usize) -> Value {
        let queue: Vec<Value> = self
            .symbols
            .iter()
            .take(top)
            .map(|s| {
                json!({
                    "symbol": s.symbol,
                    "file": s.file,
                    "kind": s.kind,
                    "id": s.id,
                    "impact": s.impact,
                    "callers": s.callers,
                })
            })
            .collect();
        json!({
            "changed_files": self.changed_files,
            "files_in_graph": self.files_in_graph,
            "files_not_found": self.files_not_found,
            "total_impacted": self.total_impacted,
            "review_queue": queue,
            "note": if self.files_in_graph == 0 {
                "No changed files matched the graph — run hound_ingest on this repo first, or the changes are all new/non-code files."
            } else {
                "review_queue is ranked by blast radius (transitive callers); review the top entries most carefully."
            },
        })
    }
}

/// Single-symbol blast radius over a [`FullGraph`] — the `hound_impact` answer
/// computed purely from the materialized graph (the hosted path has no live
/// `neighbors_within`). Mirrors the embedded `hound_impact` output shape.
#[must_use]
pub fn impact_of(graph: &FullGraph, target: &str, depth: usize) -> Value {
    let node_by_id: HashMap<&str, &GraphNode> =
        graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut incoming: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &graph.edges {
        if e.relation == "calls" {
            incoming.entry(e.target.as_str()).or_default().push(e.source.as_str());
        }
    }

    // Resolve the target: a wire id, or a function name (possibly several).
    let targets: Vec<&GraphNode> = if target.starts_with("node:") {
        graph.nodes.iter().filter(|n| n.id == target).collect()
    } else {
        graph
            .nodes
            .iter()
            .filter(|n| {
                n.node_type == "function"
                    && n.properties.get("name").and_then(Value::as_str) == Some(target)
            })
            .collect()
    };
    if targets.is_empty() {
        return json!({
            "target": target, "matches": 0, "results": [],
            "message": "no function with that name in the graph — ingest/sync the repo first?",
        });
    }

    let results: Vec<Value> = targets
        .iter()
        .map(|node| {
            let impacted = impacted_callers(&incoming, &node_by_id, node.id.as_str(), depth);
            json!({
                "target": node.id,
                "target_name": node.properties.get("name").and_then(Value::as_str),
                "target_file": node.properties.get("file").and_then(Value::as_str),
                "impacted_count": impacted.len(),
                "impacted": impacted,
            })
        })
        .collect();
    json!({ "target": target, "depth": depth, "matches": results.len(), "results": results })
}

/// BFS the incoming-`calls` graph from `start` up to `depth` hops, returning each
/// transitive caller with its details + the hop distance.
fn impacted_callers<'a>(
    incoming: &HashMap<&'a str, Vec<&'a str>>,
    node_by_id: &HashMap<&'a str, &'a GraphNode>,
    start: &'a str,
    depth: usize,
) -> Vec<Value> {
    let mut visited: HashSet<&str> = HashSet::from([start]);
    let mut queue: VecDeque<(&str, usize)> = VecDeque::from([(start, 0usize)]);
    let mut out = Vec::new();
    while let Some((cur, d)) = queue.pop_front() {
        if d >= depth {
            continue;
        }
        if let Some(srcs) = incoming.get(cur) {
            for &src in srcs {
                if visited.insert(src) {
                    if let Some(n) = node_by_id.get(src) {
                        out.push(json!({
                            "id": src,
                            "name": n.properties.get("name").and_then(Value::as_str),
                            "file": n.properties.get("file").and_then(Value::as_str),
                            "line": n.properties.get("line"),
                            "depth": d + 1,
                        }));
                    }
                    queue.push_back((src, d + 1));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use allsource_core::prime::types::{FullGraph, GraphEdge, GraphNode, GraphStats};

    fn fnode(id: &str, name: &str, file: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: "function".to_string(),
            properties: json!({ "name": name, "file": file }),
            has_vector: false,
            vector_dim: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
    fn call(src: &str, dst: &str) -> GraphEdge {
        GraphEdge {
            source: src.to_string(),
            target: dst.to_string(),
            relation: "calls".to_string(),
            properties: None,
            weight: Some(0.6),
            created_at: chrono::Utc::now(),
        }
    }

    // authenticate (auth.rs) is called by x, y, z; helper (auth.rs) only by
    // authenticate; render (ui.rs) by nobody.
    fn sample() -> FullGraph {
        let nodes = vec![
            fnode("node:function:auth", "authenticate", "auth.rs"),
            fnode("node:function:help", "helper", "auth.rs"),
            fnode("node:function:render", "render", "ui.rs"),
            fnode("node:function:x", "x", "a.rs"),
            fnode("node:function:y", "y", "b.rs"),
            fnode("node:function:z", "z", "c.rs"),
        ];
        let edges = vec![
            call("node:function:x", "node:function:auth"),
            call("node:function:y", "node:function:auth"),
            call("node:function:z", "node:function:auth"),
            call("node:function:auth", "node:function:help"),
        ];
        FullGraph { nodes, edges, stats: GraphStats::default(), has_more: false }
    }

    #[test]
    fn ranks_changed_symbols_by_blast_radius() {
        let g = sample();
        let r = pr_impact(&g, &["apps/x/auth.rs".to_string()], 3);
        assert_eq!(r.files_in_graph, 1);
        // helper has the BIGGER blast radius and ranks first: changing it affects
        // authenticate (calls it) AND x/y/z (transitively, via authenticate) = 4.
        assert_eq!(r.symbols[0].symbol, "helper");
        assert_eq!(r.symbols[0].impact, 4);
        // authenticate is called directly by x/y/z = 3.
        let auth = r.symbols.iter().find(|s| s.symbol == "authenticate").unwrap();
        assert_eq!(auth.impact, 3);
        // PR-wide unique affected functions: x, y, z, and authenticate = 4.
        assert_eq!(r.total_impacted, 4);
    }

    #[test]
    fn unmatched_file_is_reported_not_crashed() {
        let g = sample();
        let r = pr_impact(&g, &["brand/new/file.rs".to_string()], 2);
        assert_eq!(r.files_in_graph, 0);
        assert_eq!(r.files_not_found, vec!["brand/new/file.rs".to_string()]);
        assert!(r.symbols.is_empty());
    }

    #[test]
    fn depth_one_counts_only_direct_callers() {
        let g = sample();
        let r = pr_impact(&g, &["auth.rs".to_string()], 1);
        let helper = r.symbols.iter().find(|s| s.symbol == "helper").unwrap();
        // At depth 1, helper sees only its direct caller `authenticate`.
        assert_eq!(helper.impact, 1);
    }
}
