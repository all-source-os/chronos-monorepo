use std::sync::Arc;

use allsource_core::embedded::{EmbeddedCore, IngestEvent, Query};
use tokio::sync::broadcast;

use super::http_core_client::{HttpCoreClient, RemoteEvent};
use crate::domain::error::{ChronError, CoreError};

/// A lightweight signal that something changed in the underlying Core.
///
/// Carries just enough metadata for consumers (TUI refresh, SSE emitter)
/// to decide whether to re-query. We intentionally avoid shipping full
/// payloads through this channel — consumers pull fresh projections.
#[derive(Debug, Clone)]
pub struct ChangeEvent {
    pub entity_id: String,
    pub event_type: String,
}

/// Abstracts over embedded and remote Core backends.
pub enum CoreBackend {
    /// Embedded mode: chronis owns an in-process EmbeddedCore. The
    /// `change_tx` channel is fed by (a) a forwarder task that taps the
    /// EmbeddedCore broadcaster for in-process writes, and (b) an optional
    /// WAL watcher (see `embedded_wal_tail`) for cross-process writes from
    /// other chronis instances sharing the same `.chronis/` directory.
    Embedded {
        core: Arc<EmbeddedCore>,
        change_tx: broadcast::Sender<ChangeEvent>,
    },
    /// Remote mode: writes go to the HTTP client, reads use a local
    /// in-memory EmbeddedCore that caches events for projections.
    Remote {
        client: HttpCoreClient,
        local: Arc<EmbeddedCore>,
        /// Broadcast of change notifications fed by the WS client task.
        change_tx: broadcast::Sender<ChangeEvent>,
    },
}

impl CoreBackend {
    pub fn new_embedded(core: Arc<EmbeddedCore>) -> Self {
        // Capacity 1024 matches Core's internal broadcast capacity so we
        // surface lag on the same boundary the upstream does.
        let (change_tx, _) = broadcast::channel(1024);

        // Forwarder task: tap the in-process EmbeddedCore broadcaster and
        // republish into change_tx. Lets us merge in-process and WAL-tail
        // signals into a single stream.
        let mut core_rx = core.subscribe_events();
        let fwd_tx = change_tx.clone();
        tokio::spawn(async move {
            loop {
                match core_rx.recv().await {
                    Ok(event) => {
                        let _ = fwd_tx.send(ChangeEvent {
                            entity_id: event.entity_id_str().to_string(),
                            event_type: event.event_type_str().to_string(),
                        });
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let _ = fwd_tx.send(ChangeEvent {
                            entity_id: String::new(),
                            event_type: "chronis.lagged".to_string(),
                        });
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        Self::Embedded { core, change_tx }
    }

    pub fn new_remote(client: HttpCoreClient, local: Arc<EmbeddedCore>) -> Self {
        let (change_tx, _) = broadcast::channel(1024);
        Self::Remote {
            client,
            local,
            change_tx,
        }
    }

    pub async fn ingest(&self, event: IngestEvent<'_>) -> Result<(), ChronError> {
        match self {
            Self::Embedded { core, .. } => core
                .ingest(event)
                .await
                .map_err(|e| CoreError(e.to_string()).into()),
            Self::Remote { client, local, .. } => {
                // Send to remote first (source of truth)
                client
                    .ingest_event(
                        event.entity_id,
                        event.event_type,
                        &event.payload,
                        event.metadata.as_ref(),
                        event.tenant_id,
                    )
                    .await?;
                // Replay into local Core so projections stay current.
                local
                    .ingest(event)
                    .await
                    .map_err(|e| CoreError(e.to_string()).into())
            }
        }
    }

    pub async fn query(&self, query: Query) -> Result<Vec<RemoteEvent>, ChronError> {
        match self {
            Self::Embedded { core, .. } => {
                let events = core
                    .query(query)
                    .await
                    .map_err(|e| CoreError(e.to_string()))?;
                Ok(events
                    .into_iter()
                    .map(RemoteEvent::from_event_view)
                    .collect())
            }
            Self::Remote { local, .. } => {
                // Query the local cache — it has all events replayed from remote
                let events = local
                    .query(query)
                    .await
                    .map_err(|e| CoreError(e.to_string()))?;
                Ok(events
                    .into_iter()
                    .map(RemoteEvent::from_event_view)
                    .collect())
            }
        }
    }

    pub fn projection(&self, name: &str, key: &str) -> Option<serde_json::Value> {
        match self {
            Self::Embedded { core, .. } => core.projection(name, key),
            Self::Remote { local, .. } => local.projection(name, key),
        }
    }

    /// Returns the embedded Core, if this is an embedded backend.
    pub fn as_embedded(&self) -> Option<&Arc<EmbeddedCore>> {
        match self {
            Self::Embedded { core, .. } => Some(core),
            Self::Remote { .. } => None,
        }
    }

    /// Subscribe to live change notifications from the backing Core.
    ///
    /// Both backend variants forward through a single `change_tx` channel,
    /// so the subscription type is uniform. Lagging subscribers receive
    /// `SubscribeError::Lagged(n)` — treat as "something changed, re-query".
    pub fn subscribe(&self) -> BackendSubscription {
        let rx = match self {
            Self::Embedded { change_tx, .. } => change_tx.subscribe(),
            Self::Remote { change_tx, .. } => change_tx.subscribe(),
        };
        BackendSubscription { rx }
    }

    /// Internal: sender used by the WS client task to publish into the
    /// Remote change channel.
    pub(crate) fn remote_change_sender(&self) -> Option<broadcast::Sender<ChangeEvent>> {
        match self {
            Self::Remote { change_tx, .. } => Some(change_tx.clone()),
            _ => None,
        }
    }

    /// Internal: sender + core handle for the embedded WAL-tail task.
    pub(crate) fn embedded_change_sender(
        &self,
    ) -> Option<(broadcast::Sender<ChangeEvent>, Arc<EmbeddedCore>)> {
        match self {
            Self::Embedded { core, change_tx } => Some((change_tx.clone(), Arc::clone(core))),
            _ => None,
        }
    }
}

/// A subscription handle backed by a single broadcast receiver.
pub struct BackendSubscription {
    rx: broadcast::Receiver<ChangeEvent>,
}

impl BackendSubscription {
    /// Wait for the next change notification.
    pub async fn recv_change(&mut self) -> Result<ChangeEvent, SubscribeError> {
        match self.rx.recv().await {
            Ok(change) => Ok(change),
            Err(broadcast::error::RecvError::Lagged(n)) => Err(SubscribeError::Lagged(n)),
            Err(broadcast::error::RecvError::Closed) => Err(SubscribeError::Closed),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SubscribeError {
    /// Receiver fell behind by `n` messages. Treat as a "something changed" nudge.
    Lagged(u64),
    /// Sender dropped — subscription is dead.
    Closed,
}
