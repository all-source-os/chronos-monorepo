// Infrastructure layer - concrete implementations
// This contains:
// - repositories/ (In-memory, PostgreSQL, RocksDB implementations with SierraDB patterns)
// - persistence/ (Storage integrity, checksums, lock-free structures)
// - cluster/ (Node registry, request routing for distributed systems)
// - web/ (HTTP handlers, WebSocket handlers)
// - security/ (Authentication, authorization, rate limiting)
// - config/ (Configuration loading)
// - observability/ (Metrics, tracing)
// - search/ (Vector search engine with HNSW indexing)
// - di/ (Dependency injection container)

pub mod cluster;
pub mod config;
pub mod di;
pub mod observability;
pub mod persistence;
pub mod query;
pub mod replication;
pub mod repositories;
#[cfg(feature = "server")]
pub mod resp;
pub mod search;
pub mod security;
#[cfg(feature = "server")]
pub mod web;
