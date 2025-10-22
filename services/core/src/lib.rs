// Library exports for benchmarks and tests

// Clean Architecture Layers
pub mod domain;           // Layer 1: Enterprise Business Rules
pub mod application;      // Layer 2: Application Business Rules
pub mod infrastructure;   // Layer 3: Interface Adapters

// Legacy modules (to be refactored)
pub mod analytics;
pub mod auth;
pub mod auth_api;
pub mod api;
pub mod api_v1;
pub mod backup;
pub mod compaction;
pub mod config;
pub mod error;
pub mod event;
pub mod index;
pub mod metrics;
pub mod middleware;
pub mod pipeline;
pub mod projection;
pub mod rate_limit;
pub mod replay;
pub mod schema;
pub mod snapshot;
pub mod store;
pub mod storage;
pub mod tenant;
pub mod tenant_api;
pub mod wal;
pub mod websocket;

// Re-export commonly used types
pub use error::{AllSourceError, Result};
pub use event::{Event, IngestEventRequest, QueryEventsRequest};
pub use store::EventStore;

// Re-export domain types
pub use domain::entities;
pub use domain::repositories;
