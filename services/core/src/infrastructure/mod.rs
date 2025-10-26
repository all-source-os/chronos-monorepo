// Infrastructure layer - concrete implementations
// This contains:
// - repositories/ (In-memory implementations with SierraDB patterns)
// - persistence/ (ParquetRepository, WALRepository implementations)
// - web/ (HTTP handlers, WebSocket handlers)
// - messaging/ (Kafka, NATS integrations)

pub mod repositories;
