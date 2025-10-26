/// Node Registry for Distributed Partitioning
///
/// Manages cluster nodes and partition assignments for horizontal scaling.
/// Based on SierraDB's fixed partition architecture.
///
/// # Design
/// - **Fixed partitions**: 32 partitions (single-node) or 1024+ (cluster)
/// - **Consistent assignment**: Partitions assigned to nodes deterministically
/// - **Health monitoring**: Track node health status
/// - **Automatic rebalancing**: Reassign partitions on node failures
///
/// # Cluster Topology
/// - Single-node: All 32 partitions on one node
/// - 2-node: 16 partitions per node
/// - 4-node: 8 partitions per node
/// - 8-node: 4 partitions per node
///
/// # Example
/// ```ignore
/// let registry = NodeRegistry::new(32);
///
/// // Register nodes
/// registry.register_node(Node {
///     id: 0,
///     address: "node-0:8080".to_string(),
///     healthy: true,
///     assigned_partitions: vec![],
/// });
///
/// // Find node for partition
/// let node_id = registry.node_for_partition(15);
/// ```

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use crate::error::Result;

/// Node in the cluster
#[derive(Debug, Clone)]
pub struct Node {
    /// Unique node ID
    pub id: u32,

    /// Network address (host:port)
    pub address: String,

    /// Health status
    pub healthy: bool,

    /// Partitions assigned to this node
    pub assigned_partitions: Vec<u32>,
}

/// Node Registry manages cluster topology
pub struct NodeRegistry {
    /// Total number of partitions (fixed)
    partition_count: u32,

    /// Registered nodes
    nodes: Arc<RwLock<HashMap<u32, Node>>>,
}

impl NodeRegistry {
    /// Create new node registry
    ///
    /// # Arguments
    /// - `partition_count`: Total fixed partitions (32 for single-node, 1024+ for cluster)
    pub fn new(partition_count: u32) -> Self {
        Self {
            partition_count,
            nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a node in the cluster
    ///
    /// Automatically rebalances partitions across healthy nodes.
    pub fn register_node(&self, mut node: Node) {
        let mut nodes = self.nodes.write();

        // Clear assigned partitions (will be recalculated)
        node.assigned_partitions.clear();

        nodes.insert(node.id, node);

        // Rebalance partitions
        self.rebalance_partitions_locked(&mut nodes);
    }

    /// Unregister a node from the cluster
    ///
    /// Triggers automatic rebalancing to remaining nodes.
    pub fn unregister_node(&self, node_id: u32) {
        let mut nodes = self.nodes.write();
        nodes.remove(&node_id);
        self.rebalance_partitions_locked(&mut nodes);
    }

    /// Mark node as healthy or unhealthy
    ///
    /// Unhealthy nodes are excluded from partition assignment.
    pub fn set_node_health(&self, node_id: u32, healthy: bool) {
        let mut nodes = self.nodes.write();

        if let Some(node) = nodes.get_mut(&node_id) {
            node.healthy = healthy;
            self.rebalance_partitions_locked(&mut nodes);
        }
    }

    /// Rebalance partitions across healthy nodes
    ///
    /// Uses round-robin distribution for even load balancing.
    fn rebalance_partitions_locked(&self, nodes: &mut HashMap<u32, Node>) {
        // Clear existing assignments
        for node in nodes.values_mut() {
            node.assigned_partitions.clear();
        }

        // Get healthy nodes sorted by ID for deterministic assignment
        let mut healthy_nodes: Vec<u32> = nodes.iter()
            .filter(|(_, n)| n.healthy)
            .map(|(id, _)| *id)
            .collect();

        healthy_nodes.sort();

        if healthy_nodes.is_empty() {
            return; // No healthy nodes available
        }

        // Distribute partitions evenly using round-robin
        for partition_id in 0..self.partition_count {
            let node_idx = (partition_id as usize) % healthy_nodes.len();
            let node_id = healthy_nodes[node_idx];

            if let Some(node) = nodes.get_mut(&node_id) {
                node.assigned_partitions.push(partition_id);
            }
        }
    }

    /// Find node responsible for a partition
    ///
    /// Returns None if no healthy node is assigned to the partition.
    pub fn node_for_partition(&self, partition_id: u32) -> Option<u32> {
        let nodes = self.nodes.read();

        nodes.values()
            .find(|n| n.healthy && n.assigned_partitions.contains(&partition_id))
            .map(|n| n.id)
    }

    /// Get node by ID
    pub fn get_node(&self, node_id: u32) -> Option<Node> {
        self.nodes.read().get(&node_id).cloned()
    }

    /// Get all nodes
    pub fn all_nodes(&self) -> Vec<Node> {
        self.nodes.read().values().cloned().collect()
    }

    /// Get healthy nodes
    pub fn healthy_nodes(&self) -> Vec<Node> {
        self.nodes.read()
            .values()
            .filter(|n| n.healthy)
            .cloned()
            .collect()
    }

    /// Get partition distribution statistics
    pub fn partition_distribution(&self) -> HashMap<u32, Vec<u32>> {
        let nodes = self.nodes.read();

        nodes.iter()
            .filter(|(_, n)| n.healthy)
            .map(|(id, n)| (*id, n.assigned_partitions.clone()))
            .collect()
    }

    /// Check if cluster is healthy
    ///
    /// Returns true if all partitions have at least one healthy node assigned.
    pub fn is_cluster_healthy(&self) -> bool {
        let nodes = self.nodes.read();

        for partition_id in 0..self.partition_count {
            let has_node = nodes.values()
                .any(|n| n.healthy && n.assigned_partitions.contains(&partition_id));

            if !has_node {
                return false;
            }
        }

        true
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.read().len()
    }

    /// Get healthy node count
    pub fn healthy_node_count(&self) -> usize {
        self.nodes.read().values().filter(|n| n.healthy).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_registry() {
        let registry = NodeRegistry::new(32);
        assert_eq!(registry.node_count(), 0);
        assert_eq!(registry.healthy_node_count(), 0);
    }

    #[test]
    fn test_register_node() {
        let registry = NodeRegistry::new(32);

        let node = Node {
            id: 0,
            address: "node-0:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        };

        registry.register_node(node);

        assert_eq!(registry.node_count(), 1);
        assert_eq!(registry.healthy_node_count(), 1);

        // All partitions should be assigned to the single node
        let node = registry.get_node(0).unwrap();
        assert_eq!(node.assigned_partitions.len(), 32);
    }

    #[test]
    fn test_two_node_distribution() {
        let registry = NodeRegistry::new(32);

        registry.register_node(Node {
            id: 0,
            address: "node-0:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        registry.register_node(Node {
            id: 1,
            address: "node-1:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        // Each node should have ~16 partitions
        let node0 = registry.get_node(0).unwrap();
        let node1 = registry.get_node(1).unwrap();

        assert_eq!(node0.assigned_partitions.len(), 16);
        assert_eq!(node1.assigned_partitions.len(), 16);

        // Partitions should not overlap
        for partition_id in &node0.assigned_partitions {
            assert!(!node1.assigned_partitions.contains(partition_id));
        }
    }

    #[test]
    fn test_node_for_partition() {
        let registry = NodeRegistry::new(32);

        registry.register_node(Node {
            id: 0,
            address: "node-0:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        registry.register_node(Node {
            id: 1,
            address: "node-1:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        // Each partition should map to exactly one node
        for partition_id in 0..32 {
            let node_id = registry.node_for_partition(partition_id);
            assert!(node_id.is_some());
        }
    }

    #[test]
    fn test_unhealthy_node_excluded() {
        let registry = NodeRegistry::new(32);

        registry.register_node(Node {
            id: 0,
            address: "node-0:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        registry.register_node(Node {
            id: 1,
            address: "node-1:8080".to_string(),
            healthy: false, // Unhealthy
            assigned_partitions: vec![],
        });

        // All partitions should go to node 0
        let node0 = registry.get_node(0).unwrap();
        let node1 = registry.get_node(1).unwrap();

        assert_eq!(node0.assigned_partitions.len(), 32);
        assert_eq!(node1.assigned_partitions.len(), 0);
    }

    #[test]
    fn test_rebalance_on_health_change() {
        let registry = NodeRegistry::new(32);

        registry.register_node(Node {
            id: 0,
            address: "node-0:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        registry.register_node(Node {
            id: 1,
            address: "node-1:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        // Initially 16/16 split
        let node0_before = registry.get_node(0).unwrap();
        assert_eq!(node0_before.assigned_partitions.len(), 16);

        // Mark node 1 as unhealthy
        registry.set_node_health(1, false);

        // Node 0 should now have all 32 partitions
        let node0_after = registry.get_node(0).unwrap();
        assert_eq!(node0_after.assigned_partitions.len(), 32);
    }

    #[test]
    fn test_cluster_health() {
        let registry = NodeRegistry::new(32);

        // No nodes - unhealthy
        assert!(!registry.is_cluster_healthy());

        // Add one healthy node
        registry.register_node(Node {
            id: 0,
            address: "node-0:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        assert!(registry.is_cluster_healthy());

        // Mark unhealthy
        registry.set_node_health(0, false);
        assert!(!registry.is_cluster_healthy());
    }

    #[test]
    fn test_partition_distribution() {
        let registry = NodeRegistry::new(32);

        registry.register_node(Node {
            id: 0,
            address: "node-0:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        registry.register_node(Node {
            id: 1,
            address: "node-1:8080".to_string(),
            healthy: true,
            assigned_partitions: vec![],
        });

        let distribution = registry.partition_distribution();

        assert_eq!(distribution.len(), 2);
        assert_eq!(distribution.get(&0).unwrap().len(), 16);
        assert_eq!(distribution.get(&1).unwrap().len(), 16);
    }

    #[test]
    fn test_deterministic_assignment() {
        let registry1 = NodeRegistry::new(32);
        let registry2 = NodeRegistry::new(32);

        // Register same nodes in same order
        for i in 0..4 {
            let node = Node {
                id: i,
                address: format!("node-{}:8080", i),
                healthy: true,
                assigned_partitions: vec![],
            };

            registry1.register_node(node.clone());
            registry2.register_node(node);
        }

        // Partition assignments should be identical
        for partition_id in 0..32 {
            let node1 = registry1.node_for_partition(partition_id);
            let node2 = registry2.node_for_partition(partition_id);

            assert_eq!(node1, node2);
        }
    }
}
