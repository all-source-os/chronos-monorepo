use crate::domain::value_objects::{TenantId, EventType};
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Projection status
///
/// Tracks the current state of a projection's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectionStatus {
    /// Projection is defined but not yet started
    Created,
    /// Projection is actively processing events
    Running,
    /// Projection is temporarily paused
    Paused,
    /// Projection has encountered an error
    Failed,
    /// Projection has been stopped
    Stopped,
    /// Projection is being rebuilt from scratch
    Rebuilding,
}

impl ProjectionStatus {
    /// Check if projection is actively processing
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Rebuilding)
    }

    /// Check if projection can be started
    pub fn can_start(&self) -> bool {
        matches!(self, Self::Created | Self::Stopped | Self::Paused)
    }

    /// Check if projection can be paused
    pub fn can_pause(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Check if projection can be stopped
    pub fn can_stop(&self) -> bool {
        !matches!(self, Self::Stopped)
    }

    /// Check if projection is in error state
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// Projection type
///
/// Defines how the projection processes events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionType {
    /// Maintains current state by entity ID
    EntitySnapshot,
    /// Counts events by type
    EventCounter,
    /// Custom user-defined projection logic
    Custom,
    /// Time-series aggregation
    TimeSeries,
    /// Funnel analysis
    Funnel,
}

/// Projection configuration
///
/// Optional settings that control projection behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionConfig {
    /// Maximum number of events to process in a batch
    pub batch_size: usize,
    /// Whether to checkpoint state periodically
    pub enable_checkpoints: bool,
    /// Checkpoint interval in number of events
    pub checkpoint_interval: usize,
    /// Whether to process events in parallel
    pub parallel_processing: bool,
    /// Maximum concurrent event handlers
    pub max_concurrency: usize,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            enable_checkpoints: true,
            checkpoint_interval: 1000,
            parallel_processing: false,
            max_concurrency: 4,
        }
    }
}

/// Projection statistics
///
/// Tracks operational metrics for a projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionStats {
    events_processed: u64,
    last_processed_at: Option<DateTime<Utc>>,
    last_checkpoint_at: Option<DateTime<Utc>>,
    errors_count: u64,
    last_error_at: Option<DateTime<Utc>>,
    processing_time_ms: u64,
}

impl ProjectionStats {
    pub fn new() -> Self {
        Self {
            events_processed: 0,
            last_processed_at: None,
            last_checkpoint_at: None,
            errors_count: 0,
            last_error_at: None,
            processing_time_ms: 0,
        }
    }

    // Getters
    pub fn events_processed(&self) -> u64 {
        self.events_processed
    }

    pub fn errors_count(&self) -> u64 {
        self.errors_count
    }

    pub fn last_processed_at(&self) -> Option<DateTime<Utc>> {
        self.last_processed_at
    }

    pub fn processing_time_ms(&self) -> u64 {
        self.processing_time_ms
    }

    // Mutation methods
    pub fn record_event_processed(&mut self, processing_time_ms: u64) {
        self.events_processed += 1;
        self.last_processed_at = Some(Utc::now());
        self.processing_time_ms += processing_time_ms;
    }

    pub fn record_error(&mut self) {
        self.errors_count += 1;
        self.last_error_at = Some(Utc::now());
    }

    pub fn record_checkpoint(&mut self) {
        self.last_checkpoint_at = Some(Utc::now());
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Calculate average processing time per event in milliseconds
    pub fn avg_processing_time_ms(&self) -> f64 {
        if self.events_processed == 0 {
            0.0
        } else {
            self.processing_time_ms as f64 / self.events_processed as f64
        }
    }
}

impl Default for ProjectionStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Domain Entity: Projection
///
/// Represents a projection that aggregates events into a queryable view.
/// Projections are materialized views derived from the event stream.
///
/// Domain Rules:
/// - Name must be unique within a tenant
/// - Name cannot be empty
/// - Version starts at 1 and increments
/// - Cannot change projection type after creation
/// - Status transitions must follow lifecycle rules
/// - Stats are accurate and updated on each event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projection {
    id: Uuid,
    tenant_id: TenantId,
    name: String,
    version: u32,
    projection_type: ProjectionType,
    status: ProjectionStatus,
    config: ProjectionConfig,
    stats: ProjectionStats,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    stopped_at: Option<DateTime<Utc>>,
    description: Option<String>,
    /// Event types this projection is interested in (empty = all events)
    event_types: Vec<EventType>,
    /// Custom metadata
    metadata: serde_json::Value,
}

impl Projection {
    /// Create a new projection with validation
    pub fn new(
        tenant_id: TenantId,
        name: String,
        version: u32,
        projection_type: ProjectionType,
    ) -> Result<Self> {
        Self::validate_name(&name)?;
        Self::validate_version(version)?;

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            tenant_id,
            name,
            version,
            projection_type,
            status: ProjectionStatus::Created,
            config: ProjectionConfig::default(),
            stats: ProjectionStats::new(),
            created_at: now,
            updated_at: now,
            started_at: None,
            stopped_at: None,
            description: None,
            event_types: Vec::new(),
            metadata: serde_json::json!({}),
        })
    }

    /// Create first version of a projection
    pub fn new_v1(
        tenant_id: TenantId,
        name: String,
        projection_type: ProjectionType,
    ) -> Result<Self> {
        Self::new(tenant_id, name, 1, projection_type)
    }

    /// Reconstruct projection from storage (bypasses validation)
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: TenantId,
        name: String,
        version: u32,
        projection_type: ProjectionType,
        status: ProjectionStatus,
        config: ProjectionConfig,
        stats: ProjectionStats,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        started_at: Option<DateTime<Utc>>,
        stopped_at: Option<DateTime<Utc>>,
        description: Option<String>,
        event_types: Vec<EventType>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            tenant_id,
            name,
            version,
            projection_type,
            status,
            config,
            stats,
            created_at,
            updated_at,
            started_at,
            stopped_at,
            description,
            event_types,
            metadata,
        }
    }

    // Getters

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn projection_type(&self) -> ProjectionType {
        self.projection_type
    }

    pub fn status(&self) -> ProjectionStatus {
        self.status
    }

    pub fn config(&self) -> &ProjectionConfig {
        &self.config
    }

    pub fn stats(&self) -> &ProjectionStats {
        &self.stats
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn event_types(&self) -> &[EventType] {
        &self.event_types
    }

    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    // Domain behavior methods

    /// Start the projection
    pub fn start(&mut self) -> Result<()> {
        if !self.status.can_start() {
            return Err(crate::error::AllSourceError::ValidationError(
                format!("Cannot start projection in status {:?}", self.status),
            ));
        }

        self.status = ProjectionStatus::Running;
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Pause the projection
    pub fn pause(&mut self) -> Result<()> {
        if !self.status.can_pause() {
            return Err(crate::error::AllSourceError::ValidationError(
                format!("Cannot pause projection in status {:?}", self.status),
            ));
        }

        self.status = ProjectionStatus::Paused;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Stop the projection
    pub fn stop(&mut self) -> Result<()> {
        if !self.status.can_stop() {
            return Err(crate::error::AllSourceError::ValidationError(
                format!("Cannot stop projection in status {:?}", self.status),
            ));
        }

        self.status = ProjectionStatus::Stopped;
        self.stopped_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark projection as failed
    pub fn mark_failed(&mut self) {
        self.status = ProjectionStatus::Failed;
        self.updated_at = Utc::now();
    }

    /// Start rebuilding the projection
    pub fn start_rebuild(&mut self) -> Result<()> {
        self.status = ProjectionStatus::Rebuilding;
        self.stats.reset();
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ProjectionConfig) {
        self.config = config;
        self.updated_at = Utc::now();
    }

    /// Set description
    pub fn set_description(&mut self, description: String) -> Result<()> {
        Self::validate_description(&description)?;
        self.description = Some(description);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add event type filter
    pub fn add_event_type(&mut self, event_type: EventType) -> Result<()> {
        if self.event_types.contains(&event_type) {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Event type '{}' already in filter", event_type.as_str()),
            ));
        }

        self.event_types.push(event_type);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Remove event type filter
    pub fn remove_event_type(&mut self, event_type: &EventType) -> Result<()> {
        let initial_len = self.event_types.len();
        self.event_types.retain(|et| et != event_type);

        if self.event_types.len() == initial_len {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Event type '{}' not in filter", event_type.as_str()),
            ));
        }

        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if projection processes this event type
    pub fn processes_event_type(&self, event_type: &EventType) -> bool {
        // Empty filter means process all events
        self.event_types.is_empty() || self.event_types.contains(event_type)
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }

    /// Get mutable access to stats (for recording events)
    pub fn stats_mut(&mut self) -> &mut ProjectionStats {
        self.updated_at = Utc::now();
        &mut self.stats
    }

    /// Check if projection is first version
    pub fn is_first_version(&self) -> bool {
        self.version == 1
    }

    /// Check if projection belongs to tenant
    pub fn belongs_to_tenant(&self, tenant_id: &TenantId) -> bool {
        &self.tenant_id == tenant_id
    }

    /// Create next version
    pub fn create_next_version(&self) -> Result<Projection> {
        Projection::new(
            self.tenant_id.clone(),
            self.name.clone(),
            self.version + 1,
            self.projection_type,
        )
    }

    // Validation methods

    fn validate_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Projection name cannot be empty".to_string(),
            ));
        }

        if name.len() > 100 {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Projection name cannot exceed 100 characters, got {}", name.len()),
            ));
        }

        // Name should be alphanumeric with underscores/hyphens
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Projection name '{}' must be alphanumeric with underscores or hyphens", name),
            ));
        }

        Ok(())
    }

    fn validate_version(version: u32) -> Result<()> {
        if version == 0 {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Projection version must be >= 1".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_description(description: &str) -> Result<()> {
        if description.len() > 1000 {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Projection description cannot exceed 1000 characters, got {}", description.len()),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn test_event_type() -> EventType {
        EventType::new("test.event".to_string()).unwrap()
    }

    #[test]
    fn test_create_projection() {
        let projection = Projection::new(
            test_tenant_id(),
            "user_snapshot".to_string(),
            1,
            ProjectionType::EntitySnapshot,
        );

        assert!(projection.is_ok());
        let projection = projection.unwrap();
        assert_eq!(projection.name(), "user_snapshot");
        assert_eq!(projection.version(), 1);
        assert_eq!(projection.status(), ProjectionStatus::Created);
        assert_eq!(projection.projection_type(), ProjectionType::EntitySnapshot);
    }

    #[test]
    fn test_create_v1_projection() {
        let projection = Projection::new_v1(
            test_tenant_id(),
            "event_counter".to_string(),
            ProjectionType::EventCounter,
        );

        assert!(projection.is_ok());
        let projection = projection.unwrap();
        assert_eq!(projection.version(), 1);
        assert!(projection.is_first_version());
    }

    #[test]
    fn test_reject_empty_name() {
        let result = Projection::new(
            test_tenant_id(),
            "".to_string(),
            1,
            ProjectionType::Custom,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_too_long_name() {
        let long_name = "a".repeat(101);
        let result = Projection::new(
            test_tenant_id(),
            long_name,
            1,
            ProjectionType::Custom,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_invalid_name_characters() {
        let result = Projection::new(
            test_tenant_id(),
            "invalid name!".to_string(),
            1,
            ProjectionType::Custom,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_accept_valid_names() {
        let names = vec!["user_snapshot", "event-counter", "projection123"];

        for name in names {
            let result = Projection::new(
                test_tenant_id(),
                name.to_string(),
                1,
                ProjectionType::Custom,
            );
            assert!(result.is_ok(), "Name '{}' should be valid", name);
        }
    }

    #[test]
    fn test_reject_zero_version() {
        let result = Projection::new(
            test_tenant_id(),
            "test_projection".to_string(),
            0,
            ProjectionType::Custom,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_start_projection() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        assert_eq!(projection.status(), ProjectionStatus::Created);
        assert!(projection.started_at.is_none());

        let result = projection.start();
        assert!(result.is_ok());
        assert_eq!(projection.status(), ProjectionStatus::Running);
        assert!(projection.started_at.is_some());
    }

    #[test]
    fn test_cannot_start_running_projection() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        projection.start().unwrap();
        let result = projection.start();
        assert!(result.is_err());
    }

    #[test]
    fn test_pause_projection() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        projection.start().unwrap();
        let result = projection.pause();
        assert!(result.is_ok());
        assert_eq!(projection.status(), ProjectionStatus::Paused);
    }

    #[test]
    fn test_cannot_pause_non_running_projection() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        let result = projection.pause();
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_projection() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        projection.start().unwrap();
        let result = projection.stop();
        assert!(result.is_ok());
        assert_eq!(projection.status(), ProjectionStatus::Stopped);
        assert!(projection.stopped_at.is_some());
    }

    #[test]
    fn test_mark_failed() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        projection.start().unwrap();
        projection.mark_failed();
        assert_eq!(projection.status(), ProjectionStatus::Failed);
        assert!(projection.status().is_failed());
    }

    #[test]
    fn test_start_rebuild() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        // Process some events first
        projection.stats_mut().record_event_processed(10);
        assert_eq!(projection.stats().events_processed(), 1);

        let result = projection.start_rebuild();
        assert!(result.is_ok());
        assert_eq!(projection.status(), ProjectionStatus::Rebuilding);
        assert_eq!(projection.stats().events_processed(), 0); // Stats reset
    }

    #[test]
    fn test_set_description() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        let result = projection.set_description("Test projection".to_string());
        assert!(result.is_ok());
        assert_eq!(projection.description(), Some("Test projection"));
    }

    #[test]
    fn test_reject_too_long_description() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        let long_desc = "a".repeat(1001);
        let result = projection.set_description(long_desc);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_event_type() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        let event_type = test_event_type();
        let result = projection.add_event_type(event_type.clone());
        assert!(result.is_ok());
        assert_eq!(projection.event_types().len(), 1);
        assert!(projection.processes_event_type(&event_type));
    }

    #[test]
    fn test_reject_duplicate_event_type() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        let event_type = test_event_type();
        projection.add_event_type(event_type.clone()).unwrap();
        let result = projection.add_event_type(event_type);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_event_type() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        let event_type = test_event_type();
        projection.add_event_type(event_type.clone()).unwrap();

        let result = projection.remove_event_type(&event_type);
        assert!(result.is_ok());
        assert_eq!(projection.event_types().len(), 0);
    }

    #[test]
    fn test_processes_all_events_when_no_filter() {
        let projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        let event_type = test_event_type();
        assert!(projection.processes_event_type(&event_type));
    }

    #[test]
    fn test_projection_stats() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        // Record some events
        projection.stats_mut().record_event_processed(10);
        projection.stats_mut().record_event_processed(20);
        projection.stats_mut().record_event_processed(30);

        assert_eq!(projection.stats().events_processed(), 3);
        assert_eq!(projection.stats().processing_time_ms(), 60);
        assert_eq!(projection.stats().avg_processing_time_ms(), 20.0);
    }

    #[test]
    fn test_stats_record_error() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        projection.stats_mut().record_error();
        projection.stats_mut().record_error();

        assert_eq!(projection.stats().errors_count(), 2);
    }

    #[test]
    fn test_belongs_to_tenant() {
        let tenant1 = TenantId::new("tenant1".to_string()).unwrap();
        let tenant2 = TenantId::new("tenant2".to_string()).unwrap();

        let projection = Projection::new_v1(
            tenant1.clone(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        assert!(projection.belongs_to_tenant(&tenant1));
        assert!(!projection.belongs_to_tenant(&tenant2));
    }

    #[test]
    fn test_create_next_version() {
        let projection_v1 = Projection::new_v1(
            test_tenant_id(),
            "test_projection".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        let projection_v2 = projection_v1.create_next_version();
        assert!(projection_v2.is_ok());

        let projection_v2 = projection_v2.unwrap();
        assert_eq!(projection_v2.version(), 2);
        assert_eq!(projection_v2.name(), "test_projection");
        assert_eq!(projection_v2.projection_type(), ProjectionType::Custom);
        assert!(!projection_v2.is_first_version());
    }

    #[test]
    fn test_projection_status_checks() {
        assert!(ProjectionStatus::Running.is_active());
        assert!(ProjectionStatus::Rebuilding.is_active());
        assert!(!ProjectionStatus::Paused.is_active());

        assert!(ProjectionStatus::Created.can_start());
        assert!(ProjectionStatus::Stopped.can_start());
        assert!(!ProjectionStatus::Running.can_start());

        assert!(ProjectionStatus::Running.can_pause());
        assert!(!ProjectionStatus::Created.can_pause());

        assert!(ProjectionStatus::Running.can_stop());
        assert!(!ProjectionStatus::Stopped.can_stop());

        assert!(ProjectionStatus::Failed.is_failed());
        assert!(!ProjectionStatus::Running.is_failed());
    }

    #[test]
    fn test_update_config() {
        let mut projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        let mut config = ProjectionConfig::default();
        config.batch_size = 500;
        config.parallel_processing = true;

        projection.update_config(config);
        assert_eq!(projection.config().batch_size, 500);
        assert!(projection.config().parallel_processing);
    }

    #[test]
    fn test_serde_serialization() {
        let projection = Projection::new_v1(
            test_tenant_id(),
            "test".to_string(),
            ProjectionType::Custom,
        ).unwrap();

        // Should be able to serialize
        let json = serde_json::to_string(&projection);
        assert!(json.is_ok());

        // Should be able to deserialize
        let deserialized = serde_json::from_str::<Projection>(&json.unwrap());
        assert!(deserialized.is_ok());
    }
}
