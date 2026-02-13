use crate::{
    domain::value_objects::{DistanceMetric, EmbeddingVector, SimilarityScore},
    error::Result,
};
use async_trait::async_trait;
use uuid::Uuid;

/// Represents a stored vector with its associated event ID
#[derive(Debug, Clone)]
pub struct VectorEntry {
    /// The event this embedding is associated with
    pub event_id: Uuid,
    /// The embedding vector
    pub embedding: EmbeddingVector,
    /// Optional text content that was embedded (for debugging/display)
    pub source_text: Option<String>,
}

impl VectorEntry {
    pub fn new(event_id: Uuid, embedding: EmbeddingVector) -> Self {
        Self {
            event_id,
            embedding,
            source_text: None,
        }
    }

    pub fn with_source_text(mut self, text: String) -> Self {
        self.source_text = Some(text);
        self
    }
}

/// Result of a similarity search operation
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The event ID that matched
    pub event_id: Uuid,
    /// The similarity/distance score
    pub score: SimilarityScore,
    /// The matching embedding vector
    pub embedding: EmbeddingVector,
    /// Optional source text
    pub source_text: Option<String>,
}

impl SearchResult {
    pub fn new(event_id: Uuid, score: SimilarityScore, embedding: EmbeddingVector) -> Self {
        Self {
            event_id,
            score,
            embedding,
            source_text: None,
        }
    }

    pub fn with_source_text(mut self, text: Option<String>) -> Self {
        self.source_text = text;
        self
    }
}

/// Query parameters for vector similarity search
#[derive(Debug, Clone)]
pub struct VectorSearchQuery {
    /// The query vector to search for
    pub query_vector: EmbeddingVector,
    /// Number of results to return
    pub k: usize,
    /// Optional tenant filter
    pub tenant_id: Option<String>,
    /// Optional event type filter
    pub event_type: Option<String>,
    /// Minimum similarity threshold (only for cosine/dot product)
    pub min_similarity: Option<f32>,
    /// Maximum distance threshold (only for euclidean)
    pub max_distance: Option<f32>,
    /// Distance metric to use
    pub metric: DistanceMetric,
}

impl VectorSearchQuery {
    pub fn new(query_vector: EmbeddingVector, k: usize) -> Self {
        Self {
            query_vector,
            k,
            tenant_id: None,
            event_type: None,
            min_similarity: None,
            max_distance: None,
            metric: DistanceMetric::default(),
        }
    }

    pub fn with_tenant(mut self, tenant_id: String) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    pub fn with_event_type(mut self, event_type: String) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn with_min_similarity(mut self, min: f32) -> Self {
        self.min_similarity = Some(min);
        self
    }

    pub fn with_max_distance(mut self, max: f32) -> Self {
        self.max_distance = Some(max);
        self
    }

    pub fn with_metric(mut self, metric: DistanceMetric) -> Self {
        self.metric = metric;
        self
    }
}

/// Vector Search Repository Trait (Domain Layer)
///
/// This is the abstract interface for vector search operations.
/// Concrete implementations live in the infrastructure layer.
/// Following the Dependency Inversion Principle.
#[async_trait]
pub trait VectorSearchRepository: Send + Sync {
    /// Store a vector embedding for an event
    async fn store(
        &self,
        event_id: Uuid,
        embedding: &EmbeddingVector,
        tenant_id: &str,
    ) -> Result<()>;

    /// Store a vector embedding with source text
    async fn store_with_text(
        &self,
        event_id: Uuid,
        embedding: &EmbeddingVector,
        tenant_id: &str,
        source_text: &str,
    ) -> Result<()>;

    /// Store multiple embeddings in a batch
    async fn store_batch(&self, entries: &[(Uuid, EmbeddingVector, String)]) -> Result<()>;

    /// Search for similar vectors
    async fn search(&self, query: &VectorSearchQuery) -> Result<Vec<SearchResult>>;

    /// Get the embedding for a specific event
    async fn get_by_event_id(&self, event_id: Uuid) -> Result<Option<VectorEntry>>;

    /// Delete the embedding for an event
    async fn delete(&self, event_id: Uuid) -> Result<bool>;

    /// Delete all embeddings for a tenant
    async fn delete_by_tenant(&self, tenant_id: &str) -> Result<usize>;

    /// Get the total number of stored embeddings
    async fn count(&self, tenant_id: Option<&str>) -> Result<usize>;

    /// Get the dimensionality of stored vectors (returns None if empty)
    async fn dimensions(&self) -> Result<Option<usize>>;

    /// Check if the repository is healthy
    async fn health_check(&self) -> Result<()>;
}

/// Read-only vector search repository (for query optimization)
#[async_trait]
pub trait VectorSearchReader: Send + Sync {
    async fn search(&self, query: &VectorSearchQuery) -> Result<Vec<SearchResult>>;
    async fn get_by_event_id(&self, event_id: Uuid) -> Result<Option<VectorEntry>>;
    async fn count(&self, tenant_id: Option<&str>) -> Result<usize>;
}

/// Write-only vector search repository (for ingestion optimization)
#[async_trait]
pub trait VectorSearchWriter: Send + Sync {
    async fn store(
        &self,
        event_id: Uuid,
        embedding: &EmbeddingVector,
        tenant_id: &str,
    ) -> Result<()>;
    async fn store_batch(&self, entries: &[(Uuid, EmbeddingVector, String)]) -> Result<()>;
    async fn delete(&self, event_id: Uuid) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_entry_creation() {
        let event_id = Uuid::new_v4();
        let embedding = EmbeddingVector::new(vec![0.1, 0.2, 0.3]).unwrap();
        let entry = VectorEntry::new(event_id, embedding.clone());

        assert_eq!(entry.event_id, event_id);
        assert_eq!(entry.embedding.dimensions(), 3);
        assert!(entry.source_text.is_none());
    }

    #[test]
    fn test_vector_entry_with_source_text() {
        let event_id = Uuid::new_v4();
        let embedding = EmbeddingVector::new(vec![0.1, 0.2, 0.3]).unwrap();
        let entry =
            VectorEntry::new(event_id, embedding).with_source_text("test content".to_string());

        assert_eq!(entry.source_text, Some("test content".to_string()));
    }

    #[test]
    fn test_search_query_builder() {
        let query_vec = EmbeddingVector::new(vec![0.1, 0.2, 0.3]).unwrap();
        let query = VectorSearchQuery::new(query_vec, 10)
            .with_tenant("tenant-1".to_string())
            .with_event_type("user.created".to_string())
            .with_min_similarity(0.8)
            .with_metric(DistanceMetric::Cosine);

        assert_eq!(query.k, 10);
        assert_eq!(query.tenant_id, Some("tenant-1".to_string()));
        assert_eq!(query.event_type, Some("user.created".to_string()));
        assert_eq!(query.min_similarity, Some(0.8));
        assert_eq!(query.metric, DistanceMetric::Cosine);
    }

    #[test]
    fn test_search_result() {
        let event_id = Uuid::new_v4();
        let score = SimilarityScore::new(0.95).unwrap();
        let embedding = EmbeddingVector::new(vec![0.1, 0.2, 0.3]).unwrap();

        let result = SearchResult::new(event_id, score, embedding)
            .with_source_text(Some("matched text".to_string()));

        assert_eq!(result.event_id, event_id);
        assert!((result.score.value() - 0.95).abs() < 1e-6);
        assert_eq!(result.source_text, Some("matched text".to_string()));
    }
}
