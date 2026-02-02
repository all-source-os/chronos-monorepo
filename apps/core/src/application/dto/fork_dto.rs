use crate::domain::entities::{EventStoreFork, ForkIsolationLevel, ForkStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// DTO: Fork Status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForkStatusDto {
    Active,
    Merged,
    Discarded,
    Expired,
}

impl From<ForkStatus> for ForkStatusDto {
    fn from(status: ForkStatus) -> Self {
        match status {
            ForkStatus::Active => ForkStatusDto::Active,
            ForkStatus::Merged => ForkStatusDto::Merged,
            ForkStatus::Discarded => ForkStatusDto::Discarded,
            ForkStatus::Expired => ForkStatusDto::Expired,
        }
    }
}

impl From<ForkStatusDto> for ForkStatus {
    fn from(dto: ForkStatusDto) -> Self {
        match dto {
            ForkStatusDto::Active => ForkStatus::Active,
            ForkStatusDto::Merged => ForkStatus::Merged,
            ForkStatusDto::Discarded => ForkStatus::Discarded,
            ForkStatusDto::Expired => ForkStatus::Expired,
        }
    }
}

/// DTO: Fork Isolation Level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForkIsolationLevelDto {
    ReadParent,
    Complete,
    LiveParent,
}

impl From<ForkIsolationLevel> for ForkIsolationLevelDto {
    fn from(level: ForkIsolationLevel) -> Self {
        match level {
            ForkIsolationLevel::ReadParent => ForkIsolationLevelDto::ReadParent,
            ForkIsolationLevel::Complete => ForkIsolationLevelDto::Complete,
            ForkIsolationLevel::LiveParent => ForkIsolationLevelDto::LiveParent,
        }
    }
}

impl From<ForkIsolationLevelDto> for ForkIsolationLevel {
    fn from(dto: ForkIsolationLevelDto) -> Self {
        match dto {
            ForkIsolationLevelDto::ReadParent => ForkIsolationLevel::ReadParent,
            ForkIsolationLevelDto::Complete => ForkIsolationLevel::Complete,
            ForkIsolationLevelDto::LiveParent => ForkIsolationLevel::LiveParent,
        }
    }
}

/// DTO: Event Store Fork
///
/// Data transfer object for EventStoreFork entity.
/// Used for API requests/responses and serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkDto {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_fork_id: Option<String>,
    pub parent_version: u64,
    pub status: ForkStatusDto,
    pub isolation_level: ForkIsolationLevelDto,
    pub event_count: u64,
    pub expires_at: DateTime<Utc>,
    pub created_by_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&EventStoreFork> for ForkDto {
    fn from(fork: &EventStoreFork) -> Self {
        Self {
            id: fork.id().as_str(),
            tenant_id: fork.tenant_id().as_str().to_string(),
            name: fork.name().to_string(),
            description: fork.description().map(|s| s.to_string()),
            parent_fork_id: fork.parent_fork_id().map(|id| id.as_str()),
            parent_version: fork.parent_version(),
            status: fork.status().into(),
            isolation_level: fork.isolation_level().into(),
            event_count: fork.event_count(),
            expires_at: fork.expires_at(),
            created_by_agent: fork.created_by_agent().map(|s| s.to_string()),
            created_at: fork.created_at(),
            updated_at: fork.updated_at(),
        }
    }
}

/// Request: Create Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateForkRequest {
    pub tenant_id: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_fork_id: Option<String>,
    pub isolation_level: Option<ForkIsolationLevelDto>,
    pub ttl_hours: Option<i64>,
    pub created_by_agent: Option<String>,
}

/// Response: Create Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateForkResponse {
    pub fork: ForkDto,
}

/// Request: Branch Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchForkRequest {
    pub parent_fork_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by_agent: Option<String>,
}

/// Response: Branch Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchForkResponse {
    pub fork: ForkDto,
}

/// Request: Update Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateForkRequest {
    pub description: Option<String>,
    pub isolation_level: Option<ForkIsolationLevelDto>,
    pub extend_ttl_hours: Option<i64>,
}

/// Response: Update Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateForkResponse {
    pub fork: ForkDto,
}

/// Request: Append Event to Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendForkEventRequest {
    pub fork_id: String,
    pub event_type: String,
    pub entity_id: String,
    pub payload: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

/// Response: Append Event to Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendForkEventResponse {
    pub event_id: String,
    pub fork_event_count: u64,
}

/// Request: Merge Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeForkRequest {
    pub fork_id: String,
    /// If true, apply events to the parent/main store
    pub commit_events: bool,
}

/// Response: Merge Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeForkResponse {
    pub fork: ForkDto,
    pub events_committed: u64,
}

/// Request: Discard Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscardForkRequest {
    pub fork_id: String,
}

/// Response: Discard Fork
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscardForkResponse {
    pub fork: ForkDto,
}

/// Request: Query Fork Events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryForkEventsRequest {
    pub fork_id: String,
    pub entity_id: Option<String>,
    pub event_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Response: Query Fork Events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryForkEventsResponse {
    pub events: Vec<crate::application::dto::EventDto>,
    pub count: usize,
}

/// Request: List Forks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListForksRequest {
    pub tenant_id: Option<String>,
    pub status: Option<ForkStatusDto>,
    pub created_by_agent: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Response: List Forks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListForksResponse {
    pub forks: Vec<ForkDto>,
    pub count: usize,
}

/// Request: Cleanup Expired Forks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupExpiredForksRequest {
    pub tenant_id: Option<String>,
    pub before: Option<DateTime<Utc>>,
}

/// Response: Cleanup Expired Forks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupExpiredForksResponse {
    pub forks_deleted: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fork_status_conversion() {
        assert_eq!(ForkStatusDto::from(ForkStatus::Active), ForkStatusDto::Active);
        assert_eq!(ForkStatusDto::from(ForkStatus::Merged), ForkStatusDto::Merged);
        assert_eq!(ForkStatusDto::from(ForkStatus::Discarded), ForkStatusDto::Discarded);
        assert_eq!(ForkStatusDto::from(ForkStatus::Expired), ForkStatusDto::Expired);
    }

    #[test]
    fn test_fork_isolation_level_conversion() {
        assert_eq!(
            ForkIsolationLevelDto::from(ForkIsolationLevel::ReadParent),
            ForkIsolationLevelDto::ReadParent
        );
        assert_eq!(
            ForkIsolationLevelDto::from(ForkIsolationLevel::Complete),
            ForkIsolationLevelDto::Complete
        );
        assert_eq!(
            ForkIsolationLevelDto::from(ForkIsolationLevel::LiveParent),
            ForkIsolationLevelDto::LiveParent
        );
    }

    #[test]
    fn test_create_fork_request_serde() {
        let request = CreateForkRequest {
            tenant_id: "test-tenant".to_string(),
            name: "test-fork".to_string(),
            description: Some("A test fork".to_string()),
            parent_fork_id: None,
            isolation_level: Some(ForkIsolationLevelDto::ReadParent),
            ttl_hours: Some(48),
            created_by_agent: Some("agent-123".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CreateForkRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "test-fork");
        assert_eq!(deserialized.ttl_hours, Some(48));
    }

    #[test]
    fn test_list_forks_request_serde() {
        let request = ListForksRequest {
            tenant_id: Some("test-tenant".to_string()),
            status: Some(ForkStatusDto::Active),
            created_by_agent: None,
            limit: Some(10),
            offset: Some(0),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("active"));
    }
}
