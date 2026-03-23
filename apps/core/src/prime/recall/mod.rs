//! AllSource Recall — agent memory API built on Prime.
//!
//! Provides compressed index generation, semantic vector search,
//! temporal graph queries, and memory lifecycle (remember/forget/decay).

pub mod api;
pub mod compressor;
pub mod index_builder;
pub mod ollama;
pub mod types;

pub use api::{RecallContextQuery, RecallDeps, RecallEngine};
pub use compressor::IndexCompressor;
pub use index_builder::{build_heuristic_index, build_raw_summary};
pub use ollama::OllamaBackend;
pub use types::{
    CompressedIndex, ContextTier, IndexConfig, LlmBackend, MemoryPayload, RankedMemory,
    RecallContext, estimate_tokens, recall_event_types,
};
