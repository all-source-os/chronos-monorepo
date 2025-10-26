// Infrastructure layer - concrete implementations
// This contains:
// - repositories/ (In-memory implementations with SierraDB patterns)
// - persistence/ (Storage integrity, checksums)
// - web/ (HTTP handlers, WebSocket handlers)
// - messaging/ (Kafka, NATS integrations)

pub mod persistence;
pub mod repositories;
