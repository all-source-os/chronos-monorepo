use crate::{
    domain::{
        entities::Event,
        value_objects::{EntityId, ForkId, TenantId},
    },
    error::{AllSourceError, Result},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fork status indicating the lifecycle stage
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForkStatus {
    /// Fork is active and can receive events
    #[default]
    Active,
    /// Fork has been merged into parent
    Merged,
    /// Fork has been discarded
    Discarded,
    /// Fork has expired and is pending cleanup
    Expired,
}

/// Fork isolation level determining what's shared with parent
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForkIsolationLevel {
    /// Fork sees parent events but cannot modify them
    #[default]
    ReadParent,
    /// Fork has complete isolation, no parent visibility
    Complete,
    /// Fork can see parent events added after fork creation
    LiveParent,
}

/// Domain Entity: EventStoreFork
///
/// Represents an isolated fork of the event store for agent experimentation.
/// Implements the Agentic Postgres pattern for safe AI experimentation.
///
/// # Agentic Postgres Pattern
/// This pattern enables AI agents to:
/// - Create isolated copies of event streams for experimentation
/// - Try different approaches without affecting production data
/// - Compare outcomes across multiple experimental branches
/// - Safely rollback failed experiments
/// - Commit successful experiments to the main event store
///
/// # Domain Rules
/// - Forks are always scoped to a tenant
/// - Forks have a time-to-live (TTL) for automatic cleanup
/// - Parent fork reference enables fork hierarchy
/// - Fork events are stored separately from main events
/// - Forks can be merged back into parent/main store
/// - Discarded forks release their resources
///
/// # Use Cases
/// - AI agents experimenting with different event sequences
/// - A/B testing of event processing pipelines
/// - Debugging complex event sourcing scenarios
/// - Safe replay and modification of event history
/// - Training data generation for ML models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStoreFork {
    /// Unique identifier for this fork
    id: ForkId,

    /// Tenant this fork belongs to
    tenant_id: TenantId,

    /// Human-readable name for the fork
    name: String,

    /// Optional description of the fork's purpose
    description: Option<String>,

    /// Parent fork ID (None if forked from main store)
    parent_fork_id: Option<ForkId>,

    /// Version of parent at fork creation (for consistency)
    parent_version: u64,

    /// Fork status
    status: ForkStatus,

    /// Isolation level
    isolation_level: ForkIsolationLevel,

    /// Events stored in this fork (entity_id -> events)
    events: HashMap<String, Vec<Event>>,

    /// Total event count in this fork
    event_count: u64,

    /// When this fork expires (for auto-cleanup)
    expires_at: DateTime<Utc>,

    /// Agent ID that created this fork (for tracking)
    created_by_agent: Option<String>,

    /// Creation timestamp
    created_at: DateTime<Utc>,

    /// Last modification timestamp
    updated_at: DateTime<Utc>,

    /// Optional metadata
    metadata: serde_json::Value,
}

impl EventStoreFork {
    /// Default TTL for forks (24 hours)
    pub const DEFAULT_TTL_HOURS: i64 = 24;

    /// Maximum TTL for forks (7 days)
    pub const MAX_TTL_HOURS: i64 = 24 * 7;

    /// Create a new event store fork
    ///
    /// # Arguments
    /// - `tenant_id`: Tenant this fork belongs to
    /// - `name`: Human-readable name for the fork
    /// - `parent_version`: Version of parent store at fork time
    ///
    /// # Returns
    /// New EventStoreFork with default settings
    pub fn new(tenant_id: TenantId, name: String, parent_version: u64) -> Result<Self> {
        Self::validate_name(&name)?;

        let now = Utc::now();
        let expires_at = now + Duration::hours(Self::DEFAULT_TTL_HOURS);

        Ok(Self {
            id: ForkId::new(),
            tenant_id,
            name,
            description: None,
            parent_fork_id: None,
            parent_version,
            status: ForkStatus::Active,
            isolation_level: ForkIsolationLevel::ReadParent,
            events: HashMap::new(),
            event_count: 0,
            expires_at,
            created_by_agent: None,
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
        })
    }

    /// Create a child fork from an existing fork
    ///
    /// # Arguments
    /// - `parent`: Parent fork to branch from
    /// - `name`: Name for the new fork
    ///
    /// # Returns
    /// New child fork with parent reference
    pub fn branch_from(parent: &EventStoreFork, name: String) -> Result<Self> {
        Self::validate_name(&name)?;

        if parent.status != ForkStatus::Active {
            return Err(AllSourceError::ValidationError(
                "Cannot branch from non-active fork".to_string(),
            ));
        }

        let now = Utc::now();
        let expires_at = now + Duration::hours(Self::DEFAULT_TTL_HOURS);

        Ok(Self {
            id: ForkId::new(),
            tenant_id: parent.tenant_id.clone(),
            name,
            description: None,
            parent_fork_id: Some(parent.id.clone()),
            parent_version: parent.event_count,
            status: ForkStatus::Active,
            isolation_level: parent.isolation_level,
            events: HashMap::new(),
            event_count: 0,
            expires_at,
            created_by_agent: None,
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
        })
    }

    /// Reconstruct fork from storage (bypasses validation)
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: ForkId,
        tenant_id: TenantId,
        name: String,
        description: Option<String>,
        parent_fork_id: Option<ForkId>,
        parent_version: u64,
        status: ForkStatus,
        isolation_level: ForkIsolationLevel,
        events: HashMap<String, Vec<Event>>,
        event_count: u64,
        expires_at: DateTime<Utc>,
        created_by_agent: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            tenant_id,
            name,
            description,
            parent_fork_id,
            parent_version,
            status,
            isolation_level,
            events,
            event_count,
            expires_at,
            created_by_agent,
            created_at,
            updated_at,
            metadata,
        }
    }

    // Getters

    pub fn id(&self) -> &ForkId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn parent_fork_id(&self) -> Option<&ForkId> {
        self.parent_fork_id.as_ref()
    }

    pub fn parent_version(&self) -> u64 {
        self.parent_version
    }

    pub fn status(&self) -> ForkStatus {
        self.status
    }

    pub fn isolation_level(&self) -> ForkIsolationLevel {
        self.isolation_level
    }

    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub fn created_by_agent(&self) -> Option<&str> {
        self.created_by_agent.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    /// Check if fork is active
    pub fn is_active(&self) -> bool {
        self.status == ForkStatus::Active
    }

    /// Check if fork has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if fork has a parent
    pub fn has_parent(&self) -> bool {
        self.parent_fork_id.is_some()
    }

    // Domain behavior methods

    /// Append an event to this fork
    ///
    /// # Errors
    /// Returns error if fork is not active
    pub fn append_event(&mut self, event: Event) -> Result<()> {
        if self.status != ForkStatus::Active {
            return Err(AllSourceError::ValidationError(format!(
                "Cannot append to {:?} fork",
                self.status
            )));
        }

        if self.is_expired() {
            return Err(AllSourceError::ValidationError(
                "Fork has expired".to_string(),
            ));
        }

        // Validate tenant
        if event.tenant_id() != &self.tenant_id {
            return Err(AllSourceError::ValidationError(format!(
                "Event tenant '{}' does not match fork tenant '{}'",
                event.tenant_id().as_str(),
                self.tenant_id.as_str()
            )));
        }

        let entity_id = event.entity_id().to_string();
        self.events.entry(entity_id).or_default().push(event);
        self.event_count += 1;
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Get events for a specific entity
    pub fn events_for_entity(&self, entity_id: &EntityId) -> Vec<&Event> {
        self.events
            .get(entity_id.as_str())
            .map(|events| events.iter().collect())
            .unwrap_or_default()
    }

    /// Get all events in the fork
    pub fn all_events(&self) -> Vec<&Event> {
        self.events
            .values()
            .flat_map(|events| events.iter())
            .collect()
    }

    /// Get entity IDs that have events in this fork
    pub fn entity_ids(&self) -> Vec<&str> {
        self.events
            .keys()
            .map(std::string::String::as_str)
            .collect()
    }

    /// Set description
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
        self.updated_at = Utc::now();
    }

    /// Set isolation level
    pub fn set_isolation_level(&mut self, level: ForkIsolationLevel) -> Result<()> {
        if self.status != ForkStatus::Active {
            return Err(AllSourceError::ValidationError(
                "Cannot change isolation level of non-active fork".to_string(),
            ));
        }

        self.isolation_level = level;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Set the agent that created this fork
    pub fn set_created_by_agent(&mut self, agent_id: String) {
        self.created_by_agent = Some(agent_id);
        self.updated_at = Utc::now();
    }

    /// Extend the expiration time
    pub fn extend_ttl(&mut self, hours: i64) -> Result<()> {
        if self.status != ForkStatus::Active {
            return Err(AllSourceError::ValidationError(
                "Cannot extend TTL of non-active fork".to_string(),
            ));
        }

        let new_expires = self.expires_at + Duration::hours(hours);
        let max_expires = Utc::now() + Duration::hours(Self::MAX_TTL_HOURS);

        if new_expires > max_expires {
            return Err(AllSourceError::ValidationError(format!(
                "TTL extension would exceed maximum of {} hours",
                Self::MAX_TTL_HOURS
            )));
        }

        self.expires_at = new_expires;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark fork as merged (events committed to parent)
    pub fn mark_merged(&mut self) -> Result<()> {
        if self.status != ForkStatus::Active {
            return Err(AllSourceError::ValidationError(format!(
                "Cannot merge {:?} fork",
                self.status
            )));
        }

        self.status = ForkStatus::Merged;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Discard the fork (events discarded)
    pub fn discard(&mut self) -> Result<()> {
        if self.status != ForkStatus::Active {
            return Err(AllSourceError::ValidationError(format!(
                "Cannot discard {:?} fork",
                self.status
            )));
        }

        self.status = ForkStatus::Discarded;
        self.events.clear();
        self.event_count = 0;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark fork as expired (for cleanup)
    pub fn mark_expired(&mut self) {
        self.status = ForkStatus::Expired;
        self.updated_at = Utc::now();
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }

    // Validation

    fn validate_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(AllSourceError::InvalidInput(
                "Fork name cannot be empty".to_string(),
            ));
        }

        if name.len() > 128 {
            return Err(AllSourceError::InvalidInput(
                "Fork name cannot exceed 128 characters".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn create_test_event() -> Event {
        Event::from_strings(
            "test.event".to_string(),
            "entity-1".to_string(),
            "test-tenant".to_string(),
            json!({"data": "test"}),
            None,
        )
        .unwrap()
    }

    #[test]
    fn test_create_fork() {
        let fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0);

        assert!(fork.is_ok());
        let fork = fork.unwrap();
        assert_eq!(fork.name(), "test-fork");
        assert!(fork.is_active());
        assert!(!fork.is_expired());
        assert!(!fork.has_parent());
        assert_eq!(fork.event_count(), 0);
    }

    #[test]
    fn test_reject_empty_name() {
        let result = EventStoreFork::new(test_tenant_id(), String::new(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_long_name() {
        let long_name = "a".repeat(129);
        let result = EventStoreFork::new(test_tenant_id(), long_name, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_append_event() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        let event = create_test_event();
        let result = fork.append_event(event);

        assert!(result.is_ok());
        assert_eq!(fork.event_count(), 1);
    }

    #[test]
    fn test_append_event_wrong_tenant() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        let event = Event::from_strings(
            "test.event".to_string(),
            "entity-1".to_string(),
            "wrong-tenant".to_string(),
            json!({"data": "test"}),
            None,
        )
        .unwrap();

        let result = fork.append_event(event);
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_append_to_discarded_fork() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        fork.discard().unwrap();

        let event = create_test_event();
        let result = fork.append_event(event);

        assert!(result.is_err());
    }

    #[test]
    fn test_branch_from() {
        let parent = EventStoreFork::new(test_tenant_id(), "parent-fork".to_string(), 0).unwrap();

        let child = EventStoreFork::branch_from(&parent, "child-fork".to_string());

        assert!(child.is_ok());
        let child = child.unwrap();
        assert!(child.has_parent());
        assert_eq!(child.parent_fork_id(), Some(parent.id()));
    }

    #[test]
    fn test_cannot_branch_from_discarded() {
        let mut parent =
            EventStoreFork::new(test_tenant_id(), "parent-fork".to_string(), 0).unwrap();

        parent.discard().unwrap();

        let result = EventStoreFork::branch_from(&parent, "child-fork".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_merge() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        let result = fork.mark_merged();

        assert!(result.is_ok());
        assert_eq!(fork.status(), ForkStatus::Merged);
        assert!(!fork.is_active());
    }

    #[test]
    fn test_discard() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        // Add some events
        fork.append_event(create_test_event()).unwrap();
        assert_eq!(fork.event_count(), 1);

        // Discard
        fork.discard().unwrap();

        assert_eq!(fork.status(), ForkStatus::Discarded);
        assert_eq!(fork.event_count(), 0);
    }

    #[test]
    fn test_extend_ttl() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        let original_expires = fork.expires_at();
        fork.extend_ttl(12).unwrap();

        assert!(fork.expires_at() > original_expires);
    }

    #[test]
    fn test_extend_ttl_max_limit() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        // Try to extend beyond max (7 days = 168 hours)
        let result = fork.extend_ttl(200);
        assert!(result.is_err());
    }

    #[test]
    fn test_events_for_entity() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        let event1 = create_test_event();
        let event2 = create_test_event();
        fork.append_event(event1).unwrap();
        fork.append_event(event2).unwrap();

        let entity_id = EntityId::new("entity-1".to_string()).unwrap();
        let events = fork.events_for_entity(&entity_id);

        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_all_events() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        for _ in 0..5 {
            fork.append_event(create_test_event()).unwrap();
        }

        let events = fork.all_events();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_set_isolation_level() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        assert_eq!(fork.isolation_level(), ForkIsolationLevel::ReadParent);

        fork.set_isolation_level(ForkIsolationLevel::Complete)
            .unwrap();

        assert_eq!(fork.isolation_level(), ForkIsolationLevel::Complete);
    }

    #[test]
    fn test_serde_serialization() {
        let fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        let json = serde_json::to_string(&fork);
        assert!(json.is_ok());

        let deserialized: EventStoreFork = serde_json::from_str(&json.unwrap()).unwrap();
        assert_eq!(deserialized.name(), "test-fork");
    }

    #[test]
    fn test_created_by_agent() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        assert!(fork.created_by_agent().is_none());

        fork.set_created_by_agent("agent-123".to_string());

        assert_eq!(fork.created_by_agent(), Some("agent-123"));
    }

    #[test]
    fn test_entity_ids() {
        let mut fork = EventStoreFork::new(test_tenant_id(), "test-fork".to_string(), 0).unwrap();

        fork.append_event(create_test_event()).unwrap();

        let entity_ids = fork.entity_ids();
        assert_eq!(entity_ids.len(), 1);
        assert!(entity_ids.contains(&"entity-1"));
    }
}
