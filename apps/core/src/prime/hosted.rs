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
    embedded::{EventView, IngestEvent, Query},
    error::Result,
    prime::{
        event_store::EventStore,
        http_core::HttpCore,
        projection_bundle::GraphProjections,
        tenant_cache::TenantProjectionCache,
        types::{
            Direction, EdgeId, Node, NodeId, PrimeStats, edge_entity_id, event_types,
            node_entity_id,
        },
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
        self.cache.apply(
            tenant,
            &self.synth_view_typed(tenant, event_types::NODE_CREATED, &entity_id, payload),
        );

        Ok(NodeId::new(id))
    }

    /// Create a directed edge: ingest a `prime.edge.created` event (tenant-stamped)
    /// and apply it to the tenant's warm bundle so `neighbors` reflects it without
    /// a re-query. Mirrors the embedded `Prime::add_edge_inner` payload exactly.
    pub async fn add_edge(
        &self,
        tenant: &str,
        source: &str,
        target: &str,
        relation: &str,
        properties: Option<serde_json::Value>,
    ) -> Result<EdgeId> {
        self.add_edge_inner(tenant, source, target, relation, None, properties)
            .await
    }

    /// Create a weighted directed edge. See [`add_edge`](Self::add_edge).
    pub async fn add_edge_weighted(
        &self,
        tenant: &str,
        source: &str,
        target: &str,
        relation: &str,
        weight: f64,
        properties: Option<serde_json::Value>,
    ) -> Result<EdgeId> {
        self.add_edge_inner(tenant, source, target, relation, Some(weight), properties)
            .await
    }

    async fn add_edge_inner(
        &self,
        tenant: &str,
        source: &str,
        target: &str,
        relation: &str,
        weight: Option<f64>,
        properties: Option<serde_json::Value>,
    ) -> Result<EdgeId> {
        let id = uuid::Uuid::new_v4().to_string();
        let entity_id = edge_entity_id(&id);

        let mut payload = json!({
            "id": id,
            "source": source,
            "target": target,
            "relation": relation,
        });
        if let Some(w) = weight {
            payload["weight"] = json!(w);
        }
        if let Some(props) = properties {
            payload["properties"] = props;
        }

        self.core_for(tenant)
            .ingest(IngestEvent {
                entity_id: &entity_id,
                event_type: event_types::EDGE_CREATED,
                payload: payload.clone(),
                metadata: None,
                tenant_id: Some(tenant),
            })
            .await?;

        self.cache.get_or_hydrate(tenant).await?;
        self.cache.apply(
            tenant,
            &self.synth_view_typed(tenant, event_types::EDGE_CREATED, &entity_id, payload),
        );

        Ok(EdgeId::new(id))
    }

    /// Soft-delete a node: ingest a `prime.node.deleted` event (tenant-stamped)
    /// and apply it to the tenant's warm bundle so `get_node` returns `None`
    /// without a re-query. Mirrors the embedded `Prime::delete_node` node-delete
    /// event shape (connected edges are reconciled on the next hydrate).
    pub async fn delete_node(&self, tenant: &str, entity_id: &str) -> Result<()> {
        self.core_for(tenant)
            .ingest(IngestEvent {
                entity_id,
                event_type: event_types::NODE_DELETED,
                payload: json!({}),
                metadata: None,
                tenant_id: Some(tenant),
            })
            .await?;

        self.cache.get_or_hydrate(tenant).await?;
        self.cache.apply(
            tenant,
            &self.synth_view_typed(tenant, event_types::NODE_DELETED, entity_id, json!({})),
        );

        Ok(())
    }

    /// Get 1-hop neighbors of a node, optionally filtered by relation and
    /// direction. Returns full [`Node`]s with deleted nodes excluded. Mirrors
    /// the embedded `Prime::neighbors` traversal over `adjacency`/`reverse_index`.
    pub async fn neighbors(
        &self,
        tenant: &str,
        entity_id: &str,
        relation: Option<&str>,
        direction: Direction,
    ) -> Result<Vec<Node>> {
        let g = self.cache.get_or_hydrate(tenant).await?;

        let mut peer_ids: Vec<String> = Vec::new();
        let matches = |entry_relation: &str| relation.is_none_or(|r| r == entry_relation);

        match direction {
            Direction::Outgoing => {
                for entry in g.adjacency.outgoing(entity_id) {
                    if matches(&entry.relation) {
                        peer_ids.push(entry.peer.clone());
                    }
                }
            }
            Direction::Incoming => {
                for entry in g.reverse_index.incoming(entity_id) {
                    if matches(&entry.relation) {
                        peer_ids.push(entry.peer.clone());
                    }
                }
            }
            Direction::Both => {
                let mut seen = std::collections::HashSet::new();
                for entry in g.adjacency.outgoing(entity_id) {
                    if matches(&entry.relation) && seen.insert(entry.peer.clone()) {
                        peer_ids.push(entry.peer.clone());
                    }
                }
                for entry in g.reverse_index.incoming(entity_id) {
                    if matches(&entry.relation) && seen.insert(entry.peer.clone()) {
                        peer_ids.push(entry.peer.clone());
                    }
                }
            }
        }

        Ok(peer_ids
            .iter()
            .filter_map(|id| g.node_state.get_node(id).filter(|n| !n.deleted))
            .collect())
    }

    /// All live nodes of a given type. Mirrors the embedded `Prime::nodes_by_type`
    /// (reads `node_type_index` + `node_state`).
    pub async fn search(&self, tenant: &str, node_type: &str) -> Result<Vec<Node>> {
        let g = self.cache.get_or_hydrate(tenant).await?;
        Ok(g.node_type_index
            .nodes_by_type(node_type)
            .iter()
            .filter_map(|entity_id| g.node_state.get_node(entity_id).filter(|n| !n.deleted))
            .collect())
    }

    /// Statistics about the tenant's graph (O(1) via the `graph_stats` projection).
    pub async fn stats(&self, tenant: &str) -> Result<PrimeStats> {
        let g = self.cache.get_or_hydrate(tenant).await?;
        Ok(g.graph_stats.stats())
    }

    /// The full audit trail for an entity, read straight from the remote Core.
    /// Returns raw events (no projection); empty if the entity never existed.
    pub async fn history(&self, tenant: &str, entity_id: &str) -> Result<Vec<EventView>> {
        self.core_for(tenant)
            .query(Query::new().entity_id(entity_id))
            .await
    }

    /// Synthesize a local [`EventView`] for cache application. The remote Core
    /// assigns the authoritative id/version/timestamp; the projections only key
    /// on entity_id/event_type/payload, so a local stand-in is sufficient until
    /// the next hydrate reconciles from Core.
    fn synth_view_typed(
        &self,
        tenant: &str,
        event_type: &str,
        entity_id: &str,
        payload: serde_json::Value,
    ) -> EventView {
        EventView {
            id: uuid::Uuid::new_v4(),
            event_type: event_type.to_string(),
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

    /// Mount the standard pair: GET query → empty, POST ingest → 200. Proving a
    /// read after a write succeeds therefore proves the warm cache was updated,
    /// not that Core re-served the event.
    async fn mount_empty_core(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/api/v1/events/query"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"events": []})),
            )
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn add_edge_then_neighbors_returns_target() {
        let server = MockServer::start().await;
        mount_empty_core(&server).await;

        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));
        let alice = hosted
            .add_node("tenant-a", "contact", serde_json::json!({"name": "Alice"}))
            .await
            .unwrap();
        let bob = hosted
            .add_node("tenant-a", "contact", serde_json::json!({"name": "Bob"}))
            .await
            .unwrap();
        let alice_eid = node_entity_id("contact", &alice.0);
        let bob_eid = node_entity_id("contact", &bob.0);

        hosted
            .add_edge("tenant-a", &alice_eid, &bob_eid, "knows", None)
            .await
            .unwrap();

        // Outgoing neighbors of Alice should be Bob, served entirely from the
        // warm cache (Core's query is mocked empty).
        let out = hosted
            .neighbors("tenant-a", &alice_eid, None, Direction::Outgoing)
            .await
            .unwrap();
        assert_eq!(out.len(), 1, "alice should have one outgoing neighbor");
        assert_eq!(out[0].properties["name"], "Bob");

        // And Bob's incoming neighbor is Alice.
        let inc = hosted
            .neighbors("tenant-a", &bob_eid, Some("knows"), Direction::Incoming)
            .await
            .unwrap();
        assert_eq!(inc.len(), 1);
        assert_eq!(inc[0].properties["name"], "Alice");
    }

    #[tokio::test]
    async fn add_nodes_then_search_returns_all_of_type() {
        let server = MockServer::start().await;
        mount_empty_core(&server).await;

        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));
        hosted
            .add_node("tenant-a", "contact", serde_json::json!({"name": "Alice"}))
            .await
            .unwrap();
        hosted
            .add_node("tenant-a", "contact", serde_json::json!({"name": "Bob"}))
            .await
            .unwrap();
        // A node of a different type must not show up in the search.
        hosted
            .add_node("tenant-a", "company", serde_json::json!({"name": "Acme"}))
            .await
            .unwrap();

        let contacts = hosted.search("tenant-a", "contact").await.unwrap();
        assert_eq!(contacts.len(), 2, "both contacts should be returned");
        let names: std::collections::HashSet<&str> = contacts
            .iter()
            .map(|n| n.properties["name"].as_str().unwrap())
            .collect();
        assert!(names.contains("Alice"));
        assert!(names.contains("Bob"));
    }

    #[tokio::test]
    async fn stats_reflects_added_nodes_and_edges() {
        let server = MockServer::start().await;
        mount_empty_core(&server).await;

        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));
        let a = hosted
            .add_node("tenant-a", "contact", serde_json::json!({"name": "Alice"}))
            .await
            .unwrap();
        let b = hosted
            .add_node("tenant-a", "contact", serde_json::json!({"name": "Bob"}))
            .await
            .unwrap();
        hosted
            .add_edge(
                "tenant-a",
                &node_entity_id("contact", &a.0),
                &node_entity_id("contact", &b.0),
                "knows",
                None,
            )
            .await
            .unwrap();

        let stats = hosted.stats("tenant-a").await.unwrap();
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.total_edges, 1);
    }

    #[tokio::test]
    async fn delete_node_then_get_node_returns_none() {
        let server = MockServer::start().await;
        mount_empty_core(&server).await;

        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));
        let id = hosted
            .add_node("tenant-a", "contact", serde_json::json!({"name": "Alice"}))
            .await
            .unwrap();
        let entity_id = node_entity_id("contact", &id.0);

        assert!(hosted.get_node("tenant-a", &entity_id).await.unwrap().is_some());

        hosted.delete_node("tenant-a", &entity_id).await.unwrap();

        // The delete folded into the warm bundle — get_node returns None even
        // though Core's query is mocked empty.
        let node = hosted.get_node("tenant-a", &entity_id).await.unwrap();
        assert!(node.is_none(), "node should be gone after delete_node");
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
