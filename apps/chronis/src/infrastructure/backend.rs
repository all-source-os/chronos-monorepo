use std::sync::Arc;

use allsource_core::embedded::{EmbeddedCore, IngestEvent, Query};

use super::http_core_client::{HttpCoreClient, RemoteEvent};
use crate::domain::error::{ChronError, CoreError};

/// Abstracts over embedded and remote Core backends.
pub enum CoreBackend {
    Embedded(Arc<EmbeddedCore>),
    /// Remote mode: writes go to the HTTP client, reads use a local
    /// in-memory EmbeddedCore that caches events for projections.
    Remote {
        client: HttpCoreClient,
        local: Arc<EmbeddedCore>,
    },
}

impl CoreBackend {
    pub async fn ingest(&self, event: IngestEvent<'_>) -> Result<(), ChronError> {
        match self {
            Self::Embedded(core) => core
                .ingest(event)
                .await
                .map_err(|e| CoreError(e.to_string()).into()),
            Self::Remote { client, local } => {
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
                // Replay into local Core so projections stay current
                local
                    .ingest(event)
                    .await
                    .map_err(|e| CoreError(e.to_string()).into())
            }
        }
    }

    pub async fn query(&self, query: Query) -> Result<Vec<RemoteEvent>, ChronError> {
        match self {
            Self::Embedded(core) => {
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
            Self::Embedded(core) => core.projection(name, key),
            Self::Remote { local, .. } => local.projection(name, key),
        }
    }

    /// Returns the embedded Core, if this is an embedded backend.
    pub fn as_embedded(&self) -> Option<&Arc<EmbeddedCore>> {
        match self {
            Self::Embedded(core) => Some(core),
            Self::Remote { .. } => None,
        }
    }
}
