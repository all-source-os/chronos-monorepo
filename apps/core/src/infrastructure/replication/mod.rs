//! Leader-follower replication via WAL shipping.
//!
//! When `ALLSOURCE_REPLICATION_ENABLED=true` on a leader node, the WAL shipper
//! opens a TCP listener on `ALLSOURCE_REPLICATION_PORT` (default 3910) and
//! streams WAL entries to connected followers using newline-delimited JSON.
//!
//! Followers use the WAL receiver to connect, subscribe, validate, and replay
//! entries into their local EventStore.

pub mod protocol;
pub mod wal_receiver;
pub mod wal_shipper;

pub use wal_receiver::{FollowerReplicationStatus, WalReceiver};
pub use wal_shipper::{ReplicationMode, ReplicationStatus, WalShipper};
