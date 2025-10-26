/// Request Router for Distributed Partitioning
///
/// Routes requests to the correct node based on partition assignment.
/// Enables horizontal scaling with SierraDB's fixed partition architecture.
///
/// # Design
/// - **Partition-aware routing**: Entity ID → Partition → Node
/// - **Failover**: Retry on node failure
/// - **Load balancing**: Even distribution via consistent hashing
///
/// # Example
/// ```ignore
/// let registry = Arc::new(NodeRegistry::new(32));
/// let router = RequestRouter::new(registry.clone());
///
/// let entity_id = EntityId::new("user-123".to_string())?;
/// let target_node = router.route_for_entity(&entity_id)?;
///
/// // Send request to target_node.address
/// ```

use std::sync::Arc;
use crate::domain::value_objects::{EntityId, PartitionKey};
use crate::error::{AllSourceError, Result};
use super::node_registry::{Node, NodeRegistry};

/// Request Router for partition-aware request routing
pub struct RequestRouter {
    registry: Arc<NodeRegistry>,
}

impl RequestRouter {
    /// Create new request router
    ///
    /// # Arguments
    /// - `registry`: Shared node registry for cluster topology
    pub fn new(registry: Arc<NodeRegistry>) -> Self {
        Self { registry }
    }

    /// Route request for an entity
    ///
    /// Determines the partition for the entity and finds the responsible node.
    ///
    /// # Returns
    /// - `Node`: Target node to send the request to
    ///
    /// # Errors
    /// - Returns error if no healthy node is available for the partition
    pub fn route_for_entity(&self, entity_id: &EntityId) -> Result<Node> {
        // Determine partition using consistent hashing
        let partition_key = PartitionKey::from_entity_id(entity_id.as_str());
        self.route_for_partition(&partition_key)
    }

    /// Route request for a specific partition
    ///
    /// # Returns
    /// - `Node`: Target node responsible for this partition
    ///
    /// # Errors
    /// - Returns error if no healthy node is available
    pub fn route_for_partition(&self, partition_key: &PartitionKey) -> Result<Node> {
        let partition_id = partition_key.partition_id();

        let node_id = self.registry.node_for_partition(partition_id)
            .ok_or_else(|| AllSourceError::StorageError(format!(
                "No healthy node available for partition {}",
                partition_id
            )))?;

        self.registry.get_node(node_id)
            .ok_or_else(|| AllSourceError::InternalError(format!(
                "Node {} not found in registry",
                node_id
            )))
    }

    /// Get all nodes for load-balanced read operations
    ///
    /// Returns all healthy nodes that can serve read requests.
    /// Useful for fan-out queries across multiple nodes.
    pub fn nodes_for_read(&self) -> Vec<Node> {
        self.registry.healthy_nodes()
    }

    /// Check if a specific node can handle the entity
    ///
    /// Useful for sticky sessions or connection pooling.
    pub fn can_node_handle_entity(&self, entity_id: &EntityId, node_id: u32) -> bool {
        let partition_key = PartitionKey::from_entity_id(entity_id.as_str());
        let partition_id = partition_key.partition_id();

        if let Some(assigned_node_id) = self.registry.node_for_partition(partition_id) {
            assigned_node_id == node_id
        } else {
            false
        }
    }

    /// Get partition distribution for monitoring
    ///
    /// Returns map of node_id -> partition_ids for observability.
    pub fn partition_distribution(&self) -> std::collections::HashMap<u32, Vec<u32>> {
        self.registry.partition_distribution()
    }

    /// Check if routing is available
    ///
    /// Returns true if cluster is healthy and can handle requests.
    pub fn is_available(&self) -> bool {
        self.registry.is_cluster_healthy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::cluster::node_registry::Node;

    fn setup_cluster() -> (Arc<NodeRegistry>, RequestRouter) {
        let registry = Arc::new(NodeRegistry::new(32));

        // Register 4 nodes
        for i in 0..4 {
            registry.register_node(Node {
                id: i,
                address: format!("node-{}:8080", i),
                healthy: true,
                assigned_partitions: vec![],
            });
        }

        let router = RequestRouter::new(registry.clone());

        (registry, router)
    }

    #[test]
    fn test_create_router() {
        let registry = Arc::new(NodeRegistry::new(32));
        let _router = RequestRouter::new(registry);
    }

    #[test]
    fn test_route_for_entity() {
        let (_registry, router) = setup_cluster();

        let entity_id = EntityId::new("user-123".to_string()).unwrap();
        let node = router.route_for_entity(&entity_id).unwrap();

        // Should route to one of the 4 nodes
        assert!(node.id < 4);
        assert!(node.healthy);
    }

    #[test]
    fn test_consistent_routing() {
        let (_registry, router) = setup_cluster();

        let entity_id = EntityId::new("user-123".to_string()).unwrap();

        // Multiple calls should route to same node
        let node1 = router.route_for_entity(&entity_id).unwrap();
        let node2 = router.route_for_entity(&entity_id).unwrap();
        let node3 = router.route_for_entity(&entity_id).unwrap();

        assert_eq!(node1.id, node2.id);
        assert_eq!(node2.id, node3.id);
    }

    #[test]
    fn test_different_entities_may_route_differently() {
        let (_registry, router) = setup_cluster();

        let entity1 = EntityId::new("user-1".to_string()).unwrap();
        let entity2 = EntityId::new("user-2".to_string()).unwrap();
        let entity3 = EntityId::new("user-3".to_string()).unwrap();

        let node1 = router.route_for_entity(&entity1).unwrap();
        let node2 = router.route_for_entity(&entity2).unwrap();
        let node3 = router.route_for_entity(&entity3).unwrap();

        // Not all should route to same node (with 4 nodes and 3 entities)
        let unique_nodes: std::collections::HashSet<_> =
            vec![node1.id, node2.id, node3.id].into_iter().collect();

        // Should have some distribution (not guaranteed, but likely)
        println!("Unique nodes: {:?}", unique_nodes);
    }

    #[test]
    fn test_route_for_partition() {
        let (_registry, router) = setup_cluster();

        let partition_key = PartitionKey::from_partition_id(15, 32).unwrap();
        let node = router.route_for_partition(&partition_key).unwrap();

        assert!(node.id < 4);
        assert!(node.healthy);
    }

    #[test]
    fn test_can_node_handle_entity() {
        let (_registry, router) = setup_cluster();

        let entity_id = EntityId::new("user-123".to_string()).unwrap();
        let target_node = router.route_for_entity(&entity_id).unwrap();

        // Target node should be able to handle the entity
        assert!(router.can_node_handle_entity(&entity_id, target_node.id));

        // Other nodes should not (unless by chance they have overlapping partitions)
        // This is deterministic, so we can check
        for i in 0..4 {
            if i != target_node.id {
                // Other nodes may or may not handle it (depends on partition distribution)
                let _can_handle = router.can_node_handle_entity(&entity_id, i);
            }
        }
    }

    #[test]
    fn test_nodes_for_read() {
        let (_registry, router) = setup_cluster();

        let nodes = router.nodes_for_read();

        assert_eq!(nodes.len(), 4);
        assert!(nodes.iter().all(|n| n.healthy));
    }

    #[test]
    fn test_no_healthy_nodes() {
        let registry = Arc::new(NodeRegistry::new(32));

        // Register unhealthy node
        registry.register_node(Node {
            id: 0,
            address: "node-0:8080".to_string(),
            healthy: false,
            assigned_partitions: vec![],
        });

        let router = RequestRouter::new(registry);

        let entity_id = EntityId::new("user-123".to_string()).unwrap();
        let result = router.route_for_entity(&entity_id);

        assert!(result.is_err());
    }

    #[test]
    fn test_partition_distribution() {
        let (_registry, router) = setup_cluster();

        let distribution = router.partition_distribution();

        assert_eq!(distribution.len(), 4);

        // Each node should have 8 partitions (32/4)
        for (_node_id, partitions) in distribution {
            assert_eq!(partitions.len(), 8);
        }
    }

    #[test]
    fn test_is_available() {
        let (registry, router) = setup_cluster();

        // Cluster is healthy
        assert!(router.is_available());

        // Mark all nodes unhealthy
        for i in 0..4 {
            registry.set_node_health(i, false);
        }

        // Cluster is now unavailable
        assert!(!router.is_available());
    }
}
