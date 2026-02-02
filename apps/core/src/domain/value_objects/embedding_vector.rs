use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Value Object: EmbeddingVector
///
/// Represents a vector embedding for semantic search operations.
/// Embeddings are dense numerical representations of text/content
/// that capture semantic meaning in a high-dimensional space.
///
/// Domain Rules:
/// - Must have at least 1 dimension
/// - Maximum 4096 dimensions (supporting most embedding models)
/// - Values are typically normalized (but not enforced for flexibility)
/// - Immutable once created
///
/// Common dimensions:
/// - 384: MiniLM, all-MiniLM-L6-v2
/// - 768: BERT-base, sentence-transformers
/// - 1024: BERT-large
/// - 1536: OpenAI text-embedding-ada-002
/// - 3072: OpenAI text-embedding-3-large
/// - 4096: Some custom models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingVector {
    values: Vec<f32>,
}

impl EmbeddingVector {
    /// Maximum supported dimensions
    pub const MAX_DIMENSIONS: usize = 4096;

    /// Create a new EmbeddingVector with validation
    ///
    /// # Errors
    /// Returns error if:
    /// - Vector is empty
    /// - Vector exceeds MAX_DIMENSIONS
    pub fn new(values: Vec<f32>) -> Result<Self> {
        Self::validate(&values)?;
        Ok(Self { values })
    }

    /// Create EmbeddingVector without validation (for internal use)
    ///
    /// # Safety
    /// This bypasses validation. Only use when loading from trusted sources.
    pub(crate) fn new_unchecked(values: Vec<f32>) -> Self {
        Self { values }
    }

    /// Get the vector values as a slice
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Get the dimensionality of the vector
    pub fn dimensions(&self) -> usize {
        self.values.len()
    }

    /// Consume self and return the inner values
    pub fn into_inner(self) -> Vec<f32> {
        self.values
    }

    /// Calculate the L2 (Euclidean) norm of the vector
    pub fn l2_norm(&self) -> f32 {
        self.values.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    /// Check if the vector is normalized (L2 norm ≈ 1.0)
    pub fn is_normalized(&self) -> bool {
        let norm = self.l2_norm();
        (norm - 1.0).abs() < 1e-6
    }

    /// Return a normalized copy of the vector
    pub fn normalize(&self) -> Self {
        let norm = self.l2_norm();
        if norm < 1e-10 {
            return self.clone();
        }
        Self {
            values: self.values.iter().map(|x| x / norm).collect(),
        }
    }

    /// Calculate cosine similarity with another vector
    ///
    /// # Errors
    /// Returns error if vectors have different dimensions
    pub fn cosine_similarity(&self, other: &EmbeddingVector) -> Result<f32> {
        if self.dimensions() != other.dimensions() {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Vector dimension mismatch: {} vs {}",
                self.dimensions(),
                other.dimensions()
            )));
        }

        let dot_product: f32 = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm_a = self.l2_norm();
        let norm_b = other.l2_norm();

        if norm_a < 1e-10 || norm_b < 1e-10 {
            return Ok(0.0);
        }

        Ok(dot_product / (norm_a * norm_b))
    }

    /// Calculate Euclidean (L2) distance to another vector
    ///
    /// # Errors
    /// Returns error if vectors have different dimensions
    pub fn euclidean_distance(&self, other: &EmbeddingVector) -> Result<f32> {
        if self.dimensions() != other.dimensions() {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Vector dimension mismatch: {} vs {}",
                self.dimensions(),
                other.dimensions()
            )));
        }

        let sum_sq: f32 = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum();

        Ok(sum_sq.sqrt())
    }

    /// Calculate dot product with another vector
    ///
    /// # Errors
    /// Returns error if vectors have different dimensions
    pub fn dot_product(&self, other: &EmbeddingVector) -> Result<f32> {
        if self.dimensions() != other.dimensions() {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Vector dimension mismatch: {} vs {}",
                self.dimensions(),
                other.dimensions()
            )));
        }

        Ok(self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a * b)
            .sum())
    }

    /// Validate embedding vector
    fn validate(values: &[f32]) -> Result<()> {
        if values.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Embedding vector cannot be empty".to_string(),
            ));
        }

        if values.len() > Self::MAX_DIMENSIONS {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Embedding vector cannot exceed {} dimensions, got {}",
                Self::MAX_DIMENSIONS,
                values.len()
            )));
        }

        // Check for NaN or Infinity
        if values.iter().any(|x| x.is_nan() || x.is_infinite()) {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Embedding vector contains NaN or Infinite values".to_string(),
            ));
        }

        Ok(())
    }
}

impl fmt::Display for EmbeddingVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EmbeddingVector(dims={})", self.dimensions())
    }
}

impl TryFrom<Vec<f32>> for EmbeddingVector {
    type Error = crate::error::AllSourceError;

    fn try_from(values: Vec<f32>) -> Result<Self> {
        EmbeddingVector::new(values)
    }
}

/// Value Object: SimilarityScore
///
/// Represents a similarity score between two vectors.
/// Typically ranges from -1.0 to 1.0 for cosine similarity,
/// or 0.0 to infinity for distance metrics.
///
/// Domain Rules:
/// - Must be a finite number (no NaN or Infinity)
/// - Immutable once created
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SimilarityScore(f32);

impl SimilarityScore {
    /// Create a new SimilarityScore with validation
    pub fn new(value: f32) -> Result<Self> {
        if value.is_nan() || value.is_infinite() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Similarity score must be a finite number".to_string(),
            ));
        }
        Ok(Self(value))
    }

    /// Create SimilarityScore without validation
    pub(crate) fn new_unchecked(value: f32) -> Self {
        Self(value)
    }

    /// Get the score value
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Check if this score represents high similarity (>= threshold)
    pub fn is_similar(&self, threshold: f32) -> bool {
        self.0 >= threshold
    }
}

impl fmt::Display for SimilarityScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

impl From<SimilarityScore> for f32 {
    fn from(score: SimilarityScore) -> f32 {
        score.0
    }
}

/// Distance metric for vector comparisons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DistanceMetric {
    /// Cosine similarity (1 - cosine_distance)
    /// Range: [-1, 1], higher is more similar
    #[default]
    Cosine,

    /// Euclidean (L2) distance
    /// Range: [0, ∞), lower is more similar
    Euclidean,

    /// Dot product (inner product)
    /// Range: (-∞, ∞), higher is more similar for normalized vectors
    DotProduct,
}

impl DistanceMetric {
    /// Calculate similarity/distance between two vectors
    pub fn calculate(
        &self,
        a: &EmbeddingVector,
        b: &EmbeddingVector,
    ) -> Result<SimilarityScore> {
        let value = match self {
            DistanceMetric::Cosine => a.cosine_similarity(b)?,
            DistanceMetric::Euclidean => a.euclidean_distance(b)?,
            DistanceMetric::DotProduct => a.dot_product(b)?,
        };
        SimilarityScore::new(value)
    }

    /// Returns true if higher values mean more similar
    pub fn higher_is_better(&self) -> bool {
        match self {
            DistanceMetric::Cosine => true,
            DistanceMetric::Euclidean => false, // Lower distance = more similar
            DistanceMetric::DotProduct => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_valid_embedding() {
        let embedding = EmbeddingVector::new(vec![0.1, 0.2, 0.3]);
        assert!(embedding.is_ok());
        assert_eq!(embedding.unwrap().dimensions(), 3);
    }

    #[test]
    fn test_reject_empty_embedding() {
        let result = EmbeddingVector::new(vec![]);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn test_reject_too_large_embedding() {
        let large = vec![0.0; EmbeddingVector::MAX_DIMENSIONS + 1];
        let result = EmbeddingVector::new(large);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("cannot exceed"));
        }
    }

    #[test]
    fn test_accept_max_dimensions() {
        let max = vec![0.0; EmbeddingVector::MAX_DIMENSIONS];
        let result = EmbeddingVector::new(max);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_nan_values() {
        let result = EmbeddingVector::new(vec![0.1, f32::NAN, 0.3]);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("NaN"));
        }
    }

    #[test]
    fn test_reject_infinite_values() {
        let result = EmbeddingVector::new(vec![0.1, f32::INFINITY, 0.3]);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Infinite"));
        }
    }

    #[test]
    fn test_l2_norm() {
        let embedding = EmbeddingVector::new(vec![3.0, 4.0]).unwrap();
        assert!((embedding.l2_norm() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let embedding = EmbeddingVector::new(vec![3.0, 4.0]).unwrap();
        let normalized = embedding.normalize();
        assert!(normalized.is_normalized());
        assert!((normalized.values()[0] - 0.6).abs() < 1e-6);
        assert!((normalized.values()[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap();
        let b = EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap();
        let similarity = a.cosine_similarity(&b).unwrap();
        assert!((similarity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
        let b = EmbeddingVector::new(vec![0.0, 1.0]).unwrap();
        let similarity = a.cosine_similarity(&b).unwrap();
        assert!(similarity.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
        let b = EmbeddingVector::new(vec![-1.0, 0.0]).unwrap();
        let similarity = a.cosine_similarity(&b).unwrap();
        assert!((similarity + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_dimension_mismatch() {
        let a = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
        let b = EmbeddingVector::new(vec![1.0, 0.0, 0.0]).unwrap();
        let result = a.cosine_similarity(&b);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("dimension mismatch"));
        }
    }

    #[test]
    fn test_euclidean_distance() {
        let a = EmbeddingVector::new(vec![0.0, 0.0]).unwrap();
        let b = EmbeddingVector::new(vec![3.0, 4.0]).unwrap();
        let distance = a.euclidean_distance(&b).unwrap();
        assert!((distance - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = EmbeddingVector::new(vec![1.0, 2.0, 3.0]).unwrap();
        let b = EmbeddingVector::new(vec![4.0, 5.0, 6.0]).unwrap();
        let product = a.dot_product(&b).unwrap();
        assert!((product - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_display() {
        let embedding = EmbeddingVector::new(vec![0.1, 0.2, 0.3]).unwrap();
        assert_eq!(format!("{}", embedding), "EmbeddingVector(dims=3)");
    }

    #[test]
    fn test_try_from_vec() {
        let result: Result<EmbeddingVector> = vec![0.1, 0.2, 0.3].try_into();
        assert!(result.is_ok());
    }

    #[test]
    fn test_serde_serialization() {
        let embedding = EmbeddingVector::new(vec![0.1, 0.2, 0.3]).unwrap();
        let json = serde_json::to_string(&embedding).unwrap();
        let deserialized: EmbeddingVector = serde_json::from_str(&json).unwrap();
        assert_eq!(embedding, deserialized);
    }

    #[test]
    fn test_similarity_score() {
        let score = SimilarityScore::new(0.95).unwrap();
        assert!((score.value() - 0.95).abs() < 1e-6);
        assert!(score.is_similar(0.9));
        assert!(!score.is_similar(0.99));
    }

    #[test]
    fn test_similarity_score_reject_nan() {
        let result = SimilarityScore::new(f32::NAN);
        assert!(result.is_err());
    }

    #[test]
    fn test_distance_metric_cosine() {
        let a = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
        let b = EmbeddingVector::new(vec![1.0, 0.0]).unwrap();
        let score = DistanceMetric::Cosine.calculate(&a, &b).unwrap();
        assert!((score.value() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_distance_metric_euclidean() {
        let a = EmbeddingVector::new(vec![0.0, 0.0]).unwrap();
        let b = EmbeddingVector::new(vec![3.0, 4.0]).unwrap();
        let score = DistanceMetric::Euclidean.calculate(&a, &b).unwrap();
        assert!((score.value() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_higher_is_better() {
        assert!(DistanceMetric::Cosine.higher_is_better());
        assert!(!DistanceMetric::Euclidean.higher_is_better());
        assert!(DistanceMetric::DotProduct.higher_is_better());
    }
}
