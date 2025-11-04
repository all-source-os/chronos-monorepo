/// Cluster Management for Distributed Partitioning
///
/// Enables horizontal scaling with SierraDB's fixed partition architecture.
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
/// # Example
///
/// ```ignore
/// use allsource_core::infrastructure::cluster::{NodeRegistry, RequestRouter, Node};
/// use std::sync::Arc;
///
/// // Create cluster with 32 partitions
/// let registry = Arc::new(NodeRegistry::new(32));
///
/// // Register nodes
/// for i in 0..4 {
///     registry.register_node(Node {
///         id: i,
///         address: format!("node-{}:8080", i),
///         healthy: true,
///         assigned_partitions: vec![],
///     });
/// }
///
/// // Create router
/// let router = RequestRouter::new(registry.clone());
///
/// // Route requests
/// let entity_id = EntityId::new("user-123".to_string())?;
/// let target_node = router.route_for_entity(&entity_id)?;
///
/// println!("Send request to: {}", target_node.address);
/// ```

pub mod node_registry;
pub mod request_router;

pub use node_registry::{Node, NodeRegistry};
pub use request_router::RequestRouter;
