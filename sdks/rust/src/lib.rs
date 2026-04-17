//! # AllSource Rust SDK
//!
//! Official Rust client for the [AllSource](https://github.com/all-source-os/allsource-monorepo)
//! event store. Provides two clients:
//!
//! - [`QueryClient`] — reads from the Query Service (events, projections, streams)
//! - [`CoreClient`] — writes directly to Core (event ingestion, batch ingest)
//!
//! Plus the [`EventFolder`] trait for reconstructing domain state from event streams.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use allsource::{QueryClient, IngestEventInput};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), allsource::Error> {
//!     let qs = QueryClient::new("http://localhost:3902", "your-api-key")?;
//!
//!     // Query events
//!     let result = qs.query_events(
//!         allsource::QueryEventsParams::new().entity_id("user-123").limit(10),
//!     ).await?;
//!
//!     for event in &result.events {
//!         println!("{}: {}", event.event_type, event.timestamp);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Event Folding
//!
//! ```rust,no_run
//! use allsource::{Event, EventFolder, fold_events};
//!
//! #[derive(Default)]
//! struct UserState {
//!     email: Option<String>,
//!     name: Option<String>,
//!     active: bool,
//! }
//!
//! impl EventFolder for UserState {
//!     type State = Self;
//!
//!     fn apply(&mut self, event: &Event) -> bool {
//!         match event.event_type.as_str() {
//!             "user.created" => {
//!                 self.email = event.payload.get("email").and_then(|v| v.as_str()).map(String::from);
//!                 self.name = event.payload.get("name").and_then(|v| v.as_str()).map(String::from);
//!                 self.active = true;
//!                 true
//!             }
//!             "user.deactivated" => { self.active = false; true }
//!             _ => false,
//!         }
//!     }
//!
//!     fn finalize(self) -> Option<Self::State> {
//!         if self.email.is_some() { Some(self) } else { None }
//!     }
//! }
//! ```

mod circuit_breaker;
mod client;
mod error;
mod fold;
mod normalize;
mod projection_api;
#[cfg(feature = "projection-worker")]
pub mod projection_worker;
mod types;
#[cfg(feature = "ws")]
mod ws;

pub use circuit_breaker::CircuitBreaker;
pub use client::{ClientConfig, CoreClient, QueryClient, RetryConfig};
pub use error::Error;
pub use fold::{fold_events, EventFolder};
pub use normalize::normalize_event_type;
pub use types::*;

#[cfg(feature = "ws")]
pub use ws::{EventStream, EventStreamClient, StreamItem, StreamMode, StreamedEvent};

#[cfg(feature = "projection-worker")]
pub use projection_worker::{ProjectionHandle, ProjectionWorker, ProjectionWorkerBuilder, WorkerState};

// Re-export for payload construction
pub use serde_json::{self, json, Value};
