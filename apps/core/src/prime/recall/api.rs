//! Recall API — `index()` and `context()` methods for the Prime facade.
//!
//! Provides the user-facing agent memory API that combines compressed index,
//! vector search results, and graph context into a single retrieval call.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{
    compressor::IndexCompressor,
    index_builder::{build_heuristic_index, build_raw_summary},
    types::{CompressedIndex, IndexConfig, LlmBackend, RankedMemory, RecallContext},
};
use crate::application::services::projection::Projection;
use crate::prime::projections::{CrossDomainProjection, DomainIndexProjection};
use crate::prime::types::Node;

// =============================================================================
// Query types
// =============================================================================

/// Query parameters for `recall.context()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallContextQuery {
    /// Natural language query string.
    pub query: String,
    /// Agent ID for scoping.
    pub agent_id: Option<String>,
    /// Max number of vector results (default: 5).
    pub top_k: usize,
    /// Time-travel: only consider knowledge that existed at this timestamp.
    pub as_of: Option<DateTime<Utc>>,
    /// Whether to include the compressed index excerpt in the response.
    pub include_index: bool,
    /// Max total tokens in the response (truncate if exceeded).
    pub max_tokens: Option<usize>,
}

impl Default for RecallContextQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            agent_id: None,
            top_k: 5,
            as_of: None,
            include_index: true,
            max_tokens: None,
        }
    }
}

// =============================================================================
// Recall Engine
// =============================================================================

/// Agent memory engine that wraps Prime's projections to provide
/// `index()` and `context()` retrieval methods.
pub struct RecallEngine {
    domain_index: Arc<DomainIndexProjection>,
    cross_domain: Arc<CrossDomainProjection>,
    compressor: IndexCompressor,
}

impl RecallEngine {
    /// Create a new Recall engine with the given index configuration.
    ///
    /// Convenience constructor that creates default projections and an
    /// `IndexCompressor` configured from `IndexConfig`. If `IndexConfig`
    /// has an `llm_endpoint`, an [`OllamaBackend`] is created automatically.
    pub fn new(config: &IndexConfig) -> Self {
        let llm_backend: Option<Box<dyn LlmBackend>> =
            if let Some(ref endpoint) = config.llm_endpoint {
                let model = config
                    .llm_model
                    .clone()
                    .unwrap_or_else(|| "mistral".to_string());
                Some(Box::new(super::ollama::OllamaBackend::new(
                    endpoint.clone(),
                    model,
                )))
            } else {
                None
            };

        Self {
            domain_index: Arc::new(DomainIndexProjection::new()),
            cross_domain: Arc::new(CrossDomainProjection::new()),
            compressor: IndexCompressor::new(
                llm_backend,
                config.refresh_interval_events,
                config.refresh_interval_seconds,
            ),
        }
    }

    /// Create with injected dependencies (for testing and flexibility).
    pub fn with_dependencies(
        domain_index: Arc<DomainIndexProjection>,
        cross_domain: Arc<CrossDomainProjection>,
        compressor: IndexCompressor,
    ) -> Self {
        Self {
            domain_index,
            cross_domain,
            compressor,
        }
    }

    /// Get the current compressed index.
    ///
    /// Generates on first call, cached thereafter (respecting refresh thresholds).
    pub async fn index(&self) -> CompressedIndex {
        let summary = build_raw_summary(&self.domain_index, &self.cross_domain);
        let heuristic = build_heuristic_index(&summary);
        let event_count = summary.total_nodes as u64 + summary.total_edges as u64;

        self.compressor
            .compress(&summary, event_count, &heuristic)
            .await
    }

    /// Combined retrieval: compressed index + vector results + graph context.
    pub async fn context(&self, query: RecallContextQuery) -> RecallContext {
        let mut index_text = String::new();
        let mut token_count = 0usize;

        // Include compressed index if requested
        if query.include_index {
            let idx = self.index().await;
            index_text = idx.markdown;
            token_count += idx.token_count;
        }

        // Truncate if max_tokens is set and exceeded
        if let Some(max) = query.max_tokens
            && token_count > max
        {
            let target_words = max * 10 / 13; // inverse of words * 1.3
            let truncated: String = index_text
                .split_whitespace()
                .take(target_words)
                .collect::<Vec<_>>()
                .join(" ");
            index_text = truncated + "\n...(truncated)";
            token_count = max;
        }

        // TODO: vector search integration (requires prime-vectors feature)
        // For now, return empty vector results
        let vectors: Vec<RankedMemory> = Vec::new();

        // Get related graph nodes from domains matching the query
        let nodes: Vec<Node> = Vec::new();

        RecallContext {
            index: index_text,
            vectors,
            nodes,
            edges: Vec::new(),
            token_count,
        }
    }

    /// Get all known domains.
    pub fn domains(&self) -> Vec<String> {
        self.domain_index.domains()
    }

    /// Get cross-domain links.
    pub fn cross_domain_links(
        &self,
    ) -> Vec<crate::prime::projections::cross_domain::CrossDomainLink> {
        self.cross_domain.cross_domain_links()
    }

    /// Force regeneration of the compressed index.
    pub async fn refresh_index(&self) -> CompressedIndex {
        self.compressor.invalidate_cache();
        self.index().await
    }

    /// Return all projections as trait objects for registration with a Prime/Core instance.
    pub fn projections(&self) -> Vec<Arc<dyn Projection>> {
        vec![
            Arc::clone(&self.domain_index) as Arc<dyn Projection>,
            Arc::clone(&self.cross_domain) as Arc<dyn Projection>,
        ]
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::services::projection::Projection;
    use crate::domain::entities::Event;
    use crate::prime::types::event_types;
    use uuid::Uuid;

    fn make_node(node_id: &str, node_type: &str, domain: &str, name: &str) -> Event {
        Event::reconstruct_from_strings(
            Uuid::new_v4(),
            event_types::NODE_CREATED.to_string(),
            format!("node:{node_type}:{node_id}"),
            "default".to_string(),
            serde_json::json!({
                "node_id": node_id,
                "node_type": node_type,
                "domain": domain,
                "properties": {"name": name}
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
                "edge_id": edge_id,
                "source": source,
                "target": target,
                "relation": relation,
            }),
            Utc::now(),
            None,
            1,
        )
    }

    fn seed_engine() -> RecallEngine {
        let engine = RecallEngine::new(&IndexConfig::default());

        let events = vec![
            make_node("n1", "metric", "revenue", "Q3 Revenue"),
            make_node("n2", "metric", "revenue", "Churn Rate"),
            make_node("n3", "service", "engineering", "Core API"),
            make_node("n4", "feature", "product", "Dark Mode"),
            make_edge("e1", "n1", "n3", "impacts"),
            make_edge("e2", "n4", "n3", "depends_on"),
        ];

        // Process events through the Arc'd projections
        let projections = engine.projections();
        for event in &events {
            for proj in &projections {
                proj.process(event).unwrap();
            }
        }

        engine
    }

    #[tokio::test]
    async fn test_index_returns_compressed_index() {
        let engine = seed_engine();
        let index = engine.index().await;

        assert!(index.markdown.contains("revenue"));
        assert!(index.markdown.contains("engineering"));
        assert!(index.token_count > 0);
        assert!(!index.domains.is_empty());
    }

    #[tokio::test]
    async fn test_context_with_include_index() {
        let engine = seed_engine();
        let query = RecallContextQuery {
            query: "How does revenue relate to engineering?".to_string(),
            include_index: true,
            ..Default::default()
        };

        let ctx = engine.context(query).await;

        assert!(!ctx.index.is_empty());
        assert!(ctx.token_count > 0);
    }

    #[tokio::test]
    async fn test_context_without_index() {
        let engine = seed_engine();
        let query = RecallContextQuery {
            query: "test".to_string(),
            include_index: false,
            ..Default::default()
        };

        let ctx = engine.context(query).await;

        assert!(ctx.index.is_empty());
        assert_eq!(ctx.token_count, 0);
    }

    #[tokio::test]
    async fn test_context_with_max_tokens_truncates() {
        let engine = seed_engine();
        let query = RecallContextQuery {
            query: "test".to_string(),
            include_index: true,
            max_tokens: Some(10),
            ..Default::default()
        };

        let ctx = engine.context(query).await;

        assert!(ctx.token_count <= 10);
        assert!(ctx.index.contains("truncated"));
    }

    #[tokio::test]
    async fn test_refresh_index_regenerates() {
        let engine = seed_engine();

        let idx1 = engine.index().await;
        let idx2 = engine.index().await;
        // Should be cached (same event count)
        assert_eq!(idx1.markdown, idx2.markdown);

        // Force refresh
        let idx3 = engine.refresh_index().await;
        // Should have been regenerated (content may be same but last_updated differs)
        assert!(idx3.last_updated >= idx1.last_updated);
    }

    #[tokio::test]
    async fn test_domains_returns_known_domains() {
        let engine = seed_engine();
        let domains = engine.domains();

        assert!(domains.contains(&"revenue".to_string()));
        assert!(domains.contains(&"engineering".to_string()));
        assert!(domains.contains(&"product".to_string()));
    }

    #[tokio::test]
    async fn test_cross_domain_links_detected() {
        let engine = seed_engine();
        let links = engine.cross_domain_links();

        // revenue->engineering and product->engineering
        assert_eq!(links.len(), 2);
    }

    #[tokio::test]
    async fn test_projections_returns_arc_projections() {
        let engine = RecallEngine::new(&IndexConfig::default());
        let projections = engine.projections();

        assert_eq!(projections.len(), 2);
        // Verify they implement Projection (names should be set)
        let names: Vec<&str> = projections.iter().map(|p| p.name()).collect();
        assert!(names.contains(&"prime.domain_index"));
        assert!(names.contains(&"prime.cross_domain"));
    }

    #[tokio::test]
    async fn test_with_dependencies_constructor() {
        let domain_index = Arc::new(DomainIndexProjection::new());
        let cross_domain = Arc::new(CrossDomainProjection::new());
        let compressor = IndexCompressor::new(None, 100, 300);

        let engine =
            RecallEngine::with_dependencies(domain_index.clone(), cross_domain.clone(), compressor);

        // Process an event through the shared Arc
        let event = make_node("n1", "metric", "revenue", "Q3 Revenue");
        domain_index.process(&event).unwrap();
        cross_domain.process(&event).unwrap();

        let domains = engine.domains();
        assert!(domains.contains(&"revenue".to_string()));
    }

    #[tokio::test]
    async fn test_with_dependencies_custom_llm_backend() {
        use std::future::Future;
        use std::pin::Pin;

        struct MockBackend;

        impl LlmBackend for MockBackend {
            fn generate(
                &self,
                _prompt: &str,
            ) -> Pin<Box<dyn Future<Output = std::result::Result<String, String>> + Send + '_>>
            {
                Box::pin(async { Ok("# Mock Index".to_string()) })
            }
        }

        let domain_index = Arc::new(DomainIndexProjection::new());
        let cross_domain = Arc::new(CrossDomainProjection::new());

        // Inject a custom LLM backend via the compressor
        let compressor = IndexCompressor::new(Some(Box::new(MockBackend)), 100, 300);
        let engine =
            RecallEngine::with_dependencies(domain_index.clone(), cross_domain.clone(), compressor);

        // Seed some data
        let event = make_node("n1", "metric", "revenue", "Q3 Revenue");
        domain_index.process(&event).unwrap();
        cross_domain.process(&event).unwrap();

        let index = engine.index().await;
        assert_eq!(index.markdown, "# Mock Index");
    }
}
