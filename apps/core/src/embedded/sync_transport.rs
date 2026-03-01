//! HTTP-based sync transport for desktop↔cloud bidirectional sync.
//!
//! [`SyncClient`] orchestrates a full pull-then-push sync cycle against a
//! remote AllSource Core server via the `/api/v1/sync/pull` and
//! `/api/v1/sync/push` endpoints.
//!
//! # Example
//!
//! ```rust,no_run
//! use allsource_core::embedded::sync_transport::SyncClient;
//! use allsource_core::embedded::{Config, EmbeddedCore};
//!
//! # #[tokio::main]
//! # async fn main() -> allsource_core::error::Result<()> {
//! let core = EmbeddedCore::open(
//!     Config::builder().node_id(1).build()?
//! ).await?;
//!
//! let client = SyncClient::new("https://cloud.example.com", "node-1");
//! let stats = client.sync(&core).await?;
//! println!("pushed: {}, pulled: {}, conflicts: {}", stats.pushed, stats.pulled, stats.conflicts);
//! # Ok(())
//! # }
//! ```

use super::sync_types::*;
use crate::error::Result;

/// HTTP sync client for bidirectional sync with a remote Core server.
pub struct SyncClient {
    base_url: String,
    node_id: String,
    http: reqwest::Client,
}

impl SyncClient {
    /// Create a new sync client targeting the given server base URL.
    pub fn new(base_url: impl Into<String>, node_id: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            node_id: node_id.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Perform a full bidirectional sync: pull then push.
    ///
    /// Returns aggregate statistics for the sync operation.
    pub async fn sync(
        &self,
        core: &super::core::EmbeddedCore,
    ) -> Result<SyncStats> {
        let mut stats = SyncStats::default();

        // Phase 1: Pull — get events we haven't seen from the server
        let pull_stats = self.pull(core).await?;
        stats.pulled = pull_stats.pulled;
        stats.conflicts += pull_stats.conflicts;

        // Phase 2: Push — send events the server hasn't seen
        let push_stats = self.push(core).await?;
        stats.pushed = push_stats.pushed;
        stats.conflicts += push_stats.conflicts;

        Ok(stats)
    }

    /// Pull events from the remote server into the local core.
    async fn pull(
        &self,
        core: &super::core::EmbeddedCore,
    ) -> Result<SyncStats> {
        let vv = core.version_vector();
        let request = SyncPullRequest {
            node_id: self.node_id.clone(),
            version_vector: vv,
        };

        let url = format!("{}/api/v1/sync/pull", self.base_url);
        let response: SyncPullResponse = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!("sync pull HTTP failed: {e}"))
            })?
            .json()
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!(
                    "sync pull response parse failed: {e}"
                ))
            })?;

        let total = response.events.len();
        let (accepted, skipped) = core.receive_sync_push(response.events).await?;

        Ok(SyncStats {
            pulled: accepted,
            pushed: 0,
            conflicts: skipped,
        })
    }

    /// Push local events to the remote server.
    async fn push(
        &self,
        core: &super::core::EmbeddedCore,
    ) -> Result<SyncStats> {
        // Get the server's version vector first (via a zero-event pull)
        let server_vv = self.fetch_server_version_vector().await?;

        // Get local events that the server hasn't seen
        let events = core.events_for_sync(&server_vv).await?;

        if events.is_empty() {
            return Ok(SyncStats::default());
        }

        let request = SyncPushRequest {
            node_id: self.node_id.clone(),
            events,
            version_vector: core.version_vector(),
        };

        let url = format!("{}/api/v1/sync/push", self.base_url);
        let response: SyncPushResponse = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!("sync push HTTP failed: {e}"))
            })?
            .json()
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!(
                    "sync push response parse failed: {e}"
                ))
            })?;

        Ok(SyncStats {
            pushed: response.accepted,
            pulled: 0,
            conflicts: response.skipped,
        })
    }

    /// Fetch the server's current version vector via a pull with empty VV.
    async fn fetch_server_version_vector(
        &self,
    ) -> Result<std::collections::BTreeMap<String, crate::infrastructure::cluster::hlc::HlcTimestamp>>
    {
        let request = SyncPullRequest {
            node_id: self.node_id.clone(),
            version_vector: std::collections::BTreeMap::new(),
        };

        let url = format!("{}/api/v1/sync/pull", self.base_url);
        let response: SyncPullResponse = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!(
                    "sync pull (VV fetch) HTTP failed: {e}"
                ))
            })?
            .json()
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!(
                    "sync pull (VV fetch) response parse failed: {e}"
                ))
            })?;

        Ok(response.version_vector)
    }
}
