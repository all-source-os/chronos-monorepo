// Library exports for benchmarks and tests

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
