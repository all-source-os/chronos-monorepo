/// Geo-Replication Manager for cross-region event replication.
///
/// Coordinates replication between AllSource Core instances in different
/// geographic regions. Uses HLC for causal ordering and CRDTs for
/// conflict-free convergence.
///
/// # Architecture
///
/// ```text
/// Region US-East          Region EU-West          Region AP-East
/// ┌─────────────┐        ┌─────────────┐        ┌─────────────┐
/// │ Core Leader  │◄──────►│ Core Leader  │◄──────►│ Core Leader  │
/// │ (writes OK)  │  sync  │ (writes OK)  │  sync  │ (writes OK)  │
/// │ HLC + CRDT   │        │ HLC + CRDT   │        │ HLC + CRDT   │
/// └─────────────┘        └─────────────┘        └─────────────┘
/// ```
///
/// Each region is an independent leader that accepts writes locally.
/// Events are replicated asynchronously to peer regions using the
/// GeoReplicationManager. CRDT resolution ensures convergence.
///
/// # Opt-in
///
/// Enabled via `ALLSOURCE_GEO_REPLICATION_ENABLED=true` with:
/// - `ALLSOURCE_REGION_ID`: this region's identifier (e.g., "us-east-1")
/// - `ALLSOURCE_GEO_PEERS`: comma-separated peer URLs (e.g., "https://eu.core:3900,https://ap.core:3900")
use super::crdt::{ConflictResolution, CrdtResolver, ReplicatedEvent, VersionVector};
use super::hlc::{HlcTimestamp, HybridLogicalClock};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

/// Configuration for geo-replication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoReplicationConfig {
    /// This region's unique identifier.
    pub region_id: String,
    /// Peer region endpoints for replication.
    pub peers: Vec<PeerRegion>,
    /// Sync interval for pushing events to peers (ms).
    pub sync_interval_ms: u64,
    /// Maximum HLC clock drift tolerance (ms).
    pub max_clock_drift_ms: u64,
    /// Batch size for replication sync.
    pub batch_size: usize,
}

impl Default for GeoReplicationConfig {
    fn default() -> Self {
        Self {
            region_id: "default".to_string(),
            peers: vec![],
            sync_interval_ms: 1000,
            max_clock_drift_ms: 60_000,
            batch_size: 100,
        }
    }
}

impl GeoReplicationConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Option<Self> {
        let enabled = std::env::var("ALLSOURCE_GEO_REPLICATION_ENABLED")
            .map(|v| v == "true")
            .unwrap_or(false);

        if !enabled {
            return None;
        }

        let region_id =
            std::env::var("ALLSOURCE_REGION_ID").unwrap_or_else(|_| "default".to_string());

        let peers_str = std::env::var("ALLSOURCE_GEO_PEERS").unwrap_or_default();
        let peers: Vec<PeerRegion> = peers_str
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .enumerate()
            .map(|(i, url)| PeerRegion {
                region_id: format!("peer-{}", i),
                api_url: url.trim().to_string(),
                healthy: true,
                last_sync_ms: 0,
            })
            .collect();

        let sync_interval_ms: u64 = std::env::var("ALLSOURCE_GEO_SYNC_INTERVAL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000);

        let max_clock_drift_ms: u64 = std::env::var("ALLSOURCE_GEO_MAX_DRIFT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60_000);

        let batch_size: usize = std::env::var("ALLSOURCE_GEO_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        Some(Self {
            region_id,
            peers,
            sync_interval_ms,
            max_clock_drift_ms,
            batch_size,
        })
    }
}

/// A peer region endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRegion {
    /// Region identifier.
    pub region_id: String,
    /// HTTP API URL for the peer's Core instance.
    pub api_url: String,
    /// Whether the peer is currently healthy.
    pub healthy: bool,
    /// Last successful sync timestamp (ms since epoch).
    pub last_sync_ms: u64,
}

/// Health status of a peer region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerHealth {
    Healthy,
    Degraded,
    Unreachable,
}

/// Status of the geo-replication system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoReplicationStatus {
    /// This region's ID.
    pub region_id: String,
    /// Peer regions and their health.
    pub peers: Vec<PeerStatus>,
    /// Total events replicated out.
    pub events_sent: u64,
    /// Total events received from peers.
    pub events_received: u64,
    /// Total conflicts resolved (skipped duplicates).
    pub conflicts_resolved: u64,
    /// Current HLC state.
    pub current_hlc: HlcTimestamp,
    /// Version vectors per region.
    pub version_vectors: std::collections::BTreeMap<String, VersionVector>,
}

/// Status of a single peer region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStatus {
    pub region_id: String,
    pub api_url: String,
    pub health: PeerHealth,
    pub last_sync_ms: u64,
    pub replication_lag_ms: u64,
}

/// Replication sync request sent to a peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoSyncRequest {
    /// Source region ID.
    pub source_region: String,
    /// Events to replicate.
    pub events: Vec<ReplicatedEvent>,
    /// Source's version vector for this peer.
    pub version_vector: VersionVector,
}

/// Replication sync response from a peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoSyncResponse {
    /// Number of events accepted.
    pub accepted: usize,
    /// Number of events skipped (duplicates).
    pub skipped: usize,
    /// Peer's current version vector (for bi-directional sync).
    pub version_vector: VersionVector,
}

/// Geo-Replication Manager.
///
/// Manages cross-region event replication with CRDT conflict resolution
/// and HLC-based causal ordering.
pub struct GeoReplicationManager {
    /// Configuration.
    config: GeoReplicationConfig,
    /// Hybrid Logical Clock for this region.
    hlc: Arc<HybridLogicalClock>,
    /// CRDT resolver for conflict resolution.
    resolver: Arc<CrdtResolver>,
    /// Peer health tracking.
    peer_health: DashMap<String, PeerHealth>,
    /// Outbound event buffer (events pending replication to peers).
    outbound_buffer: DashMap<String, Vec<ReplicatedEvent>>,
    /// Counter: events sent.
    events_sent: std::sync::atomic::AtomicU64,
    /// Counter: events received.
    events_received: std::sync::atomic::AtomicU64,
    /// Counter: conflicts resolved.
    conflicts_resolved: std::sync::atomic::AtomicU64,
}

impl GeoReplicationManager {
    /// Create a new geo-replication manager.
    pub fn new(config: GeoReplicationConfig) -> Self {
        let node_id: u32 = std::env::var("ALLSOURCE_NODE_ID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let hlc = Arc::new(HybridLogicalClock::with_max_drift(
            node_id,
            config.max_clock_drift_ms,
        ));

        let peer_health = DashMap::new();
        let outbound_buffer = DashMap::new();
        for peer in &config.peers {
            peer_health.insert(peer.region_id.clone(), PeerHealth::Healthy);
            outbound_buffer.insert(peer.region_id.clone(), Vec::new());
        }

        Self {
            config,
            hlc,
            resolver: Arc::new(CrdtResolver::new()),
            peer_health,
            outbound_buffer,
            events_sent: std::sync::atomic::AtomicU64::new(0),
            events_received: std::sync::atomic::AtomicU64::new(0),
            conflicts_resolved: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Get this region's ID.
    pub fn region_id(&self) -> &str {
        &self.config.region_id
    }

    /// Get the HLC instance.
    pub fn hlc(&self) -> &Arc<HybridLogicalClock> {
        &self.hlc
    }

    /// Get the CRDT resolver.
    pub fn resolver(&self) -> &Arc<CrdtResolver> {
        &self.resolver
    }

    /// Stamp a new local event with an HLC timestamp.
    pub fn stamp_event(&self, event_id: &str, event_data: serde_json::Value) -> ReplicatedEvent {
        let ts = self.hlc.now();
        ReplicatedEvent {
            event_id: event_id.to_string(),
            hlc_timestamp: ts,
            origin_region: self.config.region_id.clone(),
            event_data,
        }
    }

    /// Queue an event for replication to all peers.
    pub fn queue_for_replication(&self, event: ReplicatedEvent) {
        for mut buffer in self.outbound_buffer.iter_mut() {
            buffer.value_mut().push(event.clone());
        }
    }

    /// Drain the outbound buffer for a specific peer.
    pub fn drain_outbound(&self, peer_region: &str, max_batch: usize) -> Vec<ReplicatedEvent> {
        if let Some(mut buffer) = self.outbound_buffer.get_mut(peer_region) {
            let drain_count = max_batch.min(buffer.len());
            buffer.drain(..drain_count).collect()
        } else {
            vec![]
        }
    }

    /// Receive events from a peer region.
    ///
    /// Applies CRDT conflict resolution and returns the sync response.
    pub fn receive_sync(&self, request: &GeoSyncRequest) -> GeoSyncResponse {
        let mut accepted = 0;
        let mut skipped = 0;

        for event in &request.events {
            // Update HLC from remote timestamp
            if let Err(e) = self.hlc.receive(&event.hlc_timestamp) {
                tracing::warn!(
                    "HLC drift violation from region {}: {}",
                    request.source_region,
                    e,
                );
                skipped += 1;
                self.conflicts_resolved
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                continue;
            }

            match self.resolver.resolve_and_accept(event) {
                ConflictResolution::Accept => {
                    accepted += 1;
                    self.events_received
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                ConflictResolution::Skip => {
                    skipped += 1;
                    self.conflicts_resolved
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }

        // Merge remote version vector
        self.resolver
            .merge_version_vector(&request.source_region, &request.version_vector);

        // Build our version vector for the response
        let our_vv = self
            .resolver
            .version_vector_for(&self.config.region_id)
            .unwrap_or_default();

        GeoSyncResponse {
            accepted,
            skipped,
            version_vector: our_vv,
        }
    }

    /// Update a peer's health status.
    pub fn set_peer_health(&self, peer_region: &str, health: PeerHealth) {
        self.peer_health.insert(peer_region.to_string(), health);
    }

    /// Get a peer's health status.
    pub fn peer_health(&self, peer_region: &str) -> PeerHealth {
        self.peer_health
            .get(peer_region)
            .map(|h| *h)
            .unwrap_or(PeerHealth::Unreachable)
    }

    /// Get all peers.
    pub fn peers(&self) -> &[PeerRegion] {
        &self.config.peers
    }

    /// Get sync interval.
    pub fn sync_interval(&self) -> Duration {
        Duration::from_millis(self.config.sync_interval_ms)
    }

    /// Get batch size.
    pub fn batch_size(&self) -> usize {
        self.config.batch_size
    }

    /// Build a sync request for a specific peer.
    pub fn build_sync_request(&self, peer_region: &str) -> Option<GeoSyncRequest> {
        let events = self.drain_outbound(peer_region, self.config.batch_size);
        if events.is_empty() {
            return None;
        }

        self.events_sent
            .fetch_add(events.len() as u64, std::sync::atomic::Ordering::Relaxed);

        let vv = self
            .resolver
            .version_vector_for(&self.config.region_id)
            .unwrap_or_default();

        Some(GeoSyncRequest {
            source_region: self.config.region_id.clone(),
            events,
            version_vector: vv,
        })
    }

    /// Select the best failover region (lowest replication lag, healthy).
    pub fn select_failover_region(&self) -> Option<String> {
        self.config
            .peers
            .iter()
            .filter(|p| {
                self.peer_health
                    .get(&p.region_id)
                    .map(|h| *h == PeerHealth::Healthy)
                    .unwrap_or(false)
            })
            .max_by_key(|p| p.last_sync_ms) // most recently synced = least data loss
            .map(|p| p.region_id.clone())
    }

    /// Get full replication status.
    pub fn status(&self) -> GeoReplicationStatus {
        let peers: Vec<PeerStatus> = self
            .config
            .peers
            .iter()
            .map(|p| {
                let health = self
                    .peer_health
                    .get(&p.region_id)
                    .map(|h| *h)
                    .unwrap_or(PeerHealth::Unreachable);

                let lag_ms = if p.last_sync_ms > 0 {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    now_ms.saturating_sub(p.last_sync_ms)
                } else {
                    0
                };

                PeerStatus {
                    region_id: p.region_id.clone(),
                    api_url: p.api_url.clone(),
                    health,
                    last_sync_ms: p.last_sync_ms,
                    replication_lag_ms: lag_ms,
                }
            })
            .collect();

        GeoReplicationStatus {
            region_id: self.config.region_id.clone(),
            peers,
            events_sent: self.events_sent.load(std::sync::atomic::Ordering::Relaxed),
            events_received: self
                .events_received
                .load(std::sync::atomic::Ordering::Relaxed),
            conflicts_resolved: self
                .conflicts_resolved
                .load(std::sync::atomic::Ordering::Relaxed),
            current_hlc: self.hlc.current(),
            version_vectors: self.resolver.all_version_vectors(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(region: &str) -> GeoReplicationConfig {
        GeoReplicationConfig {
            region_id: region.to_string(),
            peers: vec![PeerRegion {
                region_id: "eu-west".to_string(),
                api_url: "https://eu.core:3900".to_string(),
                healthy: true,
                last_sync_ms: 0,
            }],
            sync_interval_ms: 1000,
            max_clock_drift_ms: 60_000,
            batch_size: 100,
        }
    }

    #[test]
    fn test_stamp_event() {
        let mgr = GeoReplicationManager::new(test_config("us-east"));
        let event = mgr.stamp_event("evt-1", serde_json::json!({"foo": "bar"}));

        assert_eq!(event.event_id, "evt-1");
        assert_eq!(event.origin_region, "us-east");
        assert!(event.hlc_timestamp.physical_ms > 0);
    }

    #[test]
    fn test_queue_and_drain() {
        let mgr = GeoReplicationManager::new(test_config("us-east"));
        let event = mgr.stamp_event("evt-1", serde_json::json!({}));

        mgr.queue_for_replication(event);

        let batch = mgr.drain_outbound("eu-west", 10);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].event_id, "evt-1");

        // Drained — should be empty now
        let batch2 = mgr.drain_outbound("eu-west", 10);
        assert!(batch2.is_empty());
    }

    #[test]
    fn test_receive_sync_accepts_new_events() {
        let mgr = GeoReplicationManager::new(test_config("us-east"));

        let request = GeoSyncRequest {
            source_region: "eu-west".to_string(),
            events: vec![ReplicatedEvent {
                event_id: "evt-remote-1".to_string(),
                hlc_timestamp: HlcTimestamp::new(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    0,
                    2,
                ),
                origin_region: "eu-west".to_string(),
                event_data: serde_json::json!({"source": "eu"}),
            }],
            version_vector: VersionVector::new(),
        };

        let response = mgr.receive_sync(&request);
        assert_eq!(response.accepted, 1);
        assert_eq!(response.skipped, 0);
    }

    #[test]
    fn test_receive_sync_skips_duplicates() {
        let mgr = GeoReplicationManager::new(test_config("us-east"));
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let event = ReplicatedEvent {
            event_id: "evt-dup".to_string(),
            hlc_timestamp: HlcTimestamp::new(now_ms, 0, 2),
            origin_region: "eu-west".to_string(),
            event_data: serde_json::json!({}),
        };

        let request = GeoSyncRequest {
            source_region: "eu-west".to_string(),
            events: vec![event.clone(), event],
            version_vector: VersionVector::new(),
        };

        let response = mgr.receive_sync(&request);
        assert_eq!(response.accepted, 1);
        assert_eq!(response.skipped, 1);
    }

    #[test]
    fn test_build_sync_request() {
        let mgr = GeoReplicationManager::new(test_config("us-east"));
        let event = mgr.stamp_event("evt-1", serde_json::json!({}));
        mgr.queue_for_replication(event);

        let req = mgr.build_sync_request("eu-west");
        assert!(req.is_some());
        let req = req.unwrap();
        assert_eq!(req.source_region, "us-east");
        assert_eq!(req.events.len(), 1);
    }

    #[test]
    fn test_build_sync_request_empty() {
        let mgr = GeoReplicationManager::new(test_config("us-east"));
        let req = mgr.build_sync_request("eu-west");
        assert!(req.is_none());
    }

    #[test]
    fn test_peer_health_tracking() {
        let mgr = GeoReplicationManager::new(test_config("us-east"));
        assert_eq!(mgr.peer_health("eu-west"), PeerHealth::Healthy);

        mgr.set_peer_health("eu-west", PeerHealth::Degraded);
        assert_eq!(mgr.peer_health("eu-west"), PeerHealth::Degraded);

        mgr.set_peer_health("eu-west", PeerHealth::Unreachable);
        assert_eq!(mgr.peer_health("eu-west"), PeerHealth::Unreachable);
    }

    #[test]
    fn test_select_failover_region() {
        let config = GeoReplicationConfig {
            region_id: "us-east".to_string(),
            peers: vec![
                PeerRegion {
                    region_id: "eu-west".to_string(),
                    api_url: "https://eu.core:3900".to_string(),
                    healthy: true,
                    last_sync_ms: 100,
                },
                PeerRegion {
                    region_id: "ap-east".to_string(),
                    api_url: "https://ap.core:3900".to_string(),
                    healthy: true,
                    last_sync_ms: 200,
                },
            ],
            ..Default::default()
        };
        let mgr = GeoReplicationManager::new(config);

        // ap-east synced more recently → preferred failover
        let failover = mgr.select_failover_region();
        assert_eq!(failover, Some("ap-east".to_string()));
    }

    #[test]
    fn test_select_failover_skips_unhealthy() {
        let config = GeoReplicationConfig {
            region_id: "us-east".to_string(),
            peers: vec![
                PeerRegion {
                    region_id: "eu-west".to_string(),
                    api_url: "https://eu.core:3900".to_string(),
                    healthy: true,
                    last_sync_ms: 200,
                },
                PeerRegion {
                    region_id: "ap-east".to_string(),
                    api_url: "https://ap.core:3900".to_string(),
                    healthy: true,
                    last_sync_ms: 300,
                },
            ],
            ..Default::default()
        };
        let mgr = GeoReplicationManager::new(config);
        mgr.set_peer_health("ap-east", PeerHealth::Unreachable);

        let failover = mgr.select_failover_region();
        assert_eq!(failover, Some("eu-west".to_string()));
    }

    #[test]
    fn test_status() {
        let mgr = GeoReplicationManager::new(test_config("us-east"));
        let status = mgr.status();

        assert_eq!(status.region_id, "us-east");
        assert_eq!(status.peers.len(), 1);
        assert_eq!(status.events_sent, 0);
        assert_eq!(status.events_received, 0);
        assert_eq!(status.conflicts_resolved, 0);
    }

    #[test]
    fn test_two_region_convergence() {
        // Simulate two regions exchanging events
        let us = GeoReplicationManager::new(GeoReplicationConfig {
            region_id: "us-east".to_string(),
            peers: vec![PeerRegion {
                region_id: "eu-west".to_string(),
                api_url: "http://eu:3900".to_string(),
                healthy: true,
                last_sync_ms: 0,
            }],
            ..Default::default()
        });
        let eu = GeoReplicationManager::new(GeoReplicationConfig {
            region_id: "eu-west".to_string(),
            peers: vec![PeerRegion {
                region_id: "us-east".to_string(),
                api_url: "http://us:3900".to_string(),
                healthy: true,
                last_sync_ms: 0,
            }],
            ..Default::default()
        });

        // US writes event 1
        let evt1 = us.stamp_event("evt-1", serde_json::json!({"from": "us"}));
        us.resolver.resolve_and_accept(&evt1);
        us.queue_for_replication(evt1);

        // EU writes event 2
        let evt2 = eu.stamp_event("evt-2", serde_json::json!({"from": "eu"}));
        eu.resolver.resolve_and_accept(&evt2);
        eu.queue_for_replication(evt2);

        // US → EU sync
        let us_req = us.build_sync_request("eu-west").unwrap();
        let eu_resp = eu.receive_sync(&us_req);
        assert_eq!(eu_resp.accepted, 1);

        // EU → US sync
        let eu_req = eu.build_sync_request("us-east").unwrap();
        let us_resp = us.receive_sync(&eu_req);
        assert_eq!(us_resp.accepted, 1);

        // Both regions now have both events
        assert_eq!(us.resolver.seen_count(), 2);
        assert_eq!(eu.resolver.seen_count(), 2);
    }
}
