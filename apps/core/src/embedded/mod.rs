//! Ergonomic embedded-mode facade for AllSource Core.
//!
//! Use this module to embed AllSource Core directly in your Rust application
//! as a library — no HTTP server required.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent, Query};
//! use serde_json::json;
//!
//! # #[tokio::main]
//! # async fn main() -> allsource_core::error::Result<()> {
//! let core = EmbeddedCore::open(Config::builder().build()?).await?;
//!
//! core.ingest(IngestEvent {
//!     entity_id: "order-1",
//!     event_type: "order.placed",
//!     payload: json!({"total": 99.99}),
//!     metadata: None,
//!     tenant_id: None,
//! }).await?;
//!
//! let events = core.query(Query::new().entity_id("order-1")).await?;
//! core.shutdown().await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "embedded-projections")]
pub mod ai_projections;
mod config;
mod core;
#[cfg(feature = "embedded-replicant")]
pub mod replicant;
mod types;

pub use self::core::EmbeddedCore;
pub use config::{ConfigBuilder, EmbeddedConfig as Config};
pub use types::{EventView, IngestEvent, Query};
