//! Vector-specific types and event schema for AllSource Prime.
//!
//! Embeddings are stored as events in the WAL. The vector itself goes in event
//! metadata (keeping the payload human-readable), while text, dimensions, and
//! user metadata live in the payload.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// =============================================================================
// Vector Types
// =============================================================================

/// A stored vector embedding with optional source text and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    /// Unique identifier for this vector.
    pub id: String,
    /// Optional source text that was embedded.
    pub text: Option<String>,
    /// Dimensionality of the embedding vector.
    pub dimensions: usize,
    /// User-provided metadata (labels, tags, etc.).
    pub metadata: Option<Value>,
}

/// A result from a vector similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    /// ID of the matched vector.
    pub id: String,
    /// Similarity score (0.0–1.0 for cosine similarity).
    pub score: f64,
    /// Source text of the matched vector, if stored.
    pub text: Option<String>,
    /// User-provided metadata of the matched vector.
    pub metadata: Option<Value>,
}

// =============================================================================
// Event Type Constants
// =============================================================================

/// Event types for vector operations.
pub mod event_types {
    pub const VECTOR_STORED: &str = "prime.vector.stored";
    pub const VECTOR_DELETED: &str = "prime.vector.deleted";
}

// =============================================================================
// Entity ID Formatting
// =============================================================================

/// Generates the entity ID for a vector event: `vec:{id}`
pub fn vector_entity_id(id: &str) -> String {
    format!("vec:{id}")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_entry_serialization() {
        let entry = VectorEntry {
            id: "doc-42".to_string(),
            text: Some("CRDTs enable conflict-free replication".to_string()),
            dimensions: 384,
            metadata: Some(serde_json::json!({"source": "paper", "year": 2011})),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: VectorEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "doc-42");
        assert_eq!(parsed.dimensions, 384);
        assert_eq!(
            parsed.text.as_deref(),
            Some("CRDTs enable conflict-free replication")
        );
        assert_eq!(parsed.metadata.unwrap()["source"], "paper");
    }

    #[test]
    fn test_vector_entry_without_optional_fields() {
        let entry = VectorEntry {
            id: "vec-1".to_string(),
            text: None,
            dimensions: 1536,
            metadata: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: VectorEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "vec-1");
        assert_eq!(parsed.dimensions, 1536);
        assert!(parsed.text.is_none());
        assert!(parsed.metadata.is_none());
    }

    #[test]
    fn test_vector_search_result_serialization() {
        let result = VectorSearchResult {
            id: "doc-17".to_string(),
            score: 0.92,
            text: Some("Event sourcing is an append-only pattern".to_string()),
            metadata: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: VectorSearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "doc-17");
        assert!((parsed.score - 0.92).abs() < f64::EPSILON);
        assert_eq!(
            parsed.text.as_deref(),
            Some("Event sourcing is an append-only pattern")
        );
    }

    #[test]
    fn test_vector_entity_id_format() {
        assert_eq!(vector_entity_id("doc-42"), "vec:doc-42");
        assert_eq!(vector_entity_id("embedding-xyz"), "vec:embedding-xyz");
    }

    #[test]
    fn test_event_type_constants() {
        assert_eq!(event_types::VECTOR_STORED, "prime.vector.stored");
        assert_eq!(event_types::VECTOR_DELETED, "prime.vector.deleted");
    }

    #[test]
    fn test_vector_search_result_ordering() {
        let mut results = vec![
            VectorSearchResult {
                id: "a".into(),
                score: 0.7,
                text: None,
                metadata: None,
            },
            VectorSearchResult {
                id: "b".into(),
                score: 0.95,
                text: None,
                metadata: None,
            },
            VectorSearchResult {
                id: "c".into(),
                score: 0.85,
                text: None,
                metadata: None,
            },
        ];
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        assert_eq!(results[0].id, "b");
        assert_eq!(results[1].id, "c");
        assert_eq!(results[2].id, "a");
    }
}
