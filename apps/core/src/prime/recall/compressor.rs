//! LLM-assisted index compression.
//!
//! Takes the [`IndexRawSummary`] from the index builder and produces
//! a natural-language compressed index via an [`LlmBackend`] implementation.
//! Falls back to the heuristic index when no LLM backend is configured or
//! when the backend errors.

use chrono::Utc;
use std::sync::Mutex;
use std::time::Instant;

use super::index_builder::IndexRawSummary;
use super::types::{CompressedIndex, LlmBackend, estimate_tokens};

// =============================================================================
// LLM Compressor
// =============================================================================

/// Generates compressed markdown indexes from structured knowledge summaries.
///
/// Accepts an optional [`LlmBackend`] for LLM-assisted compression. When no
/// backend is provided (or when it errors), falls back to the heuristic index.
pub struct IndexCompressor {
    /// Optional LLM backend for summarization.
    llm_backend: Option<Box<dyn LlmBackend>>,
    /// Refresh thresholds.
    refresh_interval_events: u64,
    refresh_interval_seconds: u64,
    /// Cached index to avoid regenerating on every call.
    cache: Mutex<Option<CachedIndex>>,
}

struct CachedIndex {
    index: CompressedIndex,
    event_count_at_generation: u64,
    generated_at: Instant,
}

impl IndexCompressor {
    /// Create a compressor with an optional LLM backend.
    pub fn new(
        llm_backend: Option<Box<dyn LlmBackend>>,
        refresh_interval_events: u64,
        refresh_interval_seconds: u64,
    ) -> Self {
        Self {
            llm_backend,
            refresh_interval_events,
            refresh_interval_seconds,
            cache: Mutex::new(None),
        }
    }

    /// Generate a compressed index from raw summary data.
    ///
    /// Returns cached result if within refresh thresholds. Falls back to
    /// heuristic generation if LLM is unavailable.
    pub async fn compress(
        &self,
        summary: &IndexRawSummary,
        current_event_count: u64,
        heuristic_fallback: &str,
    ) -> CompressedIndex {
        // Check cache
        if let Some(cached) = self.get_cached(current_event_count) {
            return cached;
        }

        // Try LLM compression
        let markdown = if let Some(ref backend) = self.llm_backend {
            let prompt = Self::build_prompt(summary);
            match backend.generate(&prompt).await {
                Ok(text) => text,
                Err(e) => {
                    tracing::warn!("LLM index compression failed, using heuristic: {e}");
                    heuristic_fallback.to_string()
                }
            }
        } else {
            heuristic_fallback.to_string()
        };

        let domains: Vec<String> = summary.domains.iter().map(|d| d.domain.clone()).collect();
        let cross_references: Vec<(String, String)> = summary
            .cross_domain_links
            .iter()
            .map(|l| (l.domain_a.clone(), l.domain_b.clone()))
            .collect();
        let token_count = estimate_tokens(&markdown);

        let index = CompressedIndex {
            markdown,
            token_count,
            domains,
            cross_references,
            last_updated: Utc::now(),
            event_count_at_generation: current_event_count,
        };

        // Cache the result
        self.update_cache(&index, current_event_count);

        index
    }

    /// Check if a cached index is still valid.
    fn get_cached(&self, current_event_count: u64) -> Option<CompressedIndex> {
        let guard = self.cache.lock().ok()?;
        let cached = guard.as_ref()?;

        let events_since = current_event_count.saturating_sub(cached.event_count_at_generation);
        let seconds_since = cached.generated_at.elapsed().as_secs();

        if events_since < self.refresh_interval_events
            && seconds_since < self.refresh_interval_seconds
        {
            Some(cached.index.clone())
        } else {
            None
        }
    }

    /// Update the cache with a new index.
    fn update_cache(&self, index: &CompressedIndex, event_count: u64) {
        if let Ok(mut guard) = self.cache.lock() {
            *guard = Some(CachedIndex {
                index: index.clone(),
                event_count_at_generation: event_count,
                generated_at: Instant::now(),
            });
        }
    }

    /// Build the LLM prompt from structured data.
    pub(crate) fn build_prompt(summary: &IndexRawSummary) -> String {
        let mut prompt = String::with_capacity(2048);

        prompt.push_str(
            "Generate a compressed knowledge index in markdown. \
             The index should be a concise summary (~800 tokens) that an AI agent \
             can use as navigational scaffolding to find relevant knowledge. \
             Focus on cross-domain relationships — how different knowledge areas connect.\n\n",
        );

        prompt.push_str("## Input Data\n\n");

        // Domains
        prompt.push_str("### Domains\n");
        for domain in &summary.domains {
            prompt.push_str(&format!(
                "- **{}**: {} nodes",
                domain.domain, domain.node_count
            ));
            if !domain.node_types.is_empty() {
                let types: Vec<String> = domain
                    .node_types
                    .iter()
                    .map(|(t, c)| format!("{t}({c})"))
                    .collect();
                prompt.push_str(&format!(" [{}]", types.join(", ")));
            }
            if !domain.sample_entities.is_empty() {
                prompt.push_str(&format!(
                    " — e.g. {}",
                    domain.sample_entities.join(", ")
                ));
            }
            prompt.push('\n');
        }

        // Cross-domain links
        if !summary.cross_domain_links.is_empty() {
            prompt.push_str("\n### Cross-Domain Relationships\n");
            for link in &summary.cross_domain_links {
                prompt.push_str(&format!(
                    "- {} ↔ {}: {} edges via {}\n",
                    link.domain_a,
                    link.domain_b,
                    link.edge_count,
                    link.relation_types.join(", ")
                ));
            }
        }

        prompt.push_str(
            "\n## Instructions\n\
             Output a markdown document with:\n\
             1. A `# Knowledge Index` header\n\
             2. One section per domain with key facts and entity types\n\
             3. A `## Cross-References` section highlighting how domains connect\n\
             4. Keep under 800 tokens. Be concise — bullet points, not paragraphs.\n\
             5. Output ONLY the markdown, no explanation.\n",
        );

        prompt
    }

    /// Invalidate the cache, forcing regeneration on next call.
    pub fn invalidate_cache(&self) {
        if let Ok(mut guard) = self.cache.lock() {
            *guard = None;
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prime::recall::index_builder::{CrossDomainSummary, DomainSummary};
    use std::collections::HashMap;

    fn make_summary() -> IndexRawSummary {
        IndexRawSummary {
            domains: vec![
                DomainSummary {
                    domain: "revenue".to_string(),
                    node_count: 5,
                    node_types: {
                        let mut m = HashMap::new();
                        m.insert("metric".to_string(), 3);
                        m.insert("decision".to_string(), 2);
                        m
                    },
                    sample_entities: vec!["Q3 Revenue".to_string(), "Churn Rate".to_string()],
                    edge_count: 3,
                },
                DomainSummary {
                    domain: "engineering".to_string(),
                    node_count: 4,
                    node_types: {
                        let mut m = HashMap::new();
                        m.insert("service".to_string(), 4);
                        m
                    },
                    sample_entities: vec!["Core API".to_string()],
                    edge_count: 2,
                },
            ],
            cross_domain_links: vec![CrossDomainSummary {
                domain_a: "engineering".to_string(),
                domain_b: "revenue".to_string(),
                relation_types: vec!["impacts".to_string(), "requires".to_string()],
                edge_count: 3,
            }],
            total_nodes: 9,
            total_edges: 5,
            generated_at: Utc::now(),
        }
    }

    #[test]
    fn test_build_prompt_contains_domains() {
        let summary = make_summary();
        let prompt = IndexCompressor::build_prompt(&summary);

        assert!(prompt.contains("revenue"));
        assert!(prompt.contains("engineering"));
        assert!(prompt.contains("Cross-Domain"));
        assert!(prompt.contains("impacts"));
    }

    #[test]
    fn test_build_prompt_contains_sample_entities() {
        let summary = make_summary();
        let prompt = IndexCompressor::build_prompt(&summary);

        assert!(prompt.contains("Q3 Revenue"));
        assert!(prompt.contains("Core API"));
    }

    #[tokio::test]
    async fn test_fallback_to_heuristic_when_no_llm_configured() {
        let compressor = IndexCompressor::new(None, 100, 300);
        let summary = make_summary();
        let heuristic = "# Heuristic Index\n\nFallback content";

        let result = compressor.compress(&summary, 100, heuristic).await;

        assert_eq!(result.markdown, heuristic);
        assert_eq!(result.event_count_at_generation, 100);
    }

    #[tokio::test]
    async fn test_fallback_when_llm_unavailable() {
        use crate::prime::recall::ollama::OllamaBackend;

        let backend = OllamaBackend::new(
            "http://localhost:99999/api/generate".to_string(),
            "mistral".to_string(),
        );
        let compressor = IndexCompressor::new(Some(Box::new(backend)), 100, 300);
        let summary = make_summary();
        let heuristic = "# Heuristic Fallback";

        let result = compressor.compress(&summary, 50, heuristic).await;

        // Should fall back to heuristic since port 99999 is unreachable
        assert_eq!(result.markdown, heuristic);
    }

    #[tokio::test]
    async fn test_caching_prevents_regeneration() {
        let compressor = IndexCompressor::new(None, 1000, 300);
        let summary = make_summary();

        // First call generates
        let result1 = compressor.compress(&summary, 100, "# First").await;
        assert_eq!(result1.markdown, "# First");

        // Second call with different heuristic but within threshold — should return cached
        let result2 = compressor.compress(&summary, 150, "# Second").await;
        assert_eq!(result2.markdown, "# First"); // cached
    }

    #[tokio::test]
    async fn test_cache_invalidated_by_event_threshold() {
        let compressor = IndexCompressor::new(None, 10, 3600); // low event threshold
        let summary = make_summary();

        let result1 = compressor.compress(&summary, 100, "# First").await;
        assert_eq!(result1.markdown, "# First");

        // Exceed event threshold
        let result2 = compressor.compress(&summary, 200, "# Second").await;
        assert_eq!(result2.markdown, "# Second"); // regenerated
    }

    #[test]
    fn test_invalidate_cache() {
        let compressor = IndexCompressor::new(None, 100, 300);

        // Manually populate cache
        compressor.update_cache(
            &CompressedIndex {
                markdown: "cached".to_string(),
                token_count: 1,
                domains: vec![],
                cross_references: vec![],
                last_updated: Utc::now(),
                event_count_at_generation: 100,
            },
            100,
        );

        assert!(compressor.get_cached(100).is_some());
        compressor.invalidate_cache();
        assert!(compressor.get_cached(100).is_none());
    }

    /// Test that a custom LlmBackend can be injected.
    #[tokio::test]
    async fn test_custom_llm_backend_injection() {
        use std::future::Future;
        use std::pin::Pin;

        struct MockBackend;

        impl LlmBackend for MockBackend {
            fn generate(
                &self,
                _prompt: &str,
            ) -> Pin<Box<dyn Future<Output = std::result::Result<String, String>> + Send + '_>>
            {
                Box::pin(async { Ok("# Mock LLM Index\n\n- mock result".to_string()) })
            }
        }

        let compressor = IndexCompressor::new(Some(Box::new(MockBackend)), 100, 300);
        let summary = make_summary();

        let result = compressor
            .compress(&summary, 100, "# Heuristic Fallback")
            .await;

        assert_eq!(result.markdown, "# Mock LLM Index\n\n- mock result");
    }
}
