//! Export the Hound code graph to interchange formats — the Graphify export
//! parity: Cypher (`Neo4j`/`FalkorDB`), `GraphML` (Gephi/`yEd`), Mermaid (docs), and an
//! Obsidian / wiki markdown vault. All pure functions over a [`FullGraph`].

use std::collections::HashMap;
use std::fmt::Write as _;

use allsource_core::prime::types::FullGraph;
use serde_json::Value;

/// Human label for a node: name / path / wire id.
fn label_of(props: &Value, id: &str) -> String {
    props
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| props.get("path").and_then(Value::as_str))
        .unwrap_or(id)
        .to_string()
}

/// Filesystem / anchor-safe slug.
fn slug(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn cypher_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Scalar properties (string/number/bool) as `(key, string-value)` pairs, sorted.
fn scalar_props(props: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(map) = props.as_object() {
        for (k, v) in map {
            let val = match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => continue,
            };
            out.push((k.clone(), val));
        }
    }
    out.sort();
    out
}

/// Per-node degree from the edge set (used to cap Mermaid to the busiest nodes).
fn degrees(g: &FullGraph) -> HashMap<&str, usize> {
    let mut d: HashMap<&str, usize> = HashMap::new();
    for e in &g.edges {
        *d.entry(e.source.as_str()).or_default() += 1;
        *d.entry(e.target.as_str()).or_default() += 1;
    }
    d
}

/// Cypher `CREATE` statements for `Neo4j` / `FalkorDB` import.
#[must_use]
pub fn to_cypher(g: &FullGraph) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "// Prime Hound export — {} nodes, {} edges",
        g.nodes.len(),
        g.edges.len()
    );
    for n in &g.nodes {
        let mut props = format!("id: \"{}\"", cypher_escape(&n.id));
        for (k, v) in scalar_props(&n.properties) {
            let _ = write!(props, ", {}: \"{}\"", slug(&k).replace('-', "_"), cypher_escape(&v));
        }
        let _ = writeln!(s, "CREATE (:`{}` {{{props}}});", n.node_type);
    }
    for e in &g.edges {
        let w = e.weight.map_or(String::new(), |w| format!(" {{weight: {w}}}"));
        let _ = writeln!(
            s,
            "MATCH (a {{id: \"{}\"}}), (b {{id: \"{}\"}}) CREATE (a)-[:`{}`{w}]->(b);",
            cypher_escape(&e.source),
            cypher_escape(&e.target),
            e.relation
        );
    }
    s
}

/// `GraphML` (Gephi / `yEd`). Node `label` + edge `relation` are exposed as keys.
#[must_use]
pub fn to_graphml(g: &FullGraph) -> String {
    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str("<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">\n");
    s.push_str("  <key id=\"label\" for=\"node\" attr.name=\"label\" attr.type=\"string\"/>\n");
    s.push_str("  <key id=\"type\" for=\"node\" attr.name=\"type\" attr.type=\"string\"/>\n");
    s.push_str("  <key id=\"relation\" for=\"edge\" attr.name=\"relation\" attr.type=\"string\"/>\n");
    s.push_str("  <graph edgedefault=\"directed\">\n");
    for n in &g.nodes {
        let label = xml_escape(&label_of(&n.properties, &n.id));
        let _ = writeln!(
            s,
            "    <node id=\"{}\"><data key=\"label\">{label}</data><data key=\"type\">{}</data></node>",
            xml_escape(&n.id),
            xml_escape(&n.node_type)
        );
    }
    for (i, e) in g.edges.iter().enumerate() {
        let _ = writeln!(
            s,
            "    <edge id=\"e{i}\" source=\"{}\" target=\"{}\"><data key=\"relation\">{}</data></edge>",
            xml_escape(&e.source),
            xml_escape(&e.target),
            xml_escape(&e.relation)
        );
    }
    s.push_str("  </graph>\n</graphml>\n");
    s
}

/// Mermaid `graph` for docs. Capped to the `max_nodes` busiest nodes (and the
/// edges among them) so the diagram stays renderable.
#[must_use]
pub fn to_mermaid(g: &FullGraph, max_nodes: usize) -> String {
    let deg = degrees(g);
    let mut ids: Vec<&str> = g.nodes.iter().map(|n| n.id.as_str()).collect();
    ids.sort_by(|a, b| {
        deg.get(b).copied().unwrap_or(0).cmp(&deg.get(a).copied().unwrap_or(0)).then(a.cmp(b))
    });
    let kept: HashMap<&str, usize> = ids.iter().take(max_nodes).enumerate().map(|(i, id)| (*id, i)).collect();
    let label: HashMap<&str, String> =
        g.nodes.iter().map(|n| (n.id.as_str(), label_of(&n.properties, &n.id))).collect();

    let mut s = String::from("graph TD\n");
    for (id, i) in &kept {
        let lbl = label.get(id).cloned().unwrap_or_default().replace('"', "'");
        let _ = writeln!(s, "  n{i}[\"{lbl}\"]");
    }
    for e in &g.edges {
        if let (Some(&a), Some(&b)) = (kept.get(e.source.as_str()), kept.get(e.target.as_str())) {
            let _ = writeln!(s, "  n{a} -->|{}| n{b}", e.relation);
        }
    }
    if g.nodes.len() > max_nodes {
        let _ = writeln!(s, "  %% showing the {max_nodes} busiest of {} nodes", g.nodes.len());
    }
    s
}

/// An Obsidian vault: one markdown note per node, linked to its neighbors with
/// `[[wikilinks]]`. Returns `(relative path, contents)` pairs.
#[must_use]
pub fn to_obsidian(g: &FullGraph) -> Vec<(String, String)> {
    let label: HashMap<&str, String> =
        g.nodes.iter().map(|n| (n.id.as_str(), label_of(&n.properties, &n.id))).collect();
    // Unique note slug per node id (disambiguate same-label nodes by a suffix).
    let mut note: HashMap<&str, String> = HashMap::new();
    let mut used: HashMap<String, usize> = HashMap::new();
    for n in &g.nodes {
        let base = slug(label.get(n.id.as_str()).map_or("node", |l| l.as_str()));
        let base = if base.is_empty() { slug(&n.id) } else { base };
        let count = used.entry(base.clone()).or_default();
        let name = if *count == 0 { base.clone() } else { format!("{base}-{count}") };
        *count += 1;
        note.insert(n.id.as_str(), name);
    }

    let mut out = Vec::new();
    for n in &g.nodes {
        let id = n.id.as_str();
        let mut body = format!("# {}\n\n", label.get(id).cloned().unwrap_or_default());
        let _ = writeln!(body, "- **type:** {}", n.node_type);
        for (k, v) in scalar_props(&n.properties) {
            let _ = writeln!(body, "- **{k}:** {v}");
        }
        // Outgoing then incoming neighbours.
        let mut links = String::new();
        for e in &g.edges {
            if e.source == *id && let Some(t) = note.get(e.target.as_str()) {
                let _ = writeln!(links, "- {} → [[{t}]]", e.relation);
            } else if e.target == *id && let Some(s2) = note.get(e.source.as_str()) {
                let _ = writeln!(links, "- [[{s2}]] → {} (this)", e.relation);
            }
        }
        if !links.is_empty() {
            let _ = write!(body, "\n## Links\n{links}");
        }
        out.push((format!("{}.md", note[id]), body));
    }
    out
}

/// A navigable markdown wiki: an `index.md` grouping nodes by type with links to
/// per-node pages under `pages/`. Returns `(relative path, contents)` pairs.
#[must_use]
pub fn to_wiki(g: &FullGraph) -> Vec<(String, String)> {
    let pages = to_obsidian(g); // reuse the per-node pages, place them under pages/
    let label: HashMap<&str, String> =
        g.nodes.iter().map(|n| (n.id.as_str(), label_of(&n.properties, &n.id))).collect();
    // Map node → its page file (same order/slugging as to_obsidian).
    let mut by_type: std::collections::BTreeMap<&str, Vec<(String, String)>> =
        std::collections::BTreeMap::new();
    for (n, (page, _)) in g.nodes.iter().zip(pages.iter()) {
        by_type
            .entry(n.node_type.as_str())
            .or_default()
            .push((label.get(n.id.as_str()).cloned().unwrap_or_default(), page.clone()));
    }

    let mut index = format!("# Code graph wiki\n\n{} nodes, {} edges.\n", g.nodes.len(), g.edges.len());
    for (ty, mut items) in by_type {
        items.sort();
        let _ = write!(index, "\n## {ty} ({})\n\n", items.len());
        for (lbl, page) in items {
            let _ = writeln!(index, "- [{lbl}](pages/{page})");
        }
    }

    let mut out = vec![("index.md".to_string(), index)];
    out.extend(pages.into_iter().map(|(p, c)| (format!("pages/{p}"), c)));
    out
}

/// Render a single-file format. Returns `None` for an unknown or multi-file key.
#[must_use]
pub fn render_text(g: &FullGraph, format: &str, mermaid_max: usize) -> Option<String> {
    match format.to_ascii_lowercase().as_str() {
        "cypher" => Some(to_cypher(g)),
        "graphml" => Some(to_graphml(g)),
        "mermaid" => Some(to_mermaid(g, mermaid_max)),
        _ => None,
    }
}

/// Render a multi-file format. Returns `None` for an unknown or single-file key.
#[must_use]
pub fn render_files(g: &FullGraph, format: &str) -> Option<Vec<(String, String)>> {
    match format.to_ascii_lowercase().as_str() {
        "obsidian" => Some(to_obsidian(g)),
        "wiki" => Some(to_wiki(g)),
        _ => None,
    }
}

/// Comma-separated supported format keys (for help / errors).
#[must_use]
pub fn format_keys() -> &'static str {
    "cypher, graphml, mermaid, obsidian, wiki"
}

#[cfg(test)]
mod tests {
    use super::*;
    use allsource_core::prime::types::{GraphEdge, GraphNode, GraphStats};
    use serde_json::json;

    fn node(id: &str, name: &str, ty: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: ty.to_string(),
            properties: json!({ "name": name, "file": "a.rs" }),
            has_vector: false,
            vector_dim: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
    fn edge(s: &str, t: &str, rel: &str) -> GraphEdge {
        GraphEdge {
            source: s.to_string(),
            target: t.to_string(),
            relation: rel.to_string(),
            properties: None,
            weight: Some(0.6),
            created_at: chrono::Utc::now(),
        }
    }
    fn g() -> FullGraph {
        FullGraph {
            nodes: vec![
                node("node:function:a", "alpha", "function"),
                node("node:function:b", "beta", "function"),
            ],
            edges: vec![edge("node:function:a", "node:function:b", "calls")],
            stats: GraphStats::default(),
            has_more: false,
        }
    }

    #[test]
    fn cypher_has_nodes_and_a_relationship() {
        let c = to_cypher(&g());
        assert!(c.contains("CREATE (:`function`"));
        assert!(c.contains("name: \"alpha\""));
        assert!(c.contains("-[:`calls`"));
    }

    #[test]
    fn graphml_is_wellformed_ish() {
        let x = to_graphml(&g());
        assert!(x.starts_with("<?xml"));
        assert!(x.contains("<node id=\"node:function:a\">"));
        assert!(x.contains("<edge id=\"e0\""));
        assert!(x.trim_end().ends_with("</graphml>"));
    }

    #[test]
    fn mermaid_links_alpha_to_beta() {
        let m = to_mermaid(&g(), 10);
        assert!(m.starts_with("graph TD"));
        assert!(m.contains("[\"alpha\"]"));
        assert!(m.contains("-->|calls|"));
    }

    #[test]
    fn obsidian_writes_a_note_per_node_with_wikilinks() {
        let files = to_obsidian(&g());
        assert_eq!(files.len(), 2);
        let alpha = files.iter().find(|(p, _)| p == "alpha.md").unwrap();
        assert!(alpha.1.contains("# alpha"));
        assert!(alpha.1.contains("[[beta]]"));
    }

    #[test]
    fn wiki_has_an_index_and_pages() {
        let files = to_wiki(&g());
        assert!(files.iter().any(|(p, _)| p == "index.md"));
        assert!(files.iter().any(|(p, _)| p == "pages/alpha.md"));
        let index = &files.iter().find(|(p, _)| p == "index.md").unwrap().1;
        assert!(index.contains("[alpha](pages/alpha.md)"));
    }

    #[test]
    fn dispatch_helpers_route_correctly() {
        assert!(render_text(&g(), "cypher", 10).is_some());
        assert!(render_text(&g(), "obsidian", 10).is_none());
        assert!(render_files(&g(), "wiki").is_some());
        assert!(render_text(&g(), "nope", 10).is_none());
    }
}
