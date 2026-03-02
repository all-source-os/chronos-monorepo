//! Wire types for the HTTP sync protocol.
//!
//! Used by [`SyncClient`](super::sync_transport::SyncClient) and the
//! server-side `/api/v1/sync/pull` and `/api/v1/sync/push` endpoints.

use crate::infrastructure::cluster::{crdt::ReplicatedEvent, hlc::HlcTimestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Request body for `POST /api/v1/sync/pull`.
///
/// The client sends its node ID and version vector; the server returns
/// all events the client hasn't seen yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPullRequest {
    pub node_id: String,
    pub version_vector: BTreeMap<String, HlcTimestamp>,
}

/// Response body for `POST /api/v1/sync/pull`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPullResponse {
    pub events: Vec<ReplicatedEvent>,
    pub version_vector: BTreeMap<String, HlcTimestamp>,
}

/// Request body for `POST /api/v1/sync/push`.
///
/// The client sends events it wants to replicate to the server, along
/// with its current version vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPushRequest {
    pub node_id: String,
    pub events: Vec<ReplicatedEvent>,
    pub version_vector: BTreeMap<String, HlcTimestamp>,
}

/// Response body for `POST /api/v1/sync/push`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPushResponse {
    pub accepted: usize,
    pub skipped: usize,
    pub version_vector: BTreeMap<String, HlcTimestamp>,
}

/// Statistics from a full bidirectional sync operation.
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    /// Number of events pushed to the remote.
    pub pushed: usize,
    /// Number of events pulled from the remote.
    pub pulled: usize,
    /// Number of events skipped due to CRDT conflict resolution.
    pub conflicts: usize,
}
