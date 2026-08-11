//! Index builder — free functions for building a compressed markdown index.
//!
//! Reads from the existing [`DomainIndexProjection`] and [`CrossDomainProjection`]
//! projections to produce structured summaries and heuristic markdown indexes.
//! No duplicated domain tracking — the projections are the single source of truth.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Write};

use crate::prime::projections::{CrossDomainProjection, DomainIndexProjection};

// =============================================================================
// Summary types
// =============================================================================

/// Per-domain summary for LLM prompt construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSummary {
    pub domain: String,
    pub node_count: usize,
    /// node_type -> count
    pub node_types: HashMap<String, usize>,
    /// Up to 5 sample entity descriptions for context.
    pub sample_entities: Vec<String>,
    /// Number of edges originating from nodes in this domain.
    pub edge_count: usize,
}

/// Cross-domain relationship summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainSummary {
    pub domain_a: String,
    pub domain_b: String,
    pub relation_types: Vec<String>,
    pub edge_count: usize,
}

/// Full structured summary — input for LLM-based index generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRawSummary {
    pub domains: Vec<DomainSummary>,
    pub cross_domain_links: Vec<CrossDomainSummary>,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub generated_at: DateTime<Utc>,
}

// =============================================================================
// Builder functions
// =============================================================================

/// Build a structured summary from the existing projections.
///
/// Reads domain counts from [`DomainIndexProjection`] and cross-domain
/// links from [`CrossDomainProjection`]. No duplicated state — these
/// projections are the single source of truth.
pub fn build_raw_summary(
    domain_index: &DomainIndexProjection,
    cross_domain: &CrossDomainProjection,
) -> IndexRawSummary {
    let domain_counts = domain_index.domain_counts();

    let domains: Vec<DomainSummary> = domain_counts
        .iter()
        .map(|(domain, count)| {
            // Get the actual node IDs to build sample entities
            let node_ids = domain_index.nodes_in_domain(domain);
            let sample_entities: Vec<String> = node_ids
                .iter()
                .take(5)
                .map(|id| id.as_str().to_string())
                .collect();

            DomainSummary {
                domain: domain.clone(),
                node_count: *count,
                node_types: HashMap::new(), // DomainIndexProjection doesn't track types
                sample_entities,
                edge_count: 0, // edge counts come from cross-domain projection
            }
        })
        .collect();

    let links = cross_domain.cross_domain_links();
    let cross_domain_links: Vec<CrossDomainSummary> = links
        .iter()
        .map(|link| CrossDomainSummary {
            domain_a: link.domain_a.clone(),
            domain_b: link.domain_b.clone(),
            relation_types: link.sample_relations.clone(),
            edge_count: link.edge_count,
        })
        .collect();

    let total_nodes: usize = domains.iter().map(|d| d.node_count).sum();
    let total_edges: usize = cross_domain_links.iter().map(|l| l.edge_count).sum();

    IndexRawSummary {
        domains,
        cross_domain_links,
        total_nodes,
        total_edges,
        generated_at: Utc::now(),
    }
}

/// Generate a heuristic markdown index WITHOUT LLM.
///
/// Produces a structured markdown summary organized by domain with
/// cross-references. Stays under ~1000 tokens for most knowledge bases.
pub fn build_heuristic_index(summary: &IndexRawSummary) -> String {
    let mut md = String::with_capacity(2048);

    // Header
    let _ = writeln!(md, "# Knowledge Index");
    let _ = writeln!(
        md,
        "\n_{} nodes, {} domains, {} cross-domain links_\n",
        summary.total_nodes,
        summary.domains.len(),
        summary.cross_domain_links.len()
    );

    // Domains section
    if !summary.domains.is_empty() {
        let _ = writeln!(md, "## Domains\n");

        let mut sorted_domains = summary.domains.clone();
        sorted_domains.sort_by_key(|a| std::cmp::Reverse(a.node_count));

        for domain in &sorted_domains {
            let _ = writeln!(md, "### {}\n", domain.domain);
            let _ = writeln!(md, "- **Nodes:** {}", domain.node_count);

            if !domain.node_types.is_empty() {
                let types: Vec<String> = domain
                    .node_types
                    .iter()
                    .map(|(t, c)| format!("{t} ({c})"))
                    .collect();
                let _ = writeln!(md, "- **Types:** {}", types.join(", "));
            }

            if !domain.sample_entities.is_empty() {
                let _ = writeln!(md, "- **Examples:** {}", domain.sample_entities.join(", "));
            }

            let _ = writeln!(md);
        }
    }

    // Cross-references section
    if !summary.cross_domain_links.is_empty() {
        let _ = writeln!(md, "## Cross-References\n");

        for link in &summary.cross_domain_links {
            let relations = link.relation_types.join(", ");
            let _ = writeln!(
                md,
                "- **{} ↔ {}**: {} edges ({})",
                link.domain_a, link.domain_b, link.edge_count, relations
            );
        }
        let _ = writeln!(md);
    }

    md
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::services::projection::Projection, domain::entities::Event,
        prime::types::event_types,
    };
    use uuid::Uuid;

    // Canonical add_node/add_edge shape (facade.rs): node id under `id`, domain
    // under `properties.domain`, node referenced by entity_id; edge id under `id`
    // with entity_id source/target. node_domains/domain_index key on these.
    fn make_node(node_id: &str, node_type: &str, domain: &str, name: &str) -> Event {
        Event::reconstruct_from_strings(
            Uuid::new_v4(),
            event_types::NODE_CREATED.to_string(),
            format!("node:{node_type}:{node_id}"),
            "default".to_string(),
            serde_json::json!({
                "id": node_id,
                "node_type": node_type,
                "properties": {"name": name, "domain": domain}
            }),
            Utc::now(),
            None,
            1,
        )
    }

    fn make_edge(edge_id: &str, source: &str, target: &str, relation: &str) -> Event {
        Event::reconstruct_from_strings(
            Uuid::new_v4(),
            event_types::EDGE_CREATED.to_string(),
            format!("edge:{edge_id}"),
            "default".to_string(),
            serde_json::json!({
                "id": edge_id,
                "source": source,
                "target": target,
                "relation": relation,
            }),
            Utc::now(),
            None,
            1,
        )
    }

    fn seed_projections() -> (DomainIndexProjection, CrossDomainProjection) {
        let domain_index = DomainIndexProjection::new();
        let cross_domain = CrossDomainProjection::new();

        let events = vec![
            // Revenue domain
            make_node("n1", "metric", "revenue", "Q3 Revenue"),
            make_node("n2", "metric", "revenue", "Churn Rate"),
            make_node("n3", "decision", "revenue", "Price Change"),
            // Engineering domain
            make_node("n4", "service", "engineering", "Core API"),
            make_node("n5", "service", "engineering", "Query Service"),
            // Product domain
            make_node("n6", "feature", "product", "Dark Mode"),
            // Cross-domain edges. source/target are node entity_ids (matching
            // how add_edge references nodes and how node_domains is keyed).
            make_edge("e1", "node:decision:n3", "node:metric:n1", "impacts"), // same domain (revenue)
            make_edge("e2", "node:decision:n3", "node:service:n4", "requires"), // revenue -> engineering
            make_edge("e3", "node:feature:n6", "node:service:n5", "depends_on"), // product -> engineering
        ];

        for event in &events {
            domain_index.process(event).unwrap();
            cross_domain.process(event).unwrap();
        }

        (domain_index, cross_domain)
    }

    #[test]
    fn test_build_raw_summary_domain_counts() {
        let (domain_index, cross_domain) = seed_projections();
        let summary = build_raw_summary(&domain_index, &cross_domain);

        assert_eq!(summary.domains.len(), 3);
        assert_eq!(summary.total_nodes, 6);

        let revenue = summary
            .domains
            .iter()
            .find(|d| d.domain == "revenue")
            .unwrap();
        assert_eq!(revenue.node_count, 3);
    }

    #[test]
    fn test_build_raw_summary_cross_domain_links() {
        let (domain_index, cross_domain) = seed_projections();
        let summary = build_raw_summary(&domain_index, &cross_domain);

        assert_eq!(summary.cross_domain_links.len(), 2);
    }

    #[test]
    fn test_build_heuristic_index_produces_valid_markdown() {
        let (domain_index, cross_domain) = seed_projections();
        let summary = build_raw_summary(&domain_index, &cross_domain);
        let index = build_heuristic_index(&summary);

        assert!(index.contains("# Knowledge Index"));
        assert!(index.contains("## Domains"));
        assert!(index.contains("## Cross-References"));
        assert!(index.contains("revenue"));
        assert!(index.contains("engineering"));
        assert!(index.contains("product"));
    }

    #[test]
    fn test_build_heuristic_index_shows_cross_references() {
        let (domain_index, cross_domain) = seed_projections();
        let summary = build_raw_summary(&domain_index, &cross_domain);
        let index = build_heuristic_index(&summary);

        assert!(index.contains("↔"));
        assert!(index.contains("requires") || index.contains("depends_on"));
    }

    #[test]
    fn test_build_heuristic_index_under_1000_tokens() {
        let (domain_index, cross_domain) = seed_projections();
        let summary = build_raw_summary(&domain_index, &cross_domain);
        let index = build_heuristic_index(&summary);
        let tokens = crate::prime::recall::types::estimate_tokens(&index);

        assert!(
            tokens < 1000,
            "Index should be under 1000 tokens, got {tokens}"
        );
    }

    #[test]
    fn test_sample_entities_from_domain_index() {
        let (domain_index, cross_domain) = seed_projections();
        let summary = build_raw_summary(&domain_index, &cross_domain);

        let revenue = summary
            .domains
            .iter()
            .find(|d| d.domain == "revenue")
            .unwrap();
        // Sample entities come from node IDs (DomainIndexProjection tracks node IDs, not names)
        assert!(!revenue.sample_entities.is_empty());
        assert!(revenue.sample_entities.len() <= 5);
    }
}
