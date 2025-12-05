// Infrastructure layer - concrete implementations
// This contains:
// - repositories/ (In-memory, PostgreSQL, RocksDB implementations with SierraDB patterns)
// - persistence/ (Storage integrity, checksums, lock-free structures)
// - cluster/ (Node registry, request routing for distributed systems)
// - web/ (HTTP handlers, WebSocket handlers)
// - security/ (Authentication, authorization, rate limiting)
// - config/ (Configuration loading)
// - observability/ (Metrics, tracing)

pub mod cluster;
pub mod config;
pub mod observability;
pub mod persistence;
pub mod repositories;
pub mod security;
pub mod web;
