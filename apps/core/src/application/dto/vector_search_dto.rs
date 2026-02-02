use crate::application::dto::EventDto;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to store an event embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreEmbeddingRequest {
    /// The event ID to associate with this embedding
    pub event_id: Uuid,
    /// The tenant ID for multi-tenant isolation
    pub tenant_id: String,
    /// The embedding vector (list of floats)
    pub embedding: Vec<f32>,
    /// Optional source text that was embedded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
}

/// Request to store multiple embeddings in batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreBatchEmbeddingsRequest {
    /// List of embeddings to store
    pub embeddings: Vec<StoreEmbeddingRequest>,
}

/// Response from storing an embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreEmbeddingResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// The event ID that was indexed
    pub event_id: Uuid,
}

/// Response from batch storing embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreBatchEmbeddingsResponse {
    /// Number of embeddings successfully indexed
    pub indexed: usize,
    /// Number of embeddings that failed to index
    pub failed: usize,
    /// Error messages for failed embeddings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Request for semantic similarity search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchRequest {
    /// The query embedding vector
    pub query_embedding: Vec<f32>,
    /// Number of results to return (default: 10, max: 100)
    #[serde(default = "default_k")]
    pub k: usize,
    /// Optional tenant ID filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Optional event type filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    /// Minimum similarity threshold (for cosine/dot product metrics)
    /// Only results with similarity >= this value will be returned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_similarity: Option<f32>,
    /// Maximum distance threshold (for euclidean metric)
    /// Only results with distance <= this value will be returned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_distance: Option<f32>,
    /// Distance metric to use: "cosine", "euclidean", or "dot_product"
    /// Default: "cosine"
    #[serde(default = "default_metric")]
    pub metric: String,
    /// Whether to include full event data in results
    #[serde(default)]
    pub include_events: bool,
}

fn default_k() -> usize {
    10
}

fn default_metric() -> String {
    "cosine".to_string()
}

/// A single result from vector search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResultItem {
    /// The event ID that matched
    pub event_id: Uuid,
    /// The similarity or distance score
    /// For cosine/dot_product: higher is more similar (range: -1 to 1)
    /// For euclidean: lower is more similar (range: 0 to inf)
    pub score: f32,
    /// The source text (if available and requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    /// The full event (if requested via include_events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<EventDto>,
}

/// Response from vector search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResponse {
    /// The search results, ordered by relevance
    pub results: Vec<VectorSearchResultItem>,
    /// Total number of results returned
    pub count: usize,
    /// The metric used for scoring
    pub metric: String,
    /// Search execution statistics
    pub stats: VectorSearchStats,
}

/// Statistics about the search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchStats {
    /// Total number of vectors in the index
    pub total_vectors: usize,
    /// Number of vectors actually compared
    pub vectors_searched: usize,
    /// Search execution time in microseconds
    pub search_time_us: u64,
}

/// Request to find events similar to a given event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSimilarEventsRequest {
    /// The event ID to find similar events for
    pub event_id: Uuid,
    /// Number of similar events to return (default: 10)
    #[serde(default = "default_k")]
    pub k: usize,
    /// Optional tenant filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Response from finding similar events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSimilarEventsResponse {
    /// The source event ID
    pub source_event_id: Uuid,
    /// Similar events found
    pub similar_events: Vec<VectorSearchResultItem>,
    /// Total count of similar events
    pub count: usize,
}

/// Request to get embedding for an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEmbeddingRequest {
    /// The event ID to get embedding for
    pub event_id: Uuid,
}

/// Response from getting an embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEmbeddingResponse {
    /// The event ID
    pub event_id: Uuid,
    /// The embedding vector (if found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// The source text (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    /// Number of dimensions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
}

/// Request to delete an embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteEmbeddingRequest {
    /// The event ID whose embedding should be deleted
    pub event_id: Uuid,
}

/// Response from deleting an embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteEmbeddingResponse {
    /// Whether an embedding was actually deleted
    pub deleted: bool,
    /// The event ID that was processed
    pub event_id: Uuid,
}

/// Statistics about the vector index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexStats {
    /// Total number of indexed vectors
    pub total_vectors: usize,
    /// Dimensionality of vectors (if any are stored)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<usize>,
    /// Number of tenants with vectors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_count: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_embedding_request_serialization() {
        let request = StoreEmbeddingRequest {
            event_id: Uuid::new_v4(),
            tenant_id: "tenant-1".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            source_text: Some("test content".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: StoreEmbeddingRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.event_id, deserialized.event_id);
        assert_eq!(request.tenant_id, deserialized.tenant_id);
        assert_eq!(request.embedding, deserialized.embedding);
        assert_eq!(request.source_text, deserialized.source_text);
    }

    #[test]
    fn test_vector_search_request_defaults() {
        let json = r#"{
            "query_embedding": [0.1, 0.2, 0.3]
        }"#;

        let request: VectorSearchRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.k, 10);
        assert_eq!(request.metric, "cosine");
        assert!(!request.include_events);
        assert!(request.tenant_id.is_none());
    }

    #[test]
    fn test_vector_search_response_serialization() {
        let response = VectorSearchResponse {
            results: vec![VectorSearchResultItem {
                event_id: Uuid::new_v4(),
                score: 0.95,
                source_text: Some("matched text".to_string()),
                event: None,
            }],
            count: 1,
            metric: "cosine".to_string(),
            stats: VectorSearchStats {
                total_vectors: 1000,
                vectors_searched: 1000,
                search_time_us: 150,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: VectorSearchResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.count, deserialized.count);
        assert_eq!(response.metric, deserialized.metric);
        assert_eq!(response.results.len(), deserialized.results.len());
    }

    #[test]
    fn test_find_similar_request_defaults() {
        let json = r#"{
            "event_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;

        let request: FindSimilarEventsRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.k, 10);
        assert!(request.tenant_id.is_none());
    }

    #[test]
    fn test_batch_embeddings_request() {
        let request = StoreBatchEmbeddingsRequest {
            embeddings: vec![
                StoreEmbeddingRequest {
                    event_id: Uuid::new_v4(),
                    tenant_id: "tenant-1".to_string(),
                    embedding: vec![0.1, 0.2, 0.3],
                    source_text: None,
                },
                StoreEmbeddingRequest {
                    event_id: Uuid::new_v4(),
                    tenant_id: "tenant-1".to_string(),
                    embedding: vec![0.4, 0.5, 0.6],
                    source_text: Some("content".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: StoreBatchEmbeddingsRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.embeddings.len(), deserialized.embeddings.len());
    }

    #[test]
    fn test_skip_serializing_none_fields() {
        let request = StoreEmbeddingRequest {
            event_id: Uuid::new_v4(),
            tenant_id: "tenant-1".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            source_text: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("source_text"));
    }
}
