// Library exports for benchmarks and tests

pub mod error;
pub mod event;
pub mod index;
pub mod projection;
pub mod store;

// Re-export commonly used types
pub use error::{AllSourceError, Result};
pub use event::{Event, IngestEventRequest, QueryEventsRequest};
pub use store::EventStore;
