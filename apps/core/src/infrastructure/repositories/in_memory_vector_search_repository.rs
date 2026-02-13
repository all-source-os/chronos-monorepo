use crate::{
    domain::{
        repositories::{
            SearchResult, VectorEntry, VectorSearchQuery, VectorSearchReader,
            VectorSearchRepository, VectorSearchWriter,
        },
        value_objects::{DistanceMetric, EmbeddingVector, SimilarityScore},
    },
    error::{AllSourceError, Result},
};
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::{
    collections::{BinaryHeap, HashMap},
    sync::Arc,
};
use uuid::Uuid;

/// Internal storage entry for vectors
#[derive(Clone)]
struct StoredVector {
    event_id: Uuid,
    tenant_id: String,
    embedding: EmbeddingVector,
    source_text: Option<String>,
}

/// A scored candidate for k-NN search
#[derive(Clone)]
struct ScoredCandidate {
    event_id: Uuid,
    score: f32,
    higher_is_better: bool,
}

impl PartialEq for ScoredCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for ScoredCandidate {}

impl PartialOrd for ScoredCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // For max-heap when higher is better, we want reverse ordering
        // For min-heap when lower is better (distance), we want normal ordering
        if self.higher_is_better {
            // Min-heap behavior for top-k (we want to pop smallest scores)
            other
                .score
                .partial_cmp(&self.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            // Max-heap behavior for top-k (we want to pop largest distances)
            self.score
                .partial_cmp(&other.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    }
}

/// High-performance in-memory vector search repository
///
/// Features:
/// - Lock-free concurrent reads with DashMap
/// - Multi-tenant support with tenant isolation
/// - Brute-force k-NN search (efficient for small-medium datasets)
/// - Support for cosine, euclidean, and dot product metrics
/// - SIMD-optimized distance calculations (via f32 operations)
///
/// For large datasets (>100k vectors), consider using a specialized
/// vector database or implementing HNSW indexing.
pub struct InMemoryVectorSearchRepository {
    /// Primary storage: event_id -> StoredVector
    vectors: Arc<DashMap<Uuid, StoredVector>>,

    /// Tenant index: tenant_id -> list of event_ids
    tenant_index: Arc<DashMap<String, Vec<Uuid>>>,

    /// Cached dimensionality (all vectors must have same dimensions)
    dimensions: Arc<RwLock<Option<usize>>>,

    /// Statistics
    stats: Arc<RwLock<RepositoryStats>>,
}

#[derive(Default, Clone)]
struct RepositoryStats {
    total_vectors: usize,
    total_searches: u64,
    total_stores: u64,
}

impl InMemoryVectorSearchRepository {
    pub fn new() -> Self {
        Self {
            vectors: Arc::new(DashMap::new()),
            tenant_index: Arc::new(DashMap::new()),
            dimensions: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(RepositoryStats::default())),
        }
    }

    /// Get repository statistics
    pub fn stats(&self) -> (usize, u64, u64) {
        let stats = self.stats.read();
        (
            stats.total_vectors,
            stats.total_searches,
            stats.total_stores,
        )
    }

    /// Validate that the embedding dimensions match the stored dimensions
    fn validate_dimensions(&self, embedding: &EmbeddingVector) -> Result<()> {
        let dims = self.dimensions.read();
        if let Some(expected) = *dims
            && embedding.dimensions() != expected
        {
            return Err(AllSourceError::InvalidInput(format!(
                "Embedding dimension mismatch: expected {}, got {}",
                expected,
                embedding.dimensions()
            )));
        }
        Ok(())
    }

    /// Set dimensions if not already set
    fn set_dimensions(&self, dims: usize) {
        let mut stored_dims = self.dimensions.write();
        if stored_dims.is_none() {
            *stored_dims = Some(dims);
        }
    }

    /// Perform brute-force k-NN search
    fn brute_force_search(
        &self,
        query: &VectorSearchQuery,
        candidates: &[&StoredVector],
    ) -> Result<Vec<SearchResult>> {
        let k = query.k;
        let metric = &query.metric;
        let higher_is_better = metric.higher_is_better();

        // Use a bounded heap for efficient top-k
        let mut heap: BinaryHeap<ScoredCandidate> = BinaryHeap::with_capacity(k + 1);

        for stored in candidates {
            let score = metric.calculate(&query.query_vector, &stored.embedding)?;

            // Apply threshold filtering
            let passes_threshold = match metric {
                DistanceMetric::Cosine | DistanceMetric::DotProduct => {
                    query.min_similarity.is_none_or(|min| score.value() >= min)
                }
                DistanceMetric::Euclidean => {
                    query.max_distance.is_none_or(|max| score.value() <= max)
                }
            };

            if passes_threshold {
                heap.push(ScoredCandidate {
                    event_id: stored.event_id,
                    score: score.value(),
                    higher_is_better,
                });

                // Keep only top-k
                if heap.len() > k {
                    heap.pop();
                }
            }
        }

        // Convert heap to sorted results
        let mut results: Vec<_> = heap.into_iter().collect();

        // Sort by score (best first)
        if higher_is_better {
            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            results.sort_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // Build final results
        let mut search_results = Vec::with_capacity(results.len());
        for candidate in results {
            if let Some(stored) = self.vectors.get(&candidate.event_id) {
                search_results.push(
                    SearchResult::new(
                        candidate.event_id,
                        SimilarityScore::new_unchecked(candidate.score),
                        stored.embedding.clone(),
                    )
                    .with_source_text(stored.source_text.clone()),
                );
            }
        }

        Ok(search_results)
    }
}

impl Default for InMemoryVectorSearchRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorSearchRepository for InMemoryVectorSearchRepository {
    async fn store(
        &self,
        event_id: Uuid,
        embedding: &EmbeddingVector,
        tenant_id: &str,
    ) -> Result<()> {
        self.validate_dimensions(embedding)?;
        self.set_dimensions(embedding.dimensions());

        let stored = StoredVector {
            event_id,
            tenant_id: tenant_id.to_string(),
            embedding: embedding.clone(),
            source_text: None,
        };

        // Insert into primary storage
        let is_new = self.vectors.insert(event_id, stored).is_none();

        // Update tenant index
        self.tenant_index
            .entry(tenant_id.to_string())
            .or_default()
            .push(event_id);

        // Update stats
        if is_new {
            let mut stats = self.stats.write();
            stats.total_vectors += 1;
            stats.total_stores += 1;
        }

        Ok(())
    }

    async fn store_with_text(
        &self,
        event_id: Uuid,
        embedding: &EmbeddingVector,
        tenant_id: &str,
        source_text: &str,
    ) -> Result<()> {
        self.validate_dimensions(embedding)?;
        self.set_dimensions(embedding.dimensions());

        let stored = StoredVector {
            event_id,
            tenant_id: tenant_id.to_string(),
            embedding: embedding.clone(),
            source_text: Some(source_text.to_string()),
        };

        let is_new = self.vectors.insert(event_id, stored).is_none();

        self.tenant_index
            .entry(tenant_id.to_string())
            .or_default()
            .push(event_id);

        if is_new {
            let mut stats = self.stats.write();
            stats.total_vectors += 1;
            stats.total_stores += 1;
        }

        Ok(())
    }

    async fn store_batch(&self, entries: &[(Uuid, EmbeddingVector, String)]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        // Validate all dimensions first
        for (_, embedding, _) in entries {
            self.validate_dimensions(embedding)?;
        }

        // Set dimensions from first entry
        if let Some((_, embedding, _)) = entries.first() {
            self.set_dimensions(embedding.dimensions());
        }

        // Group by tenant for efficient index updates
        let mut by_tenant: HashMap<String, Vec<Uuid>> = HashMap::new();
        let mut new_count = 0;

        for (event_id, embedding, tenant_id) in entries {
            let stored = StoredVector {
                event_id: *event_id,
                tenant_id: tenant_id.clone(),
                embedding: embedding.clone(),
                source_text: None,
            };

            if self.vectors.insert(*event_id, stored).is_none() {
                new_count += 1;
            }

            by_tenant
                .entry(tenant_id.clone())
                .or_default()
                .push(*event_id);
        }

        // Batch update tenant indices
        for (tenant_id, event_ids) in by_tenant {
            self.tenant_index
                .entry(tenant_id)
                .or_default()
                .extend(event_ids);
        }

        // Update stats
        let mut stats = self.stats.write();
        stats.total_vectors += new_count;
        stats.total_stores += entries.len() as u64;

        Ok(())
    }

    async fn search(&self, query: &VectorSearchQuery) -> Result<Vec<SearchResult>> {
        // Update search count
        {
            let mut stats = self.stats.write();
            stats.total_searches += 1;
        }

        // Validate query dimensions
        self.validate_dimensions(&query.query_vector)?;

        // Collect candidates based on filters
        let candidates: Vec<_> = if let Some(tenant_id) = &query.tenant_id {
            // Filter by tenant
            if let Some(event_ids) = self.tenant_index.get(tenant_id) {
                event_ids
                    .iter()
                    .filter_map(|id| self.vectors.get(id).map(|r| r.clone()))
                    .collect()
            } else {
                return Ok(vec![]);
            }
        } else {
            // All vectors
            self.vectors.iter().map(|r| r.clone()).collect()
        };

        // Apply event type filter if specified
        let filtered: Vec<_> = if query.event_type.is_some() {
            // Note: We don't have event_type in StoredVector
            // This would need to be joined with the event repository
            // For now, we return all candidates and let the application layer filter
            candidates.iter().collect()
        } else {
            candidates.iter().collect()
        };

        if filtered.is_empty() {
            return Ok(vec![]);
        }

        self.brute_force_search(query, &filtered)
    }

    async fn get_by_event_id(&self, event_id: Uuid) -> Result<Option<VectorEntry>> {
        Ok(self.vectors.get(&event_id).map(|stored| {
            let entry = VectorEntry::new(stored.event_id, stored.embedding.clone());
            if let Some(text) = &stored.source_text {
                entry.with_source_text(text.clone())
            } else {
                entry
            }
        }))
    }

    async fn delete(&self, event_id: Uuid) -> Result<bool> {
        if let Some((_, stored)) = self.vectors.remove(&event_id) {
            // Remove from tenant index
            if let Some(mut event_ids) = self.tenant_index.get_mut(&stored.tenant_id) {
                event_ids.retain(|id| *id != event_id);
            }

            let mut stats = self.stats.write();
            stats.total_vectors = stats.total_vectors.saturating_sub(1);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn delete_by_tenant(&self, tenant_id: &str) -> Result<usize> {
        let event_ids = if let Some((_, ids)) = self.tenant_index.remove(tenant_id) {
            ids
        } else {
            return Ok(0);
        };

        let mut deleted = 0;
        for event_id in &event_ids {
            if self.vectors.remove(event_id).is_some() {
                deleted += 1;
            }
        }

        let mut stats = self.stats.write();
        stats.total_vectors = stats.total_vectors.saturating_sub(deleted);

        Ok(deleted)
    }

    async fn count(&self, tenant_id: Option<&str>) -> Result<usize> {
        if let Some(tid) = tenant_id {
            Ok(self.tenant_index.get(tid).map_or(0, |ids| ids.len()))
        } else {
            Ok(self.vectors.len())
        }
    }

    async fn dimensions(&self) -> Result<Option<usize>> {
        Ok(*self.dimensions.read())
    }

    async fn health_check(&self) -> Result<()> {
        // Simple health check - verify data structure integrity
        let vec_count = self.vectors.len();
        let index_count: usize = self.tenant_index.iter().map(|e| e.value().len()).sum();

        // Allow some discrepancy due to concurrent operations
        if vec_count > 0 && index_count == 0 {
            return Err(AllSourceError::InternalError(
                "Vector index inconsistency detected".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl VectorSearchReader for InMemoryVectorSearchRepository {
    async fn search(&self, query: &VectorSearchQuery) -> Result<Vec<SearchResult>> {
        VectorSearchRepository::search(self, query).await
    }

    async fn get_by_event_id(&self, event_id: Uuid) -> Result<Option<VectorEntry>> {
        VectorSearchRepository::get_by_event_id(self, event_id).await
    }

    async fn count(&self, tenant_id: Option<&str>) -> Result<usize> {
        VectorSearchRepository::count(self, tenant_id).await
    }
}

#[async_trait]
impl VectorSearchWriter for InMemoryVectorSearchRepository {
    async fn store(
        &self,
        event_id: Uuid,
        embedding: &EmbeddingVector,
        tenant_id: &str,
    ) -> Result<()> {
        VectorSearchRepository::store(self, event_id, embedding, tenant_id).await
    }

    async fn store_batch(&self, entries: &[(Uuid, EmbeddingVector, String)]) -> Result<()> {
        VectorSearchRepository::store_batch(self, entries).await
    }

    async fn delete(&self, event_id: Uuid) -> Result<bool> {
        VectorSearchRepository::delete(self, event_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::VectorSearchRepository;

    fn create_test_embedding(dims: usize, seed: f32) -> EmbeddingVector {
        let values: Vec<f32> = (0..dims).map(|i| (i as f32 + seed) / dims as f32).collect();
        EmbeddingVector::new(values).unwrap()
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let repo = InMemoryVectorSearchRepository::new();
        let event_id = Uuid::new_v4();
        let embedding = create_test_embedding(384, 1.0);

        VectorSearchRepository::store(&repo, event_id, &embedding, "tenant-1")
            .await
            .unwrap();

        let retrieved = VectorSearchRepository::get_by_event_id(&repo, event_id)
            .await
            .unwrap();
        assert!(retrieved.is_some());

        let entry = retrieved.unwrap();
        assert_eq!(entry.event_id, event_id);
        assert_eq!(entry.embedding.dimensions(), 384);
    }

    #[tokio::test]
    async fn test_store_with_text() {
        let repo = InMemoryVectorSearchRepository::new();
        let event_id = Uuid::new_v4();
        let embedding = create_test_embedding(384, 1.0);

        VectorSearchRepository::store_with_text(
            &repo,
            event_id,
            &embedding,
            "tenant-1",
            "test content",
        )
        .await
        .unwrap();

        let retrieved = VectorSearchRepository::get_by_event_id(&repo, event_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.source_text, Some("test content".to_string()));
    }

    #[tokio::test]
    async fn test_dimension_validation() {
        let repo = InMemoryVectorSearchRepository::new();
        let event_id1 = Uuid::new_v4();
        let event_id2 = Uuid::new_v4();

        let embedding1 = create_test_embedding(384, 1.0);
        let embedding2 = create_test_embedding(768, 2.0); // Different dimensions

        VectorSearchRepository::store(&repo, event_id1, &embedding1, "tenant-1")
            .await
            .unwrap();

        // Second store with different dimensions should fail
        let result = VectorSearchRepository::store(&repo, event_id2, &embedding2, "tenant-1").await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("dimension mismatch"));
        }
    }

    #[tokio::test]
    async fn test_basic_search() {
        let repo = InMemoryVectorSearchRepository::new();

        // Store some vectors
        let embedding1 = EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap();
        let embedding2 = EmbeddingVector::new(vec![0.9, 0.1, 0.0]).unwrap();
        let embedding3 = EmbeddingVector::new(vec![0.0, 1.0, 0.0]).unwrap();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        VectorSearchRepository::store(&repo, id1, &embedding1, "tenant-1")
            .await
            .unwrap();
        VectorSearchRepository::store(&repo, id2, &embedding2, "tenant-1")
            .await
            .unwrap();
        VectorSearchRepository::store(&repo, id3, &embedding3, "tenant-1")
            .await
            .unwrap();

        // Search for similar to embedding1
        let query_vec = EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap();
        let query = VectorSearchQuery::new(query_vec, 2).with_tenant("tenant-1".to_string());

        let results = VectorSearchRepository::search(&repo, &query).await.unwrap();

        assert_eq!(results.len(), 2);
        // First result should be exact match (id1)
        assert_eq!(results[0].event_id, id1);
        assert!((results[0].score.value() - 1.0).abs() < 1e-6);
        // Second result should be id2 (more similar than id3)
        assert_eq!(results[1].event_id, id2);
    }

    #[tokio::test]
    async fn test_search_with_min_similarity() {
        let repo = InMemoryVectorSearchRepository::new();

        let embedding1 = EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap();
        let embedding2 = EmbeddingVector::new(vec![0.0, 1.0, 0.0]).unwrap(); // Orthogonal

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        VectorSearchRepository::store(&repo, id1, &embedding1, "tenant-1")
            .await
            .unwrap();
        VectorSearchRepository::store(&repo, id2, &embedding2, "tenant-1")
            .await
            .unwrap();

        let query_vec = EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap();
        let query = VectorSearchQuery::new(query_vec, 10)
            .with_tenant("tenant-1".to_string())
            .with_min_similarity(0.5);

        let results = VectorSearchRepository::search(&repo, &query).await.unwrap();

        // Only id1 should match (similarity = 1.0)
        // id2 has similarity = 0.0, below threshold
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_id, id1);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let repo = InMemoryVectorSearchRepository::new();

        let embedding = EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        VectorSearchRepository::store(&repo, id1, &embedding, "tenant-1")
            .await
            .unwrap();
        VectorSearchRepository::store(&repo, id2, &embedding, "tenant-2")
            .await
            .unwrap();

        let query_vec = EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap();
        let query = VectorSearchQuery::new(query_vec, 10).with_tenant("tenant-1".to_string());

        let results = VectorSearchRepository::search(&repo, &query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_id, id1);
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryVectorSearchRepository::new();

        let embedding = create_test_embedding(384, 1.0);
        let event_id = Uuid::new_v4();

        VectorSearchRepository::store(&repo, event_id, &embedding, "tenant-1")
            .await
            .unwrap();
        assert_eq!(VectorSearchRepository::count(&repo, None).await.unwrap(), 1);

        let deleted = VectorSearchRepository::delete(&repo, event_id)
            .await
            .unwrap();
        assert!(deleted);
        assert_eq!(VectorSearchRepository::count(&repo, None).await.unwrap(), 0);

        let retrieved = VectorSearchRepository::get_by_event_id(&repo, event_id)
            .await
            .unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_by_tenant() {
        let repo = InMemoryVectorSearchRepository::new();

        let embedding = create_test_embedding(384, 1.0);

        for i in 0..5 {
            let tenant = if i < 3 { "tenant-1" } else { "tenant-2" };
            VectorSearchRepository::store(&repo, Uuid::new_v4(), &embedding, tenant)
                .await
                .unwrap();
        }

        assert_eq!(
            VectorSearchRepository::count(&repo, Some("tenant-1"))
                .await
                .unwrap(),
            3
        );
        assert_eq!(
            VectorSearchRepository::count(&repo, Some("tenant-2"))
                .await
                .unwrap(),
            2
        );

        let deleted = VectorSearchRepository::delete_by_tenant(&repo, "tenant-1")
            .await
            .unwrap();
        assert_eq!(deleted, 3);

        assert_eq!(
            VectorSearchRepository::count(&repo, Some("tenant-1"))
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            VectorSearchRepository::count(&repo, Some("tenant-2"))
                .await
                .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn test_batch_store() {
        let repo = InMemoryVectorSearchRepository::new();

        let entries: Vec<_> = (0..100)
            .map(|i| {
                let embedding = create_test_embedding(384, i as f32);
                (Uuid::new_v4(), embedding, "tenant-1".to_string())
            })
            .collect();

        VectorSearchRepository::store_batch(&repo, &entries)
            .await
            .unwrap();

        assert_eq!(
            VectorSearchRepository::count(&repo, None).await.unwrap(),
            100
        );
        assert_eq!(
            VectorSearchRepository::dimensions(&repo).await.unwrap(),
            Some(384)
        );
    }

    #[tokio::test]
    async fn test_euclidean_distance_search() {
        let repo = InMemoryVectorSearchRepository::new();

        let embedding1 = EmbeddingVector::new(vec![0.0, 0.0]).unwrap();
        let embedding2 = EmbeddingVector::new(vec![3.0, 4.0]).unwrap(); // Distance = 5
        let embedding3 = EmbeddingVector::new(vec![1.0, 0.0]).unwrap(); // Distance = 1

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        VectorSearchRepository::store(&repo, id1, &embedding1, "tenant-1")
            .await
            .unwrap();
        VectorSearchRepository::store(&repo, id2, &embedding2, "tenant-1")
            .await
            .unwrap();
        VectorSearchRepository::store(&repo, id3, &embedding3, "tenant-1")
            .await
            .unwrap();

        let query_vec = EmbeddingVector::new(vec![0.0, 0.0]).unwrap();
        let query = VectorSearchQuery::new(query_vec, 3)
            .with_tenant("tenant-1".to_string())
            .with_metric(DistanceMetric::Euclidean);

        let results = VectorSearchRepository::search(&repo, &query).await.unwrap();

        assert_eq!(results.len(), 3);
        // Results should be sorted by distance (ascending)
        assert_eq!(results[0].event_id, id1); // Distance = 0
        assert_eq!(results[1].event_id, id3); // Distance = 1
        assert_eq!(results[2].event_id, id2); // Distance = 5
    }

    #[tokio::test]
    async fn test_health_check() {
        let repo = InMemoryVectorSearchRepository::new();
        assert!(VectorSearchRepository::health_check(&repo).await.is_ok());

        let embedding = create_test_embedding(384, 1.0);
        VectorSearchRepository::store(&repo, Uuid::new_v4(), &embedding, "tenant-1")
            .await
            .unwrap();

        assert!(VectorSearchRepository::health_check(&repo).await.is_ok());
    }

    #[tokio::test]
    async fn test_stats() {
        let repo = InMemoryVectorSearchRepository::new();

        let embedding = create_test_embedding(384, 1.0);
        for _ in 0..5 {
            VectorSearchRepository::store(&repo, Uuid::new_v4(), &embedding, "tenant-1")
                .await
                .unwrap();
        }

        let query_vec = create_test_embedding(384, 2.0);
        let query = VectorSearchQuery::new(query_vec, 3).with_tenant("tenant-1".to_string());
        VectorSearchRepository::search(&repo, &query).await.unwrap();
        VectorSearchRepository::search(&repo, &query).await.unwrap();

        let (total_vectors, total_searches, total_stores) = repo.stats();
        assert_eq!(total_vectors, 5);
        assert_eq!(total_searches, 2);
        assert_eq!(total_stores, 5);
    }
}
