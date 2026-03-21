//! Types for AllSource Recall — the agent memory API built on Prime.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future::Future, pin::Pin};

use crate::prime::types::{Edge, Node};

// =============================================================================
// LLM Backend trait (Open/Closed Principle)
// =============================================================================

/// Trait for LLM backends used by the index compressor.
///
/// Implementations handle the transport and protocol details (Ollama, OpenAI, etc.)
/// while the compressor remains agnostic to the specific backend.
pub trait LlmBackend: Send + Sync {
    fn generate(
        &self,
        prompt: &str,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<String, String>> + Send + '_>>;
}

// =============================================================================
// Recall Context — returned by recall.context()
// =============================================================================

/// Combined retrieval result from compressed index + vectors + graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallContext {
    /// Compressed index excerpt relevant to the query.
    pub index: String,
    /// Ranked vector similarity results.
    pub vectors: Vec<RankedMemory>,
    /// Related graph nodes discovered via traversal.
    pub nodes: Vec<Node>,
    /// Related graph edges.
    pub edges: Vec<Edge>,
    /// Total token count of the context payload.
    pub token_count: usize,
}

/// A memory ranked by hybrid retrieval score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedMemory {
    pub memory_id: String,
    pub fact: String,
    pub domain: String,
    pub confidence: f64,
    pub source: Option<String>,
    pub stored_at: DateTime<Utc>,
    pub relevance_score: f64,
}

// =============================================================================
// Compressed Index
// =============================================================================

/// Auto-generated markdown summary of agent knowledge, organized by domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedIndex {
    /// The generated markdown content.
    pub markdown: String,
    /// Approximate token count of the markdown.
    pub token_count: usize,
    /// Domains present in the index.
    pub domains: Vec<String>,
    /// Cross-references between domains (pairs of domain names).
    pub cross_references: Vec<(String, String)>,
    /// When this index was last regenerated.
    pub last_updated: DateTime<Utc>,
    /// How many events had been processed when this index was generated.
    pub event_count_at_generation: u64,
}

// =============================================================================
// Index Configuration
// =============================================================================

/// Configuration for compressed index generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Maximum token budget for the compressed index (adaptive: 500–2000).
    pub max_tokens: usize,
    /// Optional LLM endpoint for summarization (None = deterministic only).
    pub llm_endpoint: Option<String>,
    /// LLM model name (e.g. "mistral:7b").
    pub llm_model: Option<String>,
    /// Regenerate index every N events.
    pub refresh_interval_events: u64,
    /// Regenerate index every M seconds.
    pub refresh_interval_seconds: u64,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            max_tokens: 1000,
            llm_endpoint: None,
            llm_model: None,
            refresh_interval_events: 100,
            refresh_interval_seconds: 300,
        }
    }
}

// =============================================================================
// Token estimation
// =============================================================================

/// Approximate token count using whitespace heuristic (words * 1.3).
pub fn estimate_tokens(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    (word_count as f64 * 1.3).ceil() as usize
}

// =============================================================================
// Memory Events
// =============================================================================

/// Event type constants for Recall memory operations.
pub mod recall_event_types {
    pub const MEMORY_STORED: &str = "recall.memory.stored";
    pub const MEMORY_REINFORCED: &str = "recall.memory.reinforced";
    pub const MEMORY_FORGOTTEN: &str = "recall.memory.forgotten";
    pub const MEMORY_DECAYED: &str = "recall.memory.decayed";
    pub const INDEX_UPDATED: &str = "recall.index.updated";
}

/// Payload for a stored memory event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPayload {
    pub fact: String,
    pub domain: String,
    pub source: Option<String>,
    pub confidence: f64,
    pub metadata: Option<Value>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_config_defaults() {
        let config = IndexConfig::default();
        assert_eq!(config.max_tokens, 1000);
        assert!(config.llm_endpoint.is_none());
        assert_eq!(config.refresh_interval_events, 100);
        assert_eq!(config.refresh_interval_seconds, 300);
    }

    #[test]
    fn test_compressed_index_serialization() {
        let index = CompressedIndex {
            markdown: "# Knowledge\n## Revenue\n- Q3 churn".to_string(),
            token_count: 42,
            domains: vec!["revenue".to_string(), "engineering".to_string()],
            cross_references: vec![("revenue".to_string(), "engineering".to_string())],
            last_updated: Utc::now(),
            event_count_at_generation: 150,
        };
        let json = serde_json::to_string(&index).unwrap();
        let parsed: CompressedIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.token_count, 42);
        assert_eq!(parsed.domains.len(), 2);
        assert_eq!(parsed.cross_references.len(), 1);
    }

    #[test]
    fn test_memory_payload_serialization() {
        let payload = MemoryPayload {
            fact: "Q3 churn correlated with pricing change".to_string(),
            domain: "revenue".to_string(),
            source: Some("analysis-session-42".to_string()),
            confidence: 0.87,
            metadata: Some(serde_json::json!({"correlation_id": "inc-2026-03"})),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let parsed: MemoryPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.confidence, 0.87);
        assert_eq!(parsed.domain, "revenue");
    }

    #[test]
    fn test_recall_event_type_constants() {
        assert_eq!(recall_event_types::MEMORY_STORED, "recall.memory.stored");
        assert_eq!(
            recall_event_types::MEMORY_FORGOTTEN,
            "recall.memory.forgotten"
        );
        assert_eq!(recall_event_types::INDEX_UPDATED, "recall.index.updated");
    }

    #[test]
    fn test_ranked_memory_ordering() {
        let m1 = RankedMemory {
            memory_id: "m1".to_string(),
            fact: "fact one".to_string(),
            domain: "eng".to_string(),
            confidence: 0.9,
            source: None,
            stored_at: Utc::now(),
            relevance_score: 0.85,
        };
        let m2 = RankedMemory {
            memory_id: "m2".to_string(),
            fact: "fact two".to_string(),
            domain: "eng".to_string(),
            confidence: 0.7,
            source: None,
            stored_at: Utc::now(),
            relevance_score: 0.92,
        };
        // Higher relevance_score should rank first
        assert!(m2.relevance_score > m1.relevance_score);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("one two three"), 4); // 3 * 1.3 = 3.9 → 4
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("Hello world foo bar"), 6); // 4 * 1.3 = 5.2 → 6
    }

    /// Verify the LlmBackend trait is object-safe (can be used as dyn).
    #[test]
    fn test_llm_backend_trait_is_object_safe() {
        // This test verifies at compile time that LlmBackend can be used as a trait object.
        fn _assert_object_safe(_: &dyn LlmBackend) {}
    }
}
