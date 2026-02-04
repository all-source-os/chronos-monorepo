use crate::domain::entities::Event;
use crate::domain::repositories::{
    EventRepository, SearchResult, VectorEntry, VectorSearchQuery, VectorSearchRepository,
};
use crate::domain::value_objects::{DistanceMetric, EmbeddingVector};
use crate::error::{AllSourceError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Configuration for the vector search service
#[derive(Debug, Clone)]
pub struct VectorSearchConfig {
    /// Default number of results to return
    pub default_k: usize,
    /// Maximum number of results to return
    pub max_k: usize,
    /// Default similarity threshold for cosine similarity
    pub default_min_similarity: f32,
    /// Default distance metric
    pub default_metric: DistanceMetric,
    /// Whether to include source text in results
    pub include_source_text: bool,
}

impl Default for VectorSearchConfig {
    fn default() -> Self {
        Self {
            default_k: 10,
            max_k: 100,
            default_min_similarity: 0.0,
            default_metric: DistanceMetric::Cosine,
            include_source_text: true,
        }
    }
}

/// Request to index an event's embedding
#[derive(Debug, Clone)]
pub struct IndexEventRequest {
    pub event_id: Uuid,
    pub tenant_id: String,
    pub embedding: EmbeddingVector,
    pub source_text: Option<String>,
}

/// Request for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchRequest {
    /// The query embedding vector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_embedding: Option<Vec<f32>>,
    /// Number of results to return (default: 10)
    #[serde(default)]
    pub k: Option<usize>,
    /// Tenant ID filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Event type filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    /// Minimum similarity threshold (for cosine/dot product)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_similarity: Option<f32>,
    /// Maximum distance threshold (for euclidean)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_distance: Option<f32>,
    /// Distance metric (default: cosine)
    #[serde(default)]
    pub metric: Option<String>,
    /// Whether to include full event data in results
    #[serde(default)]
    pub include_events: bool,
}

impl Default for SemanticSearchRequest {
    fn default() -> Self {
        Self {
            query_embedding: None,
            k: None,
            tenant_id: None,
            event_type: None,
            min_similarity: None,
            max_distance: None,
            metric: None,
            include_events: false,
        }
    }
}

/// A single semantic search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResultItem {
    /// The event ID that matched
    pub event_id: Uuid,
    /// The similarity/distance score
    pub score: f32,
    /// The source text (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    /// The full event (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<EventSummary>,
}

/// Summary of an event for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub id: Uuid,
    pub event_type: String,
    pub entity_id: String,
    pub tenant_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl From<&Event> for EventSummary {
    fn from(event: &Event) -> Self {
        Self {
            id: event.id(),
            event_type: event.event_type_str().to_string(),
            entity_id: event.entity_id_str().to_string(),
            tenant_id: event.tenant_id_str().to_string(),
            timestamp: event.timestamp(),
            payload: Some(event.payload().clone()),
        }
    }
}

/// Response from semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResponse {
    /// The search results
    pub results: Vec<SemanticSearchResultItem>,
    /// Total number of results
    pub count: usize,
    /// The metric used for scoring
    pub metric: String,
    /// Query execution stats
    pub stats: SearchStats,
}

/// Statistics about the search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStats {
    /// Total vectors searched
    pub vectors_searched: usize,
    /// Time taken in microseconds
    pub search_time_us: u64,
}

/// Vector Search Service
///
/// Orchestrates vector search operations including:
/// - Indexing event embeddings
/// - Semantic similarity search
/// - Integration with event repository for full results
///
/// This service follows the application layer pattern, coordinating
/// between the domain repositories without containing domain logic.
pub struct VectorSearchService {
    vector_repo: Arc<dyn VectorSearchRepository>,
    event_repo: Option<Arc<dyn EventRepository>>,
    config: VectorSearchConfig,
}

impl VectorSearchService {
    pub fn new(vector_repo: Arc<dyn VectorSearchRepository>) -> Self {
        Self {
            vector_repo,
            event_repo: None,
            config: VectorSearchConfig::default(),
        }
    }

    pub fn with_event_repo(mut self, event_repo: Arc<dyn EventRepository>) -> Self {
        self.event_repo = Some(event_repo);
        self
    }

    pub fn with_config(mut self, config: VectorSearchConfig) -> Self {
        self.config = config;
        self
    }

    /// Index a single event embedding
    pub async fn index_event(&self, request: IndexEventRequest) -> Result<()> {
        if let Some(source_text) = &request.source_text {
            self.vector_repo
                .store_with_text(
                    request.event_id,
                    &request.embedding,
                    &request.tenant_id,
                    source_text,
                )
                .await
        } else {
            self.vector_repo
                .store(request.event_id, &request.embedding, &request.tenant_id)
                .await
        }
    }

    /// Index multiple events in batch
    pub async fn index_events_batch(
        &self,
        requests: Vec<IndexEventRequest>,
    ) -> Result<BatchIndexResult> {
        if requests.is_empty() {
            return Ok(BatchIndexResult {
                indexed: 0,
                failed: 0,
                errors: vec![],
            });
        }

        let entries: Vec<_> = requests
            .iter()
            .map(|r| (r.event_id, r.embedding.clone(), r.tenant_id.clone()))
            .collect();

        self.vector_repo.store_batch(&entries).await?;

        Ok(BatchIndexResult {
            indexed: requests.len(),
            failed: 0,
            errors: vec![],
        })
    }

    /// Perform semantic search
    pub async fn search(&self, request: SemanticSearchRequest) -> Result<SemanticSearchResponse> {
        let start_time = std::time::Instant::now();

        // Parse and validate query embedding
        let query_embedding = request.query_embedding.ok_or_else(|| {
            AllSourceError::InvalidInput("query_embedding is required".to_string())
        })?;

        let query_vector = EmbeddingVector::new(query_embedding)?;

        // Parse metric
        let metric = match request.metric.as_deref() {
            Some("cosine") | None => DistanceMetric::Cosine,
            Some("euclidean") => DistanceMetric::Euclidean,
            Some("dot_product") => DistanceMetric::DotProduct,
            Some(m) => {
                return Err(AllSourceError::InvalidInput(format!(
                    "Unknown metric: {}. Supported: cosine, euclidean, dot_product",
                    m
                )));
            }
        };

        // Build query
        let k = request
            .k
            .unwrap_or(self.config.default_k)
            .min(self.config.max_k);

        let mut query = VectorSearchQuery::new(query_vector, k).with_metric(metric);

        if let Some(tenant_id) = request.tenant_id {
            query = query.with_tenant(tenant_id);
        }

        if let Some(event_type) = request.event_type {
            query = query.with_event_type(event_type);
        }

        if let Some(min_sim) = request.min_similarity {
            query = query.with_min_similarity(min_sim);
        }

        if let Some(max_dist) = request.max_distance {
            query = query.with_max_distance(max_dist);
        }

        // Execute search
        let search_results = self.vector_repo.search(&query).await?;
        let vectors_searched = self.vector_repo.count(None).await.unwrap_or(0);

        // Optionally fetch full events
        let results = if request.include_events {
            self.enrich_with_events(search_results).await?
        } else {
            search_results
                .into_iter()
                .map(|r| SemanticSearchResultItem {
                    event_id: r.event_id,
                    score: r.score.value(),
                    source_text: r.source_text,
                    event: None,
                })
                .collect()
        };

        let search_time_us = start_time.elapsed().as_micros() as u64;
        let count = results.len();

        Ok(SemanticSearchResponse {
            results,
            count,
            metric: format!("{:?}", metric).to_lowercase(),
            stats: SearchStats {
                vectors_searched,
                search_time_us,
            },
        })
    }

    /// Get embedding for a specific event
    pub async fn get_embedding(&self, event_id: Uuid) -> Result<Option<VectorEntry>> {
        self.vector_repo.get_by_event_id(event_id).await
    }

    /// Delete embedding for an event
    pub async fn delete_embedding(&self, event_id: Uuid) -> Result<bool> {
        self.vector_repo.delete(event_id).await
    }

    /// Delete all embeddings for a tenant
    pub async fn delete_tenant_embeddings(&self, tenant_id: &str) -> Result<usize> {
        self.vector_repo.delete_by_tenant(tenant_id).await
    }

    /// Get index statistics
    pub async fn get_stats(&self) -> Result<IndexStats> {
        let total_vectors = self.vector_repo.count(None).await?;
        let dimensions = self.vector_repo.dimensions().await?;

        Ok(IndexStats {
            total_vectors,
            dimensions,
        })
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        self.vector_repo.health_check().await
    }

    /// Enrich search results with full event data
    async fn enrich_with_events(
        &self,
        results: Vec<SearchResult>,
    ) -> Result<Vec<SemanticSearchResultItem>> {
        let event_repo = match &self.event_repo {
            Some(repo) => repo,
            None => {
                // No event repo, return without events
                return Ok(results
                    .into_iter()
                    .map(|r| SemanticSearchResultItem {
                        event_id: r.event_id,
                        score: r.score.value(),
                        source_text: r.source_text,
                        event: None,
                    })
                    .collect());
            }
        };

        let mut enriched = Vec::with_capacity(results.len());

        for result in results {
            let event = event_repo.find_by_id(result.event_id).await?;

            enriched.push(SemanticSearchResultItem {
                event_id: result.event_id,
                score: result.score.value(),
                source_text: result.source_text,
                event: event.as_ref().map(EventSummary::from),
            });
        }

        Ok(enriched)
    }
}

/// Result of batch indexing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIndexResult {
    pub indexed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_vectors: usize,
    pub dimensions: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::InMemoryVectorSearchRepository;

    fn create_test_service() -> VectorSearchService {
        let repo = Arc::new(InMemoryVectorSearchRepository::new());
        VectorSearchService::new(repo)
    }

    fn create_test_embedding(dims: usize, seed: f32) -> EmbeddingVector {
        let values: Vec<f32> = (0..dims).map(|i| (i as f32 + seed) / dims as f32).collect();
        EmbeddingVector::new(values).unwrap()
    }

    #[tokio::test]
    async fn test_index_and_search() {
        let service = create_test_service();

        // Index some events
        let embeddings = vec![
            (Uuid::new_v4(), vec![1.0, 0.0, 0.0_f32]),
            (Uuid::new_v4(), vec![0.9, 0.1, 0.0]),
            (Uuid::new_v4(), vec![0.0, 1.0, 0.0]),
        ];

        for (id, values) in &embeddings {
            service
                .index_event(IndexEventRequest {
                    event_id: *id,
                    tenant_id: "tenant-1".to_string(),
                    embedding: EmbeddingVector::new(values.clone()).unwrap(),
                    source_text: None,
                })
                .await
                .unwrap();
        }

        // Search
        let response = service
            .search(SemanticSearchRequest {
                query_embedding: Some(vec![1.0, 0.0, 0.0]),
                k: Some(2),
                tenant_id: Some("tenant-1".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(response.count, 2);
        assert_eq!(response.results[0].event_id, embeddings[0].0);
    }

    #[tokio::test]
    async fn test_batch_index() {
        let service = create_test_service();

        let requests: Vec<_> = (0..10)
            .map(|i| IndexEventRequest {
                event_id: Uuid::new_v4(),
                tenant_id: "tenant-1".to_string(),
                embedding: create_test_embedding(384, i as f32),
                source_text: Some(format!("Document {}", i)),
            })
            .collect();

        let result = service.index_events_batch(requests).await.unwrap();
        assert_eq!(result.indexed, 10);
        assert_eq!(result.failed, 0);

        let stats = service.get_stats().await.unwrap();
        assert_eq!(stats.total_vectors, 10);
        assert_eq!(stats.dimensions, Some(384));
    }

    #[tokio::test]
    async fn test_search_with_min_similarity() {
        let service = create_test_service();

        // Index vectors
        service
            .index_event(IndexEventRequest {
                event_id: Uuid::new_v4(),
                tenant_id: "tenant-1".to_string(),
                embedding: EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap(),
                source_text: None,
            })
            .await
            .unwrap();

        service
            .index_event(IndexEventRequest {
                event_id: Uuid::new_v4(),
                tenant_id: "tenant-1".to_string(),
                embedding: EmbeddingVector::new(vec![0.0, 1.0, 0.0]).unwrap(),
                source_text: None,
            })
            .await
            .unwrap();

        // Search with high threshold
        let response = service
            .search(SemanticSearchRequest {
                query_embedding: Some(vec![1.0, 0.0, 0.0]),
                k: Some(10),
                tenant_id: Some("tenant-1".to_string()),
                min_similarity: Some(0.5),
                ..Default::default()
            })
            .await
            .unwrap();

        // Only one should match (the exact match)
        assert_eq!(response.count, 1);
    }

    #[tokio::test]
    async fn test_delete_embedding() {
        let service = create_test_service();

        let event_id = Uuid::new_v4();
        service
            .index_event(IndexEventRequest {
                event_id,
                tenant_id: "tenant-1".to_string(),
                embedding: create_test_embedding(384, 1.0),
                source_text: None,
            })
            .await
            .unwrap();

        assert!(service.get_embedding(event_id).await.unwrap().is_some());

        let deleted = service.delete_embedding(event_id).await.unwrap();
        assert!(deleted);

        assert!(service.get_embedding(event_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_health_check() {
        let service = create_test_service();
        assert!(service.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_metric() {
        let service = create_test_service();

        let result = service
            .search(SemanticSearchRequest {
                query_embedding: Some(vec![1.0, 0.0, 0.0]),
                metric: Some("invalid".to_string()),
                ..Default::default()
            })
            .await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Unknown metric"));
        }
    }

    #[tokio::test]
    async fn test_missing_query_embedding() {
        let service = create_test_service();

        let result = service
            .search(SemanticSearchRequest {
                query_embedding: None,
                ..Default::default()
            })
            .await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("query_embedding is required"));
        }
    }
}
