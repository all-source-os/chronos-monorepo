//! Vendored `all-MiniLM-L6-v2` model weights for AllSource Prime.
//!
//! The five files are fetched once at **build time** (see `build.rs`) into
//! `OUT_DIR` and `include_bytes!`'d here, so they are baked into the consuming
//! binary. Prime's embedder loads from these bytes via fastembed's
//! `try_new_from_user_defined` and **never touches the network at runtime**.
//!
//! This crate is intentionally dependency-free at runtime — it exposes raw
//! bytes so the consumer (which already has fastembed) builds the model. The
//! model is 384-dimensional, mean-pooled, non-quantized.
//!
//! ```ignore
//! let onnx = allsource_prime_models::onnx();
//! let tok  = allsource_prime_models::tokenizer_json();
//! // → feed into fastembed UserDefinedEmbeddingModel
//! ```

/// Embedding dimensionality of the bundled model.
pub const DIMENSIONS: usize = 384;

/// HuggingFace repo + pinned revision the bytes were fetched from (provenance).
pub const SOURCE_REPO: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
pub const SOURCE_REVISION: &str = "5f1b8cd78bc4fb444dd171e59b18f3a3af89a079";

/// The ONNX model graph.
pub fn onnx() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/model.onnx"))
}

/// `tokenizer.json` — the HuggingFace fast-tokenizer definition.
pub fn tokenizer_json() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/tokenizer.json"))
}

/// `config.json` — model config (holds `pad_token_id`).
pub fn config_json() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/config.json"))
}

/// `special_tokens_map.json` — special-token definitions.
pub fn special_tokens_map_json() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/special_tokens_map.json"))
}

/// `tokenizer_config.json` — tokenizer config (holds `model_max_length`, `pad_token`).
pub fn tokenizer_config_json() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/tokenizer_config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_files_present_and_nonempty() {
        assert!(onnx().len() > 1_000_000, "onnx looks too small");
        assert!(!tokenizer_json().is_empty());
        assert!(!config_json().is_empty());
        assert!(!special_tokens_map_json().is_empty());
        assert!(!tokenizer_config_json().is_empty());
    }
}
