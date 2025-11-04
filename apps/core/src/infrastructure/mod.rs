// Infrastructure layer - concrete implementations
// This contains:
// - repositories/ (In-memory, PostgreSQL, RocksDB implementations with SierraDB patterns)
// - persistence/ (Storage integrity, checksums, lock-free structures)
// - cluster/ (Node registry, request routing for distributed systems)
// - web/ (HTTP handlers, WebSocket handlers)
// - messaging/ (Kafka, NATS integrations)

pub mod persistence;
pub mod repositories;
pub mod cluster;
pub mod security;
