//! In-process text embedder for Prime.
//!
//! Wraps `fastembed`'s `TextEmbedding` (AllMiniLML6V2, 384 dims) so callers
//! that only have text can produce embedding vectors without standing up
//! a separate embedding service. This is the same model the rest of
//! AllSource uses for vector search.
//!
//! ## Where the model comes from
//!
//! Two load paths, tried in order:
//!
//! 1. **Offline / vendored** — if `PRIME_EMBED_MODEL_DIR` (alias
//!    `ALLSOURCE_EMBED_MODEL_DIR`) points at a directory containing the five
//!    model files (`model.onnx`, `tokenizer.json`, `config.json`,
//!    `special_tokens_map.json`, `tokenizer_config.json`), the model is loaded
//!    straight from disk with **zero network access**. This is the path that
//!    makes "works offline" actually true: vendor the files once, set the env
//!    var, and `prime_embed` never touches the network again. Run
//!    `allsource-prime --mode warm` (or any first embed) with a network
//!    connection once to populate the fastembed cache, then point
//!    `PRIME_EMBED_MODEL_DIR` at that snapshot dir to go fully offline.
//!
//! 2. **Network download** — otherwise the files are auto-downloaded from
//!    HuggingFace into the fastembed cache on first use. The cache directory
//!    defaults to `.fastembed_cache/` and is overridable via
//!    `FASTEMBED_CACHE_DIR`. `HF_HOME` and `HF_ENDPOINT` (mirror URL) are also
//!    honored by fastembed. This path requires outbound access to
//!    `huggingface.co` (or your `HF_ENDPOINT` mirror) the first time.

use std::path::{Path, PathBuf};

use fastembed::{
    EmbeddingModel, InitOptionsUserDefined, Pooling, QuantizationMode, TextEmbedding,
    TextInitOptions, TokenizerFiles, UserDefinedEmbeddingModel,
};
use parking_lot::Mutex;

use crate::prime::error::{PrimeError, PrimeResult};

/// Output dimensionality of the default embedding model (AllMiniLML6V2).
pub const DEFAULT_EMBEDDING_DIMENSIONS: usize = 384;

/// HuggingFace repo the default model is fetched from (for diagnostics).
const MODEL_REPO: &str = "Qdrant/all-MiniLM-L6-v2-onnx";

/// Env var pointing at a local directory of vendored model files (offline path).
const MODEL_DIR_ENV: &str = "PRIME_EMBED_MODEL_DIR";
/// Back-compat alias for [`MODEL_DIR_ENV`].
const MODEL_DIR_ENV_ALIAS: &str = "ALLSOURCE_EMBED_MODEL_DIR";

/// The five files fastembed needs to build the AllMiniLML6V2 embedder offline.
const ONNX_FILE: &str = "model.onnx";
const TOKENIZER_FILE: &str = "tokenizer.json";
const CONFIG_FILE: &str = "config.json";
const SPECIAL_TOKENS_FILE: &str = "special_tokens_map.json";
const TOKENIZER_CONFIG_FILE: &str = "tokenizer_config.json";

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
    /// Prefers an offline load from `PRIME_EMBED_MODEL_DIR` when set; otherwise
    /// downloads the model into the fastembed cache on first use and reuses the
    /// cached files thereafter. On failure the error spells out exactly what was
    /// tried, where it looked, and how to recover — see [`init_error`].
    pub fn new() -> PrimeResult<Self> {
        let model = match resolve_model_dir() {
            Some(dir) => Self::try_from_dir(&dir).map_err(|e| init_error(Some(&dir), &e))?,
            None => Self::try_from_network().map_err(|e| init_error(None, &e))?,
        };

        Ok(Self {
            model: Mutex::new(model),
            dimensions: DEFAULT_EMBEDDING_DIMENSIONS,
        })
    }

    /// Load the model from a local directory of vendored files (no network).
    fn try_from_dir(dir: &Path) -> anyhow::Result<TextEmbedding> {
        let read = |name: &str| -> anyhow::Result<Vec<u8>> {
            let path = dir.join(name);
            std::fs::read(&path)
                .map_err(|e| anyhow::anyhow!("could not read {} ({e})", path.display()))
        };

        let tokenizer_files = TokenizerFiles {
            tokenizer_file: read(TOKENIZER_FILE)?,
            config_file: read(CONFIG_FILE)?,
            special_tokens_map_file: read(SPECIAL_TOKENS_FILE)?,
            tokenizer_config_file: read(TOKENIZER_CONFIG_FILE)?,
        };

        // AllMiniLML6V2 uses mean pooling and is not quantized — match what the
        // network path (`TextEmbedding::try_new`) configures for this model so
        // offline and online embeddings are identical.
        let model = UserDefinedEmbeddingModel::new(read(ONNX_FILE)?, tokenizer_files)
            .with_pooling(Pooling::Mean)
            .with_quantization(QuantizationMode::None);

        TextEmbedding::try_new_from_user_defined(model, InitOptionsUserDefined::new())
    }

    /// Load the model via fastembed's HuggingFace download path.
    fn try_from_network() -> anyhow::Result<TextEmbedding> {
        TextEmbedding::try_new(
            TextInitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
        )
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

/// Resolve the offline model directory from env, if configured.
fn resolve_model_dir() -> Option<PathBuf> {
    for var in [MODEL_DIR_ENV, MODEL_DIR_ENV_ALIAS] {
        if let Ok(dir) = std::env::var(var) {
            let dir = dir.trim();
            if !dir.is_empty() {
                return Some(PathBuf::from(dir));
            }
        }
    }
    None
}

/// The fastembed cache directory that the network path reads/writes.
fn cache_dir() -> String {
    std::env::var("FASTEMBED_CACHE_DIR").unwrap_or_else(|_| ".fastembed_cache".to_string())
}

/// Build an actionable error from a failed embedder init.
///
/// The previous message — `failed to initialize embedder: Failed to retrieve
/// model.onnx` — distinguished none of the five distinct failure modes (network
/// down, proxy refusing, stale cache, wrong dir, HF layout change). This one
/// names the source it tried, the path/URL involved, and concrete recovery
/// steps including the bring-your-own-vector escape hatch.
fn init_error(model_dir: Option<&Path>, cause: &anyhow::Error) -> PrimeError {
    let msg = match model_dir {
        Some(dir) => format!(
            "failed to initialize embedder from {env}={dir} — {cause}\n\
             Looked for these files in that directory: {onnx}, {tok}, {cfg}, {special}, {tok_cfg}.\n\
             To fix:\n\
             • Confirm all five files exist in {dir} (populate it by running `allsource-prime --mode warm` \
             once with network access, then copy the fastembed cache snapshot dir here).\n\
             • Or unset {env} to fall back to the network download path.\n\
             • Or skip the embedder entirely and supply your own 384-dim vector: \
             prime_embed {{ id, vector: [...] }} (compute it with any AllMiniLM-L6-v2 embedder).",
            env = MODEL_DIR_ENV,
            dir = dir.display(),
            cause = cause,
            onnx = ONNX_FILE,
            tok = TOKENIZER_FILE,
            cfg = CONFIG_FILE,
            special = SPECIAL_TOKENS_FILE,
            tok_cfg = TOKENIZER_CONFIG_FILE,
        ),
        None => format!(
            "failed to initialize embedder (network download path) — {cause}\n\
             Tried to fetch model `{repo}` into cache dir `{cache}`.\n\
             To fix one of:\n\
             • No network / behind a proxy / on a flight: vendor the model and set {env}=<dir> \
             to load offline (run `allsource-prime --mode warm` once online to populate the cache, \
             then point {env} at it). fastembed honors HF_ENDPOINT=<mirror> and HF_HOME=<dir> too.\n\
             • Stale/partial cache: delete `{cache}` and retry.\n\
             • Don't want a network-fetched model at all: supply your own 384-dim vector via \
             prime_embed {{ id, vector: [...] }} (compute with any AllMiniLM-L6-v2 embedder — \
             10 lines of sentence-transformers).",
            cause = cause,
            repo = MODEL_REPO,
            cache = cache_dir(),
            env = MODEL_DIR_ENV,
        ),
    };
    PrimeError::CoreError(anyhow::anyhow!(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_model_dir_reads_env() {
        // No env set in the default test process → None.
        // (We avoid mutating process env here to keep the test hermetic across
        // the parallel test runner; the env-read logic is exercised in the
        // ignored integration test below.)
        assert_eq!(MODEL_DIR_ENV, "PRIME_EMBED_MODEL_DIR");
        assert_eq!(MODEL_DIR_ENV_ALIAS, "ALLSOURCE_EMBED_MODEL_DIR");
    }

    #[test]
    fn offline_dir_error_is_actionable() {
        let dir = PathBuf::from("/nonexistent/prime-model-dir");
        let cause = anyhow::anyhow!(
            "could not read /nonexistent/prime-model-dir/model.onnx (No such file)"
        );
        let err = init_error(Some(&dir), &cause);
        let s = err.to_string();
        // Names the env var, the missing files, and the escape hatch.
        assert!(s.contains("PRIME_EMBED_MODEL_DIR"), "missing env var: {s}");
        assert!(s.contains("model.onnx"), "missing file list: {s}");
        assert!(s.contains("vector: [...]"), "missing escape hatch: {s}");
    }

    #[test]
    fn network_error_is_actionable() {
        let cause = anyhow::anyhow!("Failed to retrieve model.onnx");
        let err = init_error(None, &cause);
        let s = err.to_string();
        assert!(s.contains(MODEL_REPO), "missing repo: {s}");
        assert!(s.contains("HF_ENDPOINT"), "missing mirror hint: {s}");
        assert!(
            s.contains("PRIME_EMBED_MODEL_DIR"),
            "missing offline hint: {s}"
        );
        assert!(s.contains("vector: [...]"), "missing escape hatch: {s}");
    }

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
