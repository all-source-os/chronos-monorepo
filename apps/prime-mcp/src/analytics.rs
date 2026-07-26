//! Graph analytics for Prime Hound — `PageRank` centrality and community
//! detection over a materialized [`FullGraph`].
//!
//! These are **pure, read-side** functions: they take the graph the prime app
//! already reads via `Prime::full_graph` and compute over it in the app layer.
//! They never touch Core's projection engine — per the per-tenant-compute
//! boundary, analytics over a tenant's graph belong here, not in Core.
//!
//! Phase 1 of Hound's analytics: `pagerank` (flow-based importance, a better
//! "god node" signal than raw degree) and `communities` (label propagation, a
//! deterministic clustering of the call/define graph into cohesive groups).

use std::collections::{BTreeMap, HashMap};

use allsource_core::prime::types::FullGraph;

/// `PageRank` over the directed graph. Returns a score per node wire-id; scores
/// sum to ~1.0. Dangling nodes (no out-edges) redistribute their mass uniformly
/// so rank is conserved. `damping` is conventionally 0.85.
///
/// Iterates a fixed number of times or until the L1 change falls below a small
/// epsilon — whichever comes first — so the result is deterministic.
#[must_use]
pub fn pagerank(graph: &FullGraph, damping: f64, max_iterations: usize) -> HashMap<String, f64> {
    let n = graph.nodes.len();
    if n == 0 {
        return HashMap::new();
    }
    let nf = n as f64;

    let index: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();

    // Out-adjacency by index. Edges to/from unknown ids are ignored.
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in &graph.edges {
        if let (Some(&s), Some(&t)) = (index.get(e.source.as_str()), index.get(e.target.as_str())) {
            out[s].push(t);
        }
    }

    let teleport = (1.0 - damping) / nf;
    let mut rank = vec![1.0 / nf; n];
    for _ in 0..max_iterations {
        let mut next = vec![teleport; n];
        let mut dangling = 0.0;
        for (i, outs) in out.iter().enumerate() {
            if outs.is_empty() {
                dangling += rank[i];
            } else {
                let share = damping * rank[i] / outs.len() as f64;
                for &t in outs {
                    next[t] += share;
                }
            }
        }
        // Spread dangling mass uniformly so total rank stays 1.0.
        let spread = damping * dangling / nf;
        let mut delta = 0.0;
        for (v, r) in next.iter_mut().zip(rank.iter()) {
            *v += spread;
            delta += (*v - *r).abs();
        }
        rank = next;
        if delta < 1e-9 {
            break;
        }
    }

    graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.clone(), rank[i]))
        .collect()
}

/// Community detection by label propagation (Raghavan et al.), treating edges as
/// undirected. Returns a map of node wire-id → community label (the wire-id of a
/// representative node). Deterministic: nodes are processed in sorted order and
/// ties are broken toward the lexicographically smallest label.
#[must_use]
pub fn communities(graph: &FullGraph, max_iterations: usize) -> HashMap<String, String> {
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in &graph.nodes {
        adjacency.entry(node.id.as_str()).or_default();
    }
    for e in &graph.edges {
        // Skip edges whose endpoints aren't in the node set.
        if adjacency.contains_key(e.source.as_str()) && adjacency.contains_key(e.target.as_str()) {
            adjacency
                .entry(e.source.as_str())
                .or_default()
                .push(e.target.as_str());
            adjacency
                .entry(e.target.as_str())
                .or_default()
                .push(e.source.as_str());
        }
    }

    // Deterministic processing order.
    let mut order: Vec<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    order.sort_unstable();

    // Each node starts in its own community.
    let mut label: HashMap<&str, String> = order.iter().map(|&id| (id, id.to_string())).collect();

    for _ in 0..max_iterations {
        let mut changed = false;
        for &node in &order {
            let neighbors = &adjacency[node];
            if neighbors.is_empty() {
                continue;
            }
            // Tally neighbor labels; BTreeMap keeps keys sorted for tie-breaks.
            let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
            for &nb in neighbors {
                *counts.entry(label[nb].as_str()).or_default() += 1;
            }
            // Highest count wins; on a tie, the smallest label (BTreeMap is
            // ascending, so the first max we see by scanning is the smallest).
            let mut best: Option<(&str, usize)> = None;
            for (lbl, &cnt) in &counts {
                match best {
                    Some((_, bc)) if bc >= cnt => {}
                    _ => best = Some((lbl, cnt)),
                }
            }
            if let Some((best_label, _)) = best
                && label[node] != best_label
            {
                label.insert(node, best_label.to_string());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    label.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// Summarize a label map into (`community_count`, descending community sizes).
#[must_use]
pub fn community_summary(labels: &HashMap<String, String>) -> (usize, Vec<usize>) {
    let mut sizes: HashMap<&str, usize> = HashMap::new();
    for community in labels.values() {
        *sizes.entry(community.as_str()).or_default() += 1;
    }
    let mut descending: Vec<usize> = sizes.values().copied().collect();
    descending.sort_unstable_by(|a, b| b.cmp(a));
    (descending.len(), descending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use allsource_core::prime::types::{FullGraph, GraphEdge, GraphNode, GraphStats};
    use serde_json::json;

    fn node(id: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: "function".to_string(),
            properties: json!({}),
            has_vector: false,
            vector_dim: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn edge(source: &str, target: &str) -> GraphEdge {
        GraphEdge {
            source: source.to_string(),
            target: target.to_string(),
            relation: "calls".to_string(),
            properties: None,
            weight: Some(1.0),
            created_at: chrono::Utc::now(),
        }
    }

    fn graph(nodes: &[&str], edges: &[(&str, &str)]) -> FullGraph {
        FullGraph {
            nodes: nodes.iter().map(|id| node(id)).collect(),
            edges: edges.iter().map(|(s, t)| edge(s, t)).collect(),
            stats: GraphStats::default(),
            has_more: false,
        }
    }

    #[test]
    fn pagerank_ranks_a_hub_highest() {
        // x, y, z all point at hub → hub accumulates the most rank.
        let g = graph(
            &["hub", "x", "y", "z"],
            &[("x", "hub"), ("y", "hub"), ("z", "hub")],
        );
        let pr = pagerank(&g, 0.85, 100);
        let hub = pr["hub"];
        assert!(hub > pr["x"], "hub {hub} should outrank x {}", pr["x"]);
        assert!(hub > pr["y"]);
        assert!(hub > pr["z"]);
        // Scores conserve to ~1.0.
        let total: f64 = pr.values().sum();
        assert!((total - 1.0).abs() < 1e-6, "total rank {total} != 1.0");
    }

    #[test]
    fn pagerank_empty_graph_is_empty() {
        let g = graph(&[], &[]);
        assert!(pagerank(&g, 0.85, 50).is_empty());
    }

    #[test]
    fn communities_finds_two_disconnected_triangles() {
        // Two triangles with no edge between them → exactly two communities.
        let g = graph(
            &["a", "b", "c", "p", "q", "r"],
            &[
                ("a", "b"),
                ("b", "c"),
                ("c", "a"),
                ("p", "q"),
                ("q", "r"),
                ("r", "p"),
            ],
        );
        let labels = communities(&g, 50);
        let (count, sizes) = community_summary(&labels);
        assert_eq!(count, 2, "expected 2 communities, got {count}: {sizes:?}");
        assert_eq!(sizes, vec![3, 3]);
        // Triangle members share a label; the two triangles differ.
        assert_eq!(labels["a"], labels["b"]);
        assert_eq!(labels["a"], labels["c"]);
        assert_eq!(labels["p"], labels["q"]);
        assert_ne!(labels["a"], labels["p"]);
    }
}
