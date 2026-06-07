//! `HostedPrime` — a stateless, tenant-scoped Prime engine over a remote Core.
//!
//! This is the composition the hosted `allsource-prime` app uses instead of the
//! embedded [`Prime`](super::facade::Prime): it owns no durable store. Reads
//! resolve a tenant's warm [`GraphProjections`] from the
//! [`TenantProjectionCache`] (hydrated on demand from the remote Core's
//! `prime.*` events); writes ingest an event to the remote Core via an
//! [`HttpCore`] scoped to the tenant and then update the warm bundle so reads
//! stay current.
//!
//! Tenant identity is always an explicit argument supplied by the trusted
//! caller (the gateway), never inferred — so one `HostedPrime` serves every
//! tenant with strict isolation (each tenant's events are queried and folded
//! separately; see the cross-tenant test in [`super::tenant_cache`]).
//!
//! Deliberately a distinct type from the embedded `Prime` (not a new variant of
//! it) so the embedded local-first path and `facade.rs` are untouched. The
//! MCP/HTTP surface is adapted onto this in a later slice.
//!
//! Feature-gated behind `prime-recall`. See bead t-10f876 /
//! `docs/proposals/PRIME_STATELESS_OVER_CORE.md`.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use crate::{
    embedded::{EventView, IngestEvent},
    error::Result,
    prime::{
        event_store::EventStore,
        http_core::HttpCore,
        projection_bundle::GraphProjections,
        tenant_cache::TenantProjectionCache,
        types::{Node, NodeId, event_types, node_entity_id},
    },
};

/// A stateless Prime engine backed by a remote Core, serving many tenants.
pub struct HostedPrime {
    base_url: String,
    api_key: Option<String>,
    cache: TenantProjectionCache,
}

impl HostedPrime {
    /// Connect to the Core at `base_url`, keeping up to `capacity` tenants warm,
    /// re-hydrating bundles older than `ttl`.
    pub fn connect(
        base_url: impl Into<String>,
        api_key: Option<String>,
        capacity: usize,
        ttl: Duration,
    ) -> Self {
        let base_url = base_url.into();
        let cache =
            TenantProjectionCache::new(base_url.clone(), api_key.clone(), capacity, ttl);
        Self {
            base_url,
            api_key,
            cache,
        }
    }

    /// An [`HttpCore`] scoped to `tenant`, for writes.
    fn core_for(&self, tenant: &str) -> HttpCore {
        HttpCore::new(self.base_url.clone(), self.api_key.clone(), Some(tenant.to_string()))
    }

    // ── Reads ────────────────────────────────────────────────────────────

    /// The tenant's warm graph-projection bundle (hydrated on demand).
    pub async fn tenant_graph(&self, tenant: &str) -> Result<Arc<GraphProjections>> {
        self.cache.get_or_hydrate(tenant).await
    }

    /// Get a node by entity_id within a tenant's graph. `None` if absent or
    /// soft-deleted.
    pub async fn get_node(&self, tenant: &str, entity_id: &str) -> Result<Option<Node>> {
        let graph = self.cache.get_or_hydrate(tenant).await?;
        Ok(graph
            .node_state
            .get_node(entity_id)
            .filter(|n| !n.deleted))
    }

    // ── Writes ───────────────────────────────────────────────────────────

    /// Create a node: ingest a `prime.node.created` event to the remote Core
    /// (tenant-stamped) and apply it to the tenant's warm bundle so subsequent
    /// reads reflect it without a re-query.
    pub async fn add_node(
        &self,
        tenant: &str,
        node_type: &str,
        properties: serde_json::Value,
    ) -> Result<NodeId> {
        let id = uuid::Uuid::new_v4().to_string();
        let entity_id = node_entity_id(node_type, &id);
        let payload = json!({
            "id": id,
            "node_type": node_type,
            "properties": properties,
        });

        self.core_for(tenant)
            .ingest(IngestEvent {
                entity_id: &entity_id,
                event_type: event_types::NODE_CREATED,
                payload: payload.clone(),
                metadata: None,
                tenant_id: Some(tenant),
            })
            .await?;

        // Ensure the tenant is warm, then apply the new event to the bundle.
        // (get_or_hydrate may already include this event if Core served it;
        // re-applying NODE_CREATED for the same entity is idempotent.)
        self.cache.get_or_hydrate(tenant).await?;
        self.cache.apply(tenant, &self.synth_view(tenant, &entity_id, payload));

        Ok(NodeId::new(id))
    }

    /// Synthesize a local [`EventView`] for cache application. The remote Core
    /// assigns the authoritative id/version/timestamp; the projections only key
    /// on entity_id/event_type/payload, so a local stand-in is sufficient until
    /// the next hydrate reconciles from Core.
    fn synth_view(&self, tenant: &str, entity_id: &str, payload: serde_json::Value) -> EventView {
        EventView {
            id: uuid::Uuid::new_v4(),
            event_type: event_types::NODE_CREATED.to_string(),
            entity_id: entity_id.to_string(),
            tenant_id: tenant.to_string(),
            payload,
            metadata: None,
            timestamp: chrono::Utc::now(),
            version: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn add_node_then_get_node_round_trips_through_warm_cache() {
        let server = MockServer::start().await;
        // Core has no prior events for this tenant…
        Mock::given(method("GET"))
            .and(path("/api/v1/events/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"events": []})))
            .mount(&server)
            .await;
        // …and accepts the ingest.
        Mock::given(method("POST"))
            .and(path("/api/v1/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&server)
            .await;

        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));
        let id = hosted
            .add_node("tenant-a", "contact", serde_json::json!({"name": "Alice"}))
            .await
            .unwrap();

        // The node is visible from the warm bundle even though Core's query
        // returned empty — proving the write updated the cache, not just Core.
        let entity_id = node_entity_id("contact", &id.0);
        let node = hosted.get_node("tenant-a", &entity_id).await.unwrap();
        assert!(node.is_some(), "node should be readable after add_node");
        assert_eq!(node.unwrap().properties["name"], "Alice");
    }

    #[tokio::test]
    async fn get_node_absent_returns_none() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/events/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"events": []})))
            .mount(&server)
            .await;

        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));
        let node = hosted
            .get_node("tenant-a", "node:contact:ghost")
            .await
            .unwrap();
        assert!(node.is_none());
    }
}
