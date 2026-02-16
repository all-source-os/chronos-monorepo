/// Simplified Raft-inspired Term-Based Consensus
///
/// Provides term-based leader election and cluster membership management
/// for multi-node AllSource Core deployments.
///
/// # Design
///
/// This is a simplified Raft implementation focused on:
/// - **Term-based leader election**: Monotonically increasing terms prevent split-brain
/// - **Deterministic leader selection**: Highest term wins; on tie, highest WAL offset wins
/// - **Manual failover support**: Integrate with existing `/internal/promote` + `/internal/repoint`
///
/// NOT implemented (handled by existing infrastructure):
/// - Log replication (WAL shipping handles this)
/// - Snapshot transfer (Parquet catch-up handles this)
///
/// # Architecture
///
/// ```text
/// ClusterManager
///   ├── NodeRegistry (partition assignment, health tracking)
///   ├── term: AtomicU64 (current consensus term)
///   ├── voted_for: RwLock<Option<u32>> (who we voted for this term)
///   └── members: DashMap<u32, ClusterMember> (full member metadata)
/// ```
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::RwLock;

use super::node_registry::{Node, NodeRegistry};

/// Full cluster member metadata (extends Node with consensus state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    /// Unique node ID
    pub node_id: u32,
    /// Network address for API traffic (host:port)
    pub api_address: String,
    /// Network address for replication traffic (host:port)
    pub replication_address: String,
    /// Current role
    pub role: MemberRole,
    /// Last known WAL offset (used for leader selection)
    pub last_wal_offset: u64,
    /// Last heartbeat timestamp (millis since epoch)
    pub last_heartbeat_ms: u64,
    /// Whether the node is healthy
    pub healthy: bool,
}

/// Role of a cluster member
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    Leader,
    Follower,
    Candidate,
}

/// Vote request for leader election
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    /// Term number for this election
    pub term: u64,
    /// Candidate requesting the vote
    pub candidate_id: u32,
    /// Candidate's last WAL offset (for log completeness check)
    pub last_wal_offset: u64,
}

/// Vote response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    /// Current term (may be higher than requested)
    pub term: u64,
    /// Whether the vote was granted
    pub vote_granted: bool,
}

/// Cluster status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    /// Current consensus term
    pub term: u64,
    /// Current leader node ID (if known)
    pub leader_id: Option<u32>,
    /// This node's ID
    pub self_id: u32,
    /// This node's role
    pub self_role: MemberRole,
    /// Total members
    pub member_count: usize,
    /// Healthy members
    pub healthy_count: usize,
    /// Partition count
    pub partition_count: u32,
    /// All members
    pub members: Vec<ClusterMember>,
}

/// Cluster Manager — coordinates consensus, membership, and partition assignment
pub struct ClusterManager {
    /// This node's ID
    self_id: u32,
    /// This node's role
    role: RwLock<MemberRole>,
    /// Current consensus term (monotonically increasing)
    term: AtomicU64,
    /// Who we voted for in the current term
    voted_for: RwLock<Option<u32>>,
    /// Current leader ID
    leader_id: RwLock<Option<u32>>,
    /// Cluster members (full metadata)
    members: DashMap<u32, ClusterMember>,
    /// Node registry for partition assignment
    registry: Arc<NodeRegistry>,
}

impl ClusterManager {
    /// Create a new cluster manager
    ///
    /// # Arguments
    /// - `self_id`: This node's unique ID
    /// - `partition_count`: Total partitions for the cluster
    pub fn new(self_id: u32, partition_count: u32) -> Self {
        Self {
            self_id,
            role: RwLock::new(MemberRole::Follower),
            term: AtomicU64::new(0),
            voted_for: RwLock::new(None),
            leader_id: RwLock::new(None),
            members: DashMap::new(),
            registry: Arc::new(NodeRegistry::new(partition_count)),
        }
    }

    /// Get the underlying node registry (for partition routing)
    pub fn registry(&self) -> &Arc<NodeRegistry> {
        &self.registry
    }

    /// Get this node's ID
    pub fn self_id(&self) -> u32 {
        self.self_id
    }

    /// Get the current term
    pub fn current_term(&self) -> u64 {
        self.term.load(Ordering::SeqCst)
    }

    /// Get this node's current role
    pub async fn current_role(&self) -> MemberRole {
        *self.role.read().await
    }

    /// Get the current leader ID
    pub async fn leader_id(&self) -> Option<u32> {
        *self.leader_id.read().await
    }

    // -------------------------------------------------------------------------
    // Membership management
    // -------------------------------------------------------------------------

    /// Register a new member in the cluster
    pub async fn add_member(&self, member: ClusterMember) {
        let node_id = member.node_id;
        let healthy = member.healthy;

        // Register in partition registry
        self.registry.register_node(Node {
            id: node_id,
            address: member.api_address.clone(),
            healthy,
            assigned_partitions: vec![],
        });

        // Store full member metadata
        self.members.insert(node_id, member);

        // If adding self as leader, update role
        if node_id == self.self_id {
            let mut role = self.role.write().await;
            let member_ref = self.members.get(&node_id).unwrap();
            *role = member_ref.role;
        }
    }

    /// Remove a member from the cluster
    pub async fn remove_member(&self, node_id: u32) -> Option<ClusterMember> {
        self.registry.unregister_node(node_id);
        let removed = self.members.remove(&node_id).map(|(_, m)| m);

        // If we removed the leader, clear leader_id
        let leader = *self.leader_id.read().await;
        if leader == Some(node_id) {
            *self.leader_id.write().await = None;
        }

        removed
    }

    /// Update a member's health status and WAL offset
    pub fn update_member_heartbeat(&self, node_id: u32, wal_offset: u64, healthy: bool) {
        if let Some(mut member) = self.members.get_mut(&node_id) {
            member.last_wal_offset = wal_offset;
            member.healthy = healthy;
            member.last_heartbeat_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
        }
        self.registry.set_node_health(node_id, healthy);
    }

    /// Get a member by ID
    pub fn get_member(&self, node_id: u32) -> Option<ClusterMember> {
        self.members.get(&node_id).map(|m| m.clone())
    }

    /// Get all members
    pub fn all_members(&self) -> Vec<ClusterMember> {
        self.members.iter().map(|m| m.value().clone()).collect()
    }

    /// Get healthy members
    pub fn healthy_members(&self) -> Vec<ClusterMember> {
        self.members
            .iter()
            .filter(|m| m.value().healthy)
            .map(|m| m.value().clone())
            .collect()
    }

    /// Get the number of members
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    // -------------------------------------------------------------------------
    // Term-based consensus
    // -------------------------------------------------------------------------

    /// Handle a vote request (simplified Raft RequestVote RPC)
    ///
    /// Grants vote if:
    /// 1. Request term >= our term
    /// 2. We haven't voted for someone else in this term
    /// 3. Candidate's WAL offset >= our knowledge of the log
    pub async fn handle_vote_request(&self, request: &VoteRequest) -> VoteResponse {
        let current_term = self.term.load(Ordering::SeqCst);

        // If candidate's term is stale, reject
        if request.term < current_term {
            return VoteResponse {
                term: current_term,
                vote_granted: false,
            };
        }

        // If candidate's term is higher, step down and update term
        if request.term > current_term {
            self.term.store(request.term, Ordering::SeqCst);
            *self.voted_for.write().await = None;
            // Step down to follower if we were leader/candidate
            *self.role.write().await = MemberRole::Follower;
        }

        let mut voted_for = self.voted_for.write().await;
        let current_term = self.term.load(Ordering::SeqCst);

        // Check if we can vote for this candidate
        let can_vote = match *voted_for {
            None => true,
            Some(id) => id == request.candidate_id,
        };

        if can_vote {
            // Check log completeness: candidate must be at least as up-to-date
            let self_offset = self
                .members
                .get(&self.self_id)
                .map(|m| m.last_wal_offset)
                .unwrap_or(0);

            if request.last_wal_offset >= self_offset {
                *voted_for = Some(request.candidate_id);
                return VoteResponse {
                    term: current_term,
                    vote_granted: true,
                };
            }
        }

        VoteResponse {
            term: current_term,
            vote_granted: false,
        }
    }

    /// Start an election: increment term and vote for self
    ///
    /// Returns the new term. Caller should then send VoteRequests to other nodes.
    pub async fn start_election(&self) -> u64 {
        let new_term = self.term.fetch_add(1, Ordering::SeqCst) + 1;
        *self.role.write().await = MemberRole::Candidate;
        *self.voted_for.write().await = Some(self.self_id);
        *self.leader_id.write().await = None;
        new_term
    }

    /// Declare self as leader after winning an election
    ///
    /// Should only be called after receiving a majority of votes.
    pub async fn become_leader(&self, term: u64) {
        let current_term = self.term.load(Ordering::SeqCst);
        if term != current_term {
            return; // Stale election — a new term started
        }
        *self.role.write().await = MemberRole::Leader;
        *self.leader_id.write().await = Some(self.self_id);

        // Update self in members list
        if let Some(mut member) = self.members.get_mut(&self.self_id) {
            member.role = MemberRole::Leader;
        }
    }

    /// Accept a leader (after receiving an AppendEntries or heartbeat from a valid leader)
    pub async fn accept_leader(&self, leader_id: u32, term: u64) {
        let current_term = self.term.load(Ordering::SeqCst);
        if term < current_term {
            return; // Stale leader
        }
        if term > current_term {
            self.term.store(term, Ordering::SeqCst);
            *self.voted_for.write().await = None;
        }
        *self.role.write().await = MemberRole::Follower;
        *self.leader_id.write().await = Some(leader_id);

        // Update member roles
        for mut member in self.members.iter_mut() {
            member.role = if member.node_id == leader_id {
                MemberRole::Leader
            } else {
                MemberRole::Follower
            };
        }
    }

    /// Select the best candidate for leader from healthy members
    ///
    /// Deterministic: highest WAL offset wins; on tie, lowest node ID wins.
    pub fn select_leader_candidate(&self) -> Option<u32> {
        let mut best: Option<(u32, u64)> = None; // (node_id, wal_offset)

        for member in self.members.iter() {
            if !member.healthy {
                continue;
            }
            match best {
                None => best = Some((member.node_id, member.last_wal_offset)),
                Some((_, best_offset)) => {
                    if member.last_wal_offset > best_offset
                        || (member.last_wal_offset == best_offset
                            && member.node_id < best.unwrap().0)
                    {
                        best = Some((member.node_id, member.last_wal_offset));
                    }
                }
            }
        }

        best.map(|(id, _)| id)
    }

    // -------------------------------------------------------------------------
    // Status
    // -------------------------------------------------------------------------

    /// Get full cluster status
    pub async fn status(&self) -> ClusterStatus {
        ClusterStatus {
            term: self.term.load(Ordering::SeqCst),
            leader_id: *self.leader_id.read().await,
            self_id: self.self_id,
            self_role: *self.role.read().await,
            member_count: self.members.len(),
            healthy_count: self.registry.healthy_node_count(),
            partition_count: self
                .registry
                .partition_distribution()
                .values()
                .flat_map(|v| v.iter())
                .count() as u32,
            members: self.all_members(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_member(id: u32, role: MemberRole, offset: u64) -> ClusterMember {
        ClusterMember {
            node_id: id,
            api_address: format!("node-{}:3900", id),
            replication_address: format!("node-{}:3910", id),
            role,
            last_wal_offset: offset,
            last_heartbeat_ms: 0,
            healthy: true,
        }
    }

    #[tokio::test]
    async fn test_create_cluster_manager() {
        let cm = ClusterManager::new(0, 32);
        assert_eq!(cm.self_id(), 0);
        assert_eq!(cm.current_term(), 0);
        assert_eq!(cm.current_role().await, MemberRole::Follower);
        assert_eq!(cm.leader_id().await, None);
        assert_eq!(cm.member_count(), 0);
    }

    #[tokio::test]
    async fn test_add_and_remove_members() {
        let cm = ClusterManager::new(0, 32);

        cm.add_member(make_member(0, MemberRole::Leader, 100)).await;
        cm.add_member(make_member(1, MemberRole::Follower, 90))
            .await;
        cm.add_member(make_member(2, MemberRole::Follower, 80))
            .await;

        assert_eq!(cm.member_count(), 3);
        assert_eq!(cm.healthy_members().len(), 3);

        // Verify partition distribution
        let dist = cm.registry().partition_distribution();
        assert_eq!(dist.len(), 3);

        // Remove a member
        let removed = cm.remove_member(2).await;
        assert!(removed.is_some());
        assert_eq!(cm.member_count(), 2);
    }

    #[tokio::test]
    async fn test_heartbeat_update() {
        let cm = ClusterManager::new(0, 32);
        cm.add_member(make_member(1, MemberRole::Follower, 50))
            .await;

        cm.update_member_heartbeat(1, 100, true);
        let member = cm.get_member(1).unwrap();
        assert_eq!(member.last_wal_offset, 100);
        assert!(member.last_heartbeat_ms > 0);

        // Mark unhealthy
        cm.update_member_heartbeat(1, 100, false);
        let member = cm.get_member(1).unwrap();
        assert!(!member.healthy);
    }

    #[tokio::test]
    async fn test_deterministic_leader_selection() {
        let cm = ClusterManager::new(0, 32);

        cm.add_member(make_member(0, MemberRole::Follower, 100))
            .await;
        cm.add_member(make_member(1, MemberRole::Follower, 200))
            .await;
        cm.add_member(make_member(2, MemberRole::Follower, 150))
            .await;

        // Node 1 has highest offset → should be selected
        let candidate = cm.select_leader_candidate();
        assert_eq!(candidate, Some(1));
    }

    #[tokio::test]
    async fn test_deterministic_leader_selection_tiebreak() {
        let cm = ClusterManager::new(0, 32);

        cm.add_member(make_member(0, MemberRole::Follower, 100))
            .await;
        cm.add_member(make_member(1, MemberRole::Follower, 100))
            .await;
        cm.add_member(make_member(2, MemberRole::Follower, 100))
            .await;

        // Same offset → lowest node ID wins
        let candidate = cm.select_leader_candidate();
        assert_eq!(candidate, Some(0));
    }

    #[tokio::test]
    async fn test_leader_selection_skips_unhealthy() {
        let cm = ClusterManager::new(0, 32);

        cm.add_member(make_member(0, MemberRole::Follower, 200))
            .await;
        cm.add_member(make_member(1, MemberRole::Follower, 100))
            .await;

        // Mark node 0 (highest offset) as unhealthy
        cm.update_member_heartbeat(0, 200, false);

        let candidate = cm.select_leader_candidate();
        assert_eq!(candidate, Some(1));
    }

    #[tokio::test]
    async fn test_vote_request_grants_vote() {
        let cm = ClusterManager::new(0, 32);
        cm.add_member(make_member(0, MemberRole::Follower, 50))
            .await;

        let request = VoteRequest {
            term: 1,
            candidate_id: 1,
            last_wal_offset: 100,
        };

        let response = cm.handle_vote_request(&request).await;
        assert!(response.vote_granted);
        assert_eq!(response.term, 1);
    }

    #[tokio::test]
    async fn test_vote_request_rejects_stale_term() {
        let cm = ClusterManager::new(0, 32);
        cm.add_member(make_member(0, MemberRole::Follower, 50))
            .await;

        // Advance term to 5
        cm.term.store(5, Ordering::SeqCst);

        let request = VoteRequest {
            term: 3, // Stale
            candidate_id: 1,
            last_wal_offset: 100,
        };

        let response = cm.handle_vote_request(&request).await;
        assert!(!response.vote_granted);
        assert_eq!(response.term, 5);
    }

    #[tokio::test]
    async fn test_vote_request_rejects_duplicate_vote() {
        let cm = ClusterManager::new(0, 32);
        cm.add_member(make_member(0, MemberRole::Follower, 50))
            .await;

        // Vote for candidate 1
        let request1 = VoteRequest {
            term: 1,
            candidate_id: 1,
            last_wal_offset: 100,
        };
        let response1 = cm.handle_vote_request(&request1).await;
        assert!(response1.vote_granted);

        // Candidate 2 asks for vote in same term → reject
        let request2 = VoteRequest {
            term: 1,
            candidate_id: 2,
            last_wal_offset: 100,
        };
        let response2 = cm.handle_vote_request(&request2).await;
        assert!(!response2.vote_granted);
    }

    #[tokio::test]
    async fn test_start_election() {
        let cm = ClusterManager::new(0, 32);

        let new_term = cm.start_election().await;
        assert_eq!(new_term, 1);
        assert_eq!(cm.current_term(), 1);
        assert_eq!(cm.current_role().await, MemberRole::Candidate);
    }

    #[tokio::test]
    async fn test_become_leader() {
        let cm = ClusterManager::new(0, 32);
        cm.add_member(make_member(0, MemberRole::Follower, 100))
            .await;

        let term = cm.start_election().await;
        cm.become_leader(term).await;

        assert_eq!(cm.current_role().await, MemberRole::Leader);
        assert_eq!(cm.leader_id().await, Some(0));
    }

    #[tokio::test]
    async fn test_accept_leader() {
        let cm = ClusterManager::new(1, 32);
        cm.add_member(make_member(0, MemberRole::Follower, 100))
            .await;
        cm.add_member(make_member(1, MemberRole::Follower, 90))
            .await;

        cm.accept_leader(0, 1).await;

        assert_eq!(cm.current_role().await, MemberRole::Follower);
        assert_eq!(cm.leader_id().await, Some(0));
        assert_eq!(cm.current_term(), 1);

        // Verify member roles updated
        let leader = cm.get_member(0).unwrap();
        assert_eq!(leader.role, MemberRole::Leader);
    }

    #[tokio::test]
    async fn test_stale_become_leader_ignored() {
        let cm = ClusterManager::new(0, 32);
        cm.add_member(make_member(0, MemberRole::Follower, 100))
            .await;

        let _term = cm.start_election().await; // term=1

        // Meanwhile, someone else increments term
        cm.accept_leader(1, 2).await; // term=2

        // Trying to become leader for term 1 should be ignored
        cm.become_leader(1).await;
        assert_eq!(cm.current_role().await, MemberRole::Follower);
        assert_eq!(cm.leader_id().await, Some(1));
    }

    #[tokio::test]
    async fn test_cluster_status() {
        let cm = ClusterManager::new(0, 32);
        cm.add_member(make_member(0, MemberRole::Leader, 100)).await;
        cm.add_member(make_member(1, MemberRole::Follower, 90))
            .await;

        let status = cm.status().await;
        assert_eq!(status.self_id, 0);
        assert_eq!(status.member_count, 2);
        assert_eq!(status.healthy_count, 2);
        assert_eq!(status.members.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_leader_clears_leader_id() {
        let cm = ClusterManager::new(1, 32);
        cm.add_member(make_member(0, MemberRole::Leader, 100)).await;
        cm.add_member(make_member(1, MemberRole::Follower, 90))
            .await;

        *cm.leader_id.write().await = Some(0);

        cm.remove_member(0).await;
        assert_eq!(cm.leader_id().await, None);
    }
}
