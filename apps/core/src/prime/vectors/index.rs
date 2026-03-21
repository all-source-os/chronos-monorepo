//! VectorIndex projection — maintains an HNSW index over stored embeddings.
//!
//! Uses `instant-distance` for pure-Rust HNSW (ADR-015). Since the crate is
//! batch-only (no incremental insert), vectors are accumulated in a `DashMap`
//! and the HNSW index is lazily rebuilt when a search is performed after mutations.
//!
//! ## Key design decisions (ADR-015)
//!
//! - **Generation counter** instead of boolean dirty flag — avoids TOCTOU race
//!   where concurrent mutations during rebuild cause missed vectors.
//! - **No separate `deleted` DashMap** — deletions remove directly from `vectors`,
//!   eliminating snapshot/restore asymmetry and `len()` underflow bugs.

use dashmap::DashMap;
use instant_distance::{Builder, HnswMap, Point, Search};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::{
    application::services::projection::Projection,
    domain::entities::Event,
    error::Result,
};

use super::types::event_types as vec_events;

/// A point in the HNSW index — wraps a `Vec<f32>`.
#[derive(Clone, Debug)]
struct VecPoint(Vec<f32>);

impl Point for VecPoint {
    fn distance(&self, other: &Self) -> f32 {
        // Cosine distance = 1.0 - cosine_similarity
        let dot: f32 = self.0.iter().zip(other.0.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f32 = self.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        let denom = norm_a * norm_b;
        if denom < f32::EPSILON {
            1.0
        } else {
            1.0 - (dot / denom)
        }
    }
}

/// Serializable entry for snapshot/restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorRecord {
    entity_id: String,
    vector: Vec<f32>,
    text: Option<String>,
    metadata: Option<Value>,
}

/// Configuration for the HNSW index.
#[derive(Debug, Clone)]
pub struct VectorIndexConfig {
    pub ef_construction: usize,
    pub ef_search: usize,
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            ef_construction: 100,
            ef_search: 100,
        }
    }
}

/// Result from a vector search.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub entity_id: String,
    pub distance: f32,
    pub text: Option<String>,
    pub metadata: Option<Value>,
}

/// HNSW-indexed vector projection with generation-based staleness tracking.
///
/// Vectors are stored in a `DashMap` as events arrive. The HNSW index is
/// rebuilt lazily when a search is performed and the generation has advanced.
pub struct VectorIndexProjection {
    name: String,
    /// entity_id -> VectorRecord (live vectors only — deletions remove entries)
    vectors: Arc<DashMap<String, VectorRecord>>,
    /// The built HNSW index (None when empty or invalidated)
    index: Arc<RwLock<Option<HnswMap<VecPoint, String>>>>,
    /// Mutation generation counter — incremented on each insert/delete.
    generation: Arc<AtomicU64>,
    /// Generation at which the index was last built.
    built_generation: Arc<AtomicU64>,
    config: VectorIndexConfig,
}

impl VectorIndexProjection {
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_config(name, VectorIndexConfig::default())
    }

    pub fn with_config(name: impl Into<String>, config: VectorIndexConfig) -> Self {
        Self {
            name: name.into(),
            vectors: Arc::new(DashMap::new()),
            index: Arc::new(RwLock::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            built_generation: Arc::new(AtomicU64::new(0)),
            config,
        }
    }

    /// Number of live vectors.
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Whether there are no live vectors.
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Search for the `top_k` nearest neighbors to `query`.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<SearchHit> {
        self.ensure_index();

        let guard = self.index.read();
        let Some(hnsw) = guard.as_ref() else {
            return Vec::new();
        };

        let query_point = VecPoint(query.to_vec());
        let mut search = Search::default();

        hnsw.search(&query_point, &mut search)
            .take(top_k)
            .filter_map(|item| {
                let entity_id = item.value;
                // Double-check existence (may have been deleted between rebuild and now)
                let record = self.vectors.get(entity_id)?;
                Some(SearchHit {
                    entity_id: entity_id.clone(),
                    distance: item.distance,
                    text: record.text.clone(),
                    metadata: record.metadata.clone(),
                })
            })
            .collect()
    }

    /// Rebuild the HNSW index if the generation has advanced since last build.
    fn ensure_index(&self) {
        let current_gen = self.generation.load(Ordering::Acquire);
        let built_gen = self.built_generation.load(Ordering::Acquire);
        if current_gen == built_gen {
            return; // Index is up to date
        }

        // Collect live vectors
        let mut points = Vec::new();
        let mut values = Vec::new();

        for entry in self.vectors.iter() {
            points.push(VecPoint(entry.value().vector.clone()));
            values.push(entry.key().clone());
        }

        if points.is_empty() {
            *self.index.write() = None;
        } else {
            let hnsw = Builder::default()
                .ef_construction(self.config.ef_construction)
                .build(points, values);
            *self.index.write() = Some(hnsw);
        }

        // CAS: only update if no concurrent mutations happened during build
        let _ = self.built_generation.compare_exchange(
            built_gen,
            current_gen,
            Ordering::Release,
            Ordering::Relaxed,
        );
    }
}

impl Projection for VectorIndexProjection {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&self, event: &Event) -> Result<()> {
        let event_type = event.event_type_str();
        let entity_id = event.entity_id_str().to_string();
        let payload = &event.payload;

        match event_type {
            vec_events::VECTOR_STORED => {
                let vector: Vec<f32> = event
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("embedding"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let text = payload
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                let metadata = payload.get("metadata").cloned();

                self.vectors.insert(
                    entity_id.clone(),
                    VectorRecord {
                        entity_id,
                        vector,
                        text,
                        metadata,
                    },
                );
                self.generation.fetch_add(1, Ordering::Release);
            }
            vec_events::VECTOR_DELETED => {
                self.vectors.remove(&entity_id);
                self.generation.fetch_add(1, Ordering::Release);
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.vectors
            .get(entity_id)
            .map(|r| serde_json::to_value(r.value()).unwrap_or(Value::Null))
    }

    fn clear(&self) {
        self.vectors.clear();
        *self.index.write() = None;
        self.generation.fetch_add(1, Ordering::Release);
    }

    fn snapshot(&self) -> Option<Value> {
        let records: Vec<VectorRecord> = self
            .vectors
            .iter()
            .map(|e| e.value().clone())
            .collect();
        serde_json::to_value(records).ok()
    }

    fn restore(&self, snapshot: &Value) -> Result<()> {
        let records: Vec<VectorRecord> = serde_json::from_value(snapshot.clone())
            .map_err(|e| crate::error::AllSourceError::StorageError(e.to_string()))?;
        self.vectors.clear();
        for record in records {
            self.vectors.insert(record.entity_id.clone(), record);
        }
        self.generation.fetch_add(1, Ordering::Release);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_vector_event(entity_id: &str, vector: Vec<f32>, text: Option<&str>) -> Event {
        Event::reconstruct_from_strings(
            Uuid::new_v4(),
            vec_events::VECTOR_STORED.to_string(),
            entity_id.to_string(),
            "default".to_string(),
            serde_json::json!({
                "text": text,
                "dimensions": vector.len(),
            }),
            Utc::now(),
            Some(serde_json::json!({ "embedding": vector })),
            1,
        )
    }

    fn make_delete_event(entity_id: &str) -> Event {
        Event::reconstruct_from_strings(
            Uuid::new_v4(),
            vec_events::VECTOR_DELETED.to_string(),
            entity_id.to_string(),
            "default".to_string(),
            serde_json::json!({}),
            Utc::now(),
            None,
            1,
        )
    }

    #[test]
    fn test_insert_100_vectors() {
        let proj = VectorIndexProjection::new("vec_idx");

        for i in 0..100 {
            let vector: Vec<f32> = (0..8).map(|j| (i * 8 + j) as f32).collect();
            let event =
                make_vector_event(&format!("vec:doc-{i}"), vector, Some(&format!("doc {i}")));
            proj.process(&event).unwrap();
        }

        assert_eq!(proj.len(), 100);
    }

    #[test]
    fn test_search_returns_results() {
        let proj = VectorIndexProjection::new("vec_idx");

        proj.process(&make_vector_event("vec:a", vec![1.0, 0.0, 0.0, 0.0], Some("close")))
            .unwrap();
        proj.process(&make_vector_event("vec:b", vec![0.7, 0.7, 0.0, 0.0], Some("medium")))
            .unwrap();
        proj.process(&make_vector_event("vec:c", vec![0.0, 0.0, 0.0, 1.0], Some("far")))
            .unwrap();

        let hits = proj.search(&[1.0, 0.0, 0.0, 0.0], 3);

        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].entity_id, "vec:a");
        assert!(hits[0].distance < 0.01);
        assert_eq!(hits[2].entity_id, "vec:c");
    }

    #[test]
    fn test_delete_excludes_from_search() {
        let proj = VectorIndexProjection::new("vec_idx");

        proj.process(&make_vector_event("vec:a", vec![1.0, 0.0], Some("a")))
            .unwrap();
        proj.process(&make_vector_event("vec:b", vec![0.9, 0.1], Some("b")))
            .unwrap();

        proj.process(&make_delete_event("vec:a")).unwrap();

        let hits = proj.search(&[1.0, 0.0], 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].entity_id, "vec:b");
    }

    #[test]
    fn test_snapshot_restore_roundtrip() {
        let proj = VectorIndexProjection::new("vec_idx");

        proj.process(&make_vector_event("vec:x", vec![1.0, 0.0, 0.0], Some("x")))
            .unwrap();
        proj.process(&make_vector_event("vec:y", vec![0.0, 1.0, 0.0], Some("y")))
            .unwrap();

        let snap = proj.snapshot().unwrap();
        proj.clear();
        assert_eq!(proj.len(), 0);

        proj.restore(&snap).unwrap();
        assert_eq!(proj.len(), 2);

        let hits = proj.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].entity_id, "vec:x");
    }

    #[test]
    fn test_snapshot_restore_after_delete_is_clean() {
        // Regression: old code had deleted vectors reappearing after restore
        let proj = VectorIndexProjection::new("vec_idx");

        proj.process(&make_vector_event("vec:a", vec![1.0, 0.0], Some("a")))
            .unwrap();
        proj.process(&make_vector_event("vec:b", vec![0.0, 1.0], Some("b")))
            .unwrap();
        proj.process(&make_delete_event("vec:a")).unwrap();

        let snap = proj.snapshot().unwrap();
        proj.clear();
        proj.restore(&snap).unwrap();

        assert_eq!(proj.len(), 1);
        assert!(proj.get_state("vec:a").is_none());
        assert!(proj.get_state("vec:b").is_some());
    }

    #[test]
    fn test_search_empty_index() {
        let proj = VectorIndexProjection::new("vec_idx");
        let hits = proj.search(&[1.0, 0.0], 10);
        assert!(hits.is_empty());
    }

    #[test]
    fn test_delete_nonexistent_is_noop() {
        let proj = VectorIndexProjection::new("vec_idx");
        proj.process(&make_delete_event("vec:ghost")).unwrap();
        assert_eq!(proj.len(), 0);
    }

    #[test]
    fn test_zero_vector_search() {
        let proj = VectorIndexProjection::new("vec_idx");
        proj.process(&make_vector_event("vec:a", vec![1.0, 0.0], Some("a")))
            .unwrap();
        proj.process(&make_vector_event("vec:zero", vec![0.0, 0.0], Some("zero")))
            .unwrap();

        // Searching with zero vector should return results (distance = 1.0 for all)
        let hits = proj.search(&[0.0, 0.0], 10);
        assert_eq!(hits.len(), 2);
    }
}
