// Library exports for benchmarks and tests

pub mod analytics;
pub mod compaction;
pub mod error;
pub mod event;
pub mod index;
pub mod pipeline;
pub mod projection;
pub mod replay;
pub mod schema;
pub mod snapshot;
pub mod store;
pub mod storage;
pub mod wal;
pub mod websocket;

// Re-export commonly used types
pub use error::{AllSourceError, Result};
pub use event::{Event, IngestEventRequest, QueryEventsRequest};
pub use store::EventStore;
