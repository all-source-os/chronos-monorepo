use crate::error::{AllSourceError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

/// Partition key for distributing events across fixed partitions
///
/// SierraDB uses 32 fixed partitions for single-node, 1024+ for clusters.
/// We start with 32 for single-node deployment, ready for clustering.
///
/// # Invariants
/// - Partition count is fixed at construction (default: 32)
/// - Partition ID is always in range [0, partition_count)
/// - Same entity always maps to same partition (consistent hashing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionKey {
    partition_id: u32,
    partition_count: u32,
}

impl PartitionKey {
    /// Default partition count (SierraDB uses 32 for single-node)
    pub const DEFAULT_PARTITION_COUNT: u32 = 32;

    /// Create a partition key from an entity ID
    ///
    /// Uses consistent hashing to ensure same entity always maps to same partition.
    /// This is critical for ordering guarantees within a partition.
    pub fn from_entity_id(entity_id: &str) -> Self {
        Self::from_entity_id_with_count(entity_id, Self::DEFAULT_PARTITION_COUNT)
    }

    /// Create a partition key with custom partition count
    pub fn from_entity_id_with_count(entity_id: &str, partition_count: u32) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        entity_id.hash(&mut hasher);
        let hash = hasher.finish();
        let partition_id = (hash % partition_count as u64) as u32;

        Self {
            partition_id,
            partition_count,
        }
    }

    /// Create from explicit partition ID (for reconstruction)
    pub fn from_partition_id(partition_id: u32, partition_count: u32) -> Result<Self> {
        if partition_id >= partition_count {
            return Err(AllSourceError::InvalidInput(format!(
                "Partition ID {} exceeds partition count {}",
                partition_id, partition_count
            )));
        }

        Ok(Self {
            partition_id,
            partition_count,
        })
    }

    /// Get partition ID
    pub fn partition_id(&self) -> u32 {
        self.partition_id
    }

    /// Get partition count
    pub fn partition_count(&self) -> u32 {
        self.partition_count
    }

    /// Check if this partition belongs to a specific node (for clustering)
    pub fn belongs_to_node(&self, node_id: u32, total_nodes: u32) -> bool {
        self.partition_id % total_nodes == node_id
    }
}

impl fmt::Display for PartitionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "partition-{}/{}", self.partition_id, self.partition_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hashing() {
        let entity_id = "user-123";
        let key1 = PartitionKey::from_entity_id(entity_id);
        let key2 = PartitionKey::from_entity_id(entity_id);

        assert_eq!(key1, key2, "Same entity must always map to same partition");
    }

    #[test]
    fn test_partition_range() {
        let key = PartitionKey::from_entity_id("test");
        assert!(key.partition_id() < PartitionKey::DEFAULT_PARTITION_COUNT);
    }

    #[test]
    fn test_distribution() {
        let mut partition_counts = vec![0; PartitionKey::DEFAULT_PARTITION_COUNT as usize];

        for i in 0..1000 {
            let entity_id = format!("entity-{}", i);
            let key = PartitionKey::from_entity_id(&entity_id);
            partition_counts[key.partition_id() as usize] += 1;
        }

        // Check reasonable distribution (no partition should be empty or overloaded)
        for (idx, &count) in partition_counts.iter().enumerate() {
            assert!(count > 10, "Partition {} too few events: {}", idx, count);
            assert!(count < 60, "Partition {} too many events: {}", idx, count);
        }
    }

    #[test]
    fn test_node_assignment() {
        let key = PartitionKey::from_partition_id(0, 32).unwrap();
        assert!(key.belongs_to_node(0, 4)); // 0 % 4 = 0

        let key = PartitionKey::from_partition_id(5, 32).unwrap();
        assert!(key.belongs_to_node(1, 4)); // 5 % 4 = 1
    }

    #[test]
    fn test_invalid_partition_id() {
        let result = PartitionKey::from_partition_id(32, 32);
        assert!(result.is_err());
    }

    #[test]
    fn test_display() {
        let key = PartitionKey::from_partition_id(5, 32).unwrap();
        assert_eq!(key.to_string(), "partition-5/32");
    }
}
