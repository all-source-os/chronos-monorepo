//! In-process text embedder for Prime.
//!
//! Wraps `fastembed`'s `TextEmbedding` (AllMiniLML6V2, 384 dims) so callers
//! that only have text can produce embedding vectors without standing up
//! a separate embedding service. This is the same model the rest of
//! AllSource uses for vector search.
//!
//! The model files are auto-downloaded into the `fastembed` cache on first
//! use (`.fastembed_cache/` by default; configurable via `FASTEMBED_CACHE_PATH`).

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use parking_lot::Mutex;

use crate::prime::error::{PrimeError, PrimeResult};

/// Output dimensionality of the default embedding model (AllMiniLML6V2).
pub const DEFAULT_EMBEDDING_DIMENSIONS: usize = 384;

/// Text → vector embedder used by `Prime::embed_text`.
///
/// Thread-safe: the underlying `TextEmbedding` is wrapped in a mutex because
/// `fastembed`'s model is `!Sync` during inference.
pub struct TextEmbedder {
    model: Mutex<TextEmbedding>,
    dimensions: usize,
}

impl TextEmbedder {
    /// Initialize the default embedder (`AllMiniLML6V2`, 384 dims).
    ///
    /// On first use the model is downloaded into the fastembed cache.
    /// Subsequent calls reuse the cached files.
    pub fn new() -> PrimeResult<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
        )
        .map_err(|e| {
            PrimeError::CoreError(anyhow::anyhow!("failed to initialize embedder: {e}"))
        })?;

        Ok(Self {
            model: Mutex::new(model),
            dimensions: DEFAULT_EMBEDDING_DIMENSIONS,
        })
    }

    /// Embed a single string. Returns a `dimensions()`-length vector.
    pub fn embed(&self, text: &str) -> PrimeResult<Vec<f32>> {
        let mut out = self
            .model
            .lock()
            .embed(vec![text], None)
            .map_err(|e| PrimeError::CoreError(anyhow::anyhow!("embedding failed: {e}")))?;

        out.pop()
            .ok_or_else(|| PrimeError::CoreError(anyhow::anyhow!("embedder produced no output")))
    }

    /// Embedding dimensionality for the configured model.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests download the embedding model on first run (~25 MB) and
    // need network + a writable fastembed cache. Marked `#[ignore]` so the
    // default `cargo test` is offline-safe; run with `cargo test -- --ignored`.

    #[test]
    #[ignore = "downloads model — run with --ignored"]
    fn embed_returns_expected_dimensions() {
        let embedder = TextEmbedder::new().expect("embedder init");
        let v = embedder.embed("hello world").expect("embed");
        assert_eq!(v.len(), DEFAULT_EMBEDDING_DIMENSIONS);
        assert_eq!(v.len(), embedder.dimensions());
    }

    #[test]
    #[ignore = "downloads model — run with --ignored"]
    fn embed_is_deterministic_for_same_input() {
        let embedder = TextEmbedder::new().expect("embedder init");
        let a = embedder.embed("test sentence").expect("embed a");
        let b = embedder.embed("test sentence").expect("embed b");
        assert_eq!(a, b);
    }

    #[test]
    #[ignore = "downloads model — run with --ignored"]
    fn embed_differs_for_different_input() {
        let embedder = TextEmbedder::new().expect("embedder init");
        let a = embedder.embed("project status update").expect("embed a");
        let b = embedder.embed("apple pie recipe").expect("embed b");
        assert_ne!(a, b);
    }
}
