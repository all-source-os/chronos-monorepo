//! `GRAPH_REPORT.md` generation for Prime Hound.
//!
//! [`compute`] folds a materialized [`FullGraph`] (plus the app-side
//! [`crate::analytics`]) into a [`ReportData`] once; [`ReportData::to_json`]
//! shapes the `hound_report` MCP payload and [`ReportData::to_markdown`] renders
//! the human-readable report Graphify writes as a file. Both views come from the
//! same computation so they never drift.

use std::collections::{BTreeMap, HashMap};

use allsource_core::prime::types::FullGraph;
use serde_json::{Value, json};

/// One central symbol in the report.
pub struct GodNode {
    pub id: String,
    pub label: String,
    pub degree: usize,
    pub pagerank: f64,
    pub community: String,
}

/// A relationship that crosses a community boundary — a "surprising" link in
/// Graphify's sense (code that ties two otherwise-separate clusters together).
pub struct CrossLink {
    pub from: String,
    pub relation: String,
    pub to: String,
}

/// Everything the report needs, computed once from the graph.
pub struct ReportData {
    pub node_count: usize,
    pub edge_count: usize,
    pub nodes_by_type: BTreeMap<String, usize>,
    pub edges_by_relation: BTreeMap<String, usize>,
    pub confidence: BTreeMap<String, usize>,
    pub god_nodes: Vec<GodNode>,
    pub community_count: usize,
    pub community_sizes: Vec<usize>,
    pub cross_community_edges: usize,
    pub cross_links: Vec<CrossLink>,
    pub has_more: bool,
}

/// Map an edge weight to its confidence tier. The adjacency projection
/// `full_graph` reads carries weight but not properties, and Hound encodes the
/// tier in the weight (EXTRACTED=1.0, INFERRED=0.6, AMBIGUOUS=0.3).
fn confidence_tier(weight: Option<f64>) -> &'static str {
    match weight {
        Some(w) if w >= 0.9 => "EXTRACTED",
        Some(w) if w >= 0.5 => "INFERRED",
        Some(_) => "AMBIGUOUS",
        None => "UNTAGGED",
    }
}

/// Fold the graph into report data. `top` caps the god-node and cross-link lists.
#[must_use]
pub fn compute(graph: &FullGraph, top: usize) -> ReportData {
    let pr = crate::analytics::pagerank(graph, 0.85, 100);
    let labels = crate::analytics::communities(graph, 50);
    let (community_count, community_sizes) = crate::analytics::community_summary(&labels);

    let mut degree: HashMap<String, usize> = HashMap::new();
    let mut edges_by_relation: BTreeMap<String, usize> = BTreeMap::new();
    let mut confidence: BTreeMap<String, usize> = BTreeMap::new();
    let mut cross_community_edges = 0usize;
    let mut cross_links: Vec<CrossLink> = Vec::new();

    // Human label per node: function/type name, else file path, else wire id.
    let label_of: HashMap<&str, &str> = graph
        .nodes
        .iter()
        .map(|n| {
            let label = n
                .properties
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| n.properties.get("path").and_then(Value::as_str))
                .unwrap_or(n.id.as_str());
            (n.id.as_str(), label)
        })
        .collect();

    for e in &graph.edges {
        *degree.entry(e.source.clone()).or_default() += 1;
        *degree.entry(e.target.clone()).or_default() += 1;
        *edges_by_relation.entry(e.relation.clone()).or_default() += 1;
        *confidence
            .entry(confidence_tier(e.weight).to_string())
            .or_default() += 1;

        // A "surprising" link ties two communities together.
        if let (Some(cs), Some(ct)) = (labels.get(&e.source), labels.get(&e.target))
            && cs != ct
        {
            cross_community_edges += 1;
            if cross_links.len() < top {
                cross_links.push(CrossLink {
                    from: label_of
                        .get(e.source.as_str())
                        .copied()
                        .unwrap_or("?")
                        .to_string(),
                    relation: e.relation.clone(),
                    to: label_of
                        .get(e.target.as_str())
                        .copied()
                        .unwrap_or("?")
                        .to_string(),
                });
            }
        }
    }

    // Rank by PageRank (degree, then wire id, as deterministic tiebreaks).
    let mut ids: Vec<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    ids.sort_by(|a, b| {
        let pa = pr.get(*a).copied().unwrap_or(0.0);
        let pb = pr.get(*b).copied().unwrap_or(0.0);
        pb.partial_cmp(&pa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                degree
                    .get(*b)
                    .copied()
                    .unwrap_or(0)
                    .cmp(&degree.get(*a).copied().unwrap_or(0))
            })
            .then_with(|| a.cmp(b))
    });
    let god_nodes: Vec<GodNode> = ids
        .iter()
        .take(top)
        .map(|id| GodNode {
            id: (*id).to_string(),
            label: label_of.get(*id).copied().unwrap_or("?").to_string(),
            degree: degree.get(*id).copied().unwrap_or(0),
            pagerank: pr.get(*id).copied().unwrap_or(0.0),
            community: labels.get(*id).cloned().unwrap_or_default(),
        })
        .collect();

    ReportData {
        node_count: graph.stats.node_count,
        edge_count: graph.stats.edge_count,
        nodes_by_type: graph.stats.nodes_by_type.clone(),
        edges_by_relation,
        confidence,
        god_nodes,
        community_count,
        community_sizes,
        cross_community_edges,
        cross_links,
        has_more: graph.has_more,
    }
}

impl ReportData {
    /// The `hound_report` MCP payload.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let god_nodes: Vec<Value> = self
            .god_nodes
            .iter()
            .map(|g| {
                json!({
                    "id": g.id,
                    "label": g.label,
                    "degree": g.degree,
                    "pagerank": g.pagerank,
                    "community": g.community,
                })
            })
            .collect();
        json!({
            "node_count": self.node_count,
            "edge_count": self.edge_count,
            "nodes_by_type": self.nodes_by_type,
            "edges_by_relation": self.edges_by_relation,
            "confidence": self.confidence,
            "god_nodes": god_nodes,
            "communities": {
                "count": self.community_count,
                "sizes": self.community_sizes.iter().take(20).collect::<Vec<_>>(),
                "cross_community_edges": self.cross_community_edges,
            },
            "has_more": self.has_more,
        })
    }

    /// The human-readable `GRAPH_REPORT.md`.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        use std::fmt::Write as _;
        let mut m = String::new();

        let _ = writeln!(m, "# Hound code graph report\n");
        let _ = writeln!(
            m,
            "**{} nodes · {} edges** across **{} communities**.\n",
            self.node_count, self.edge_count, self.community_count
        );
        if self.has_more {
            let _ = writeln!(
                m,
                "> Note: the node set was truncated by a limit; counts reflect the returned subset.\n"
            );
        }

        let _ = writeln!(m, "## Nodes by type\n");
        let _ = writeln!(m, "| type | count |\n|---|---|");
        for (t, c) in &self.nodes_by_type {
            let _ = writeln!(m, "| {t} | {c} |");
        }

        let _ = writeln!(m, "\n## Edges by relation\n");
        let _ = writeln!(m, "| relation | count |\n|---|---|");
        for (r, c) in &self.edges_by_relation {
            let _ = writeln!(m, "| {r} | {c} |");
        }

        let _ = writeln!(m, "\n## Confidence\n");
        let _ = writeln!(
            m,
            "How certain each relationship is — `EXTRACTED` is AST-certain, the rest are inferred.\n"
        );
        let _ = writeln!(m, "| tier | edges |\n|---|---|");
        for (tier, c) in &self.confidence {
            let _ = writeln!(m, "| {tier} | {c} |");
        }

        let _ = writeln!(m, "\n## God nodes\n");
        let _ = writeln!(
            m,
            "The most central symbols by PageRank — the architectural hubs.\n"
        );
        let _ = writeln!(m, "| symbol | PageRank | degree |\n|---|---|---|");
        for g in &self.god_nodes {
            let _ = writeln!(m, "| `{}` | {:.5} | {} |", g.label, g.pagerank, g.degree);
        }

        let _ = writeln!(m, "\n## Communities\n");
        let sizes: Vec<String> = self
            .community_sizes
            .iter()
            .take(10)
            .map(ToString::to_string)
            .collect();
        let _ = writeln!(
            m,
            "{} cohesive groups (label propagation). Largest: {} members.",
            self.community_count,
            sizes.join(", ")
        );
        let _ = writeln!(
            m,
            "{} edges cross a community boundary — the seams that tie clusters together.\n",
            self.cross_community_edges
        );
        if !self.cross_links.is_empty() {
            let _ = writeln!(m, "Surprising cross-cluster links:\n");
            for c in &self.cross_links {
                let _ = writeln!(m, "- `{}` {} `{}`", c.from, c.relation, c.to);
            }
            let _ = writeln!(m);
        }

        if !self.god_nodes.is_empty() {
            let _ = writeln!(m, "## Suggested questions\n");
            // Dedupe by label: distinct symbols can share a name (e.g. two
            // `tool_error`s), and a repeated question reads as a bug.
            let mut seen = std::collections::HashSet::new();
            for g in self
                .god_nodes
                .iter()
                .filter(|g| seen.insert(g.label.as_str()))
                .take(5)
            {
                let _ = writeln!(
                    m,
                    "- What would break if I change `{}`? → `hound_impact target=\"{}\"`",
                    g.label, g.label
                );
            }
        }

        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use allsource_core::prime::types::{FullGraph, GraphEdge, GraphNode, GraphStats};

    fn gnode(id: &str, name: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: "function".to_string(),
            properties: json!({ "name": name }),
            has_vector: false,
            vector_dim: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
    fn gedge(s: &str, t: &str, rel: &str, w: f64) -> GraphEdge {
        GraphEdge {
            source: s.to_string(),
            target: t.to_string(),
            relation: rel.to_string(),
            properties: None,
            weight: Some(w),
            created_at: chrono::Utc::now(),
        }
    }

    fn sample() -> FullGraph {
        let nodes = vec![
            gnode("node:function:a", "alpha"),
            gnode("node:function:b", "beta"),
        ];
        let edges = vec![gedge("node:function:a", "node:function:b", "calls", 0.6)];
        FullGraph {
            nodes,
            edges,
            stats: GraphStats {
                node_count: 2,
                edge_count: 1,
                vector_count: 0,
                nodes_by_type: [("function".to_string(), 2)].into_iter().collect(),
            },
            has_more: false,
        }
    }

    #[test]
    fn json_keeps_the_hound_report_shape() {
        let data = compute(&sample(), 10);
        let j = data.to_json();
        assert_eq!(j["node_count"], 2);
        assert_eq!(j["edge_count"], 1);
        assert!(j["god_nodes"].is_array());
        assert_eq!(j["nodes_by_type"]["function"], 2);
        assert_eq!(j["edges_by_relation"]["calls"], 1);
        assert_eq!(j["confidence"]["INFERRED"], 1);
        assert!(j["communities"]["count"].is_number());
    }

    #[test]
    fn markdown_has_the_expected_sections() {
        let md = compute(&sample(), 10).to_markdown();
        assert!(md.contains("# Hound code graph report"));
        assert!(md.contains("## Nodes by type"));
        assert!(md.contains("## God nodes"));
        assert!(md.contains("## Communities"));
        assert!(md.contains("## Suggested questions"));
        // god-node labels render as the symbol name
        assert!(md.contains("`alpha`") || md.contains("`beta`"));
    }
}
