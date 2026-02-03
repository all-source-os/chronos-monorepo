//! Search Infrastructure
//!
//! This module provides search engines for event queries:
//! - VectorSearchEngine: Semantic similarity search using embeddings and HNSW
//! - KeywordSearchEngine: Full-text BM25 keyword search using Tantivy
//! - HybridSearchEngine: Combined semantic and keyword search orchestrator

mod hybrid_search_engine;
mod keyword_search_engine;
mod vector_search_engine;

pub use hybrid_search_engine::*;
pub use keyword_search_engine::*;
pub use vector_search_engine::*;
