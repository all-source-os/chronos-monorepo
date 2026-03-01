/// Cluster Management for Multi-Node Scaling
///
/// Provides partition-based routing, term-based consensus, and membership management.
///
/// # Components
///
/// ## NodeRegistry
/// - Manages cluster nodes and health status
/// - Automatic partition rebalancing
/// - Deterministic partition assignment
///
/// ## RequestRouter
/// - Routes requests to correct node based on entity/partition
/// - Failover on node failures
/// - Load balancing for read operations
///
/// ## ClusterManager (consensus)
/// - Term-based leader election (simplified Raft)
/// - Deterministic leader selection (highest WAL offset, lowest ID tiebreak)
/// - Cluster membership management
/// - Integrates with NodeRegistry for partition replication
///
/// # Example
///
/// ```ignore
/// use allsource_core::infrastructure::cluster::{ClusterManager, ClusterMember, MemberRole};
///
/// // Create cluster manager (node 0, 32 partitions)
/// let cluster = ClusterManager::new(0, 32);
///
/// // Add members
/// cluster.add_member(ClusterMember {
///     node_id: 0,
///     api_address: "node-0:3900".to_string(),
///     replication_address: "node-0:3910".to_string(),
///     role: MemberRole::Leader,
///     last_wal_offset: 0,
///     last_heartbeat_ms: 0,
///     healthy: true,
/// }).await;
///
/// // Get cluster status
/// let status = cluster.status().await;
/// ```
pub mod consensus;
pub mod crdt;
pub mod geo_replication;
pub mod hlc;
pub mod node_registry;
pub mod request_router;

pub use consensus::{
    ClusterManager, ClusterMember, ClusterStatus, MemberRole, VoteRequest, VoteResponse,
};
pub use crdt::{ConflictResolution, CrdtResolver, MergeStrategy, ReplicatedEvent, VersionVector};
pub use geo_replication::{
    GeoReplicationConfig, GeoReplicationManager, GeoReplicationStatus, GeoSyncRequest,
    GeoSyncResponse, PeerHealth, PeerRegion, PeerStatus,
};
pub use hlc::{HlcTimestamp, HybridLogicalClock};
pub use node_registry::{Node, NodeRegistry};
pub use request_router::RequestRouter;
