//! Redis RESP3 protocol compatibility layer.
//!
//! Provides a TCP server that speaks the Redis RESP3 wire protocol, enabling
//! `redis-cli` and Redis client libraries to interact with AllSource Core.
//!
//! ## Command Mapping
//!
//! | Redis Command | AllSource Operation |
//! |---------------|---------------------|
//! | `XADD <tenant> * event_type <t> entity_id <id> [payload <json>]` | `EventStore::ingest()` |
//! | `XRANGE <stream_key> - + [COUNT n]` | `EventStore::query()` |
//! | `XLEN <stream_key>` | event count |
//! | `SUBSCRIBE <channel>` | real-time event broadcast |
//! | `PING` | health check |
//! | `INFO` | server info |
//!
//! ## Stream Key Conventions
//!
//! - Plain name (e.g., `default`) → filters by tenant_id
//! - `entity:<id>` → filters by entity_id
//! - `type:<event_type>` → filters by event_type
//!
//! ## Configuration
//!
//! Set `ALLSOURCE_RESP_PORT` to enable the RESP3 server (default: disabled).
//! When set, the server listens on that port alongside the HTTP API.

pub mod commands;
pub mod protocol;
pub mod server;

pub use server::RespServer;
