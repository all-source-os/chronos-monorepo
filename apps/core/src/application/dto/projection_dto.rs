use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::domain::entities::{Projection, ProjectionType, ProjectionStatus, ProjectionConfig, ProjectionStats};

/// DTO for creating a new projection
#[derive(Debug, Deserialize)]
pub struct CreateProjectionRequest {
    pub name: String,
    pub projection_type: ProjectionTypeDto,
    pub tenant_id: String,
    pub event_types: Vec<String>,
    pub description: Option<String>,
    pub config: Option<ProjectionConfigDto>,
}

/// DTO for updating a projection
#[derive(Debug, Deserialize)]
pub struct UpdateProjectionRequest {
    pub description: Option<String>,
    pub config: Option<ProjectionConfigDto>,
    pub event_types: Option<Vec<String>>,
}

/// DTO for projection type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionTypeDto {
    EntitySnapshot,
    EventCounter,
    Custom,
    TimeSeries,
    Funnel,
}

impl From<ProjectionType> for ProjectionTypeDto {
    fn from(ptype: ProjectionType) -> Self {
        match ptype {
            ProjectionType::EntitySnapshot => ProjectionTypeDto::EntitySnapshot,
            ProjectionType::EventCounter => ProjectionTypeDto::EventCounter,
            ProjectionType::Custom => ProjectionTypeDto::Custom,
            ProjectionType::TimeSeries => ProjectionTypeDto::TimeSeries,
            ProjectionType::Funnel => ProjectionTypeDto::Funnel,
        }
    }
}

impl From<ProjectionTypeDto> for ProjectionType {
    fn from(dto: ProjectionTypeDto) -> Self {
        match dto {
            ProjectionTypeDto::EntitySnapshot => ProjectionType::EntitySnapshot,
            ProjectionTypeDto::EventCounter => ProjectionType::EventCounter,
            ProjectionTypeDto::Custom => ProjectionType::Custom,
            ProjectionTypeDto::TimeSeries => ProjectionType::TimeSeries,
            ProjectionTypeDto::Funnel => ProjectionType::Funnel,
        }
    }
}

/// DTO for projection status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionStatusDto {
    Created,
    Running,
    Paused,
    Failed,
    Stopped,
    Rebuilding,
}

impl From<ProjectionStatus> for ProjectionStatusDto {
    fn from(status: ProjectionStatus) -> Self {
        match status {
            ProjectionStatus::Created => ProjectionStatusDto::Created,
            ProjectionStatus::Running => ProjectionStatusDto::Running,
            ProjectionStatus::Paused => ProjectionStatusDto::Paused,
            ProjectionStatus::Failed => ProjectionStatusDto::Failed,
            ProjectionStatus::Stopped => ProjectionStatusDto::Stopped,
            ProjectionStatus::Rebuilding => ProjectionStatusDto::Rebuilding,
        }
    }
}

/// DTO for projection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionConfigDto {
    pub batch_size: Option<usize>,
    pub checkpoint_interval: Option<usize>,
}

impl From<ProjectionConfig> for ProjectionConfigDto {
    fn from(config: ProjectionConfig) -> Self {
        Self {
            batch_size: Some(config.batch_size),
            checkpoint_interval: Some(config.checkpoint_interval),
        }
    }
}

impl From<ProjectionConfigDto> for ProjectionConfig {
    fn from(dto: ProjectionConfigDto) -> Self {
        Self {
            batch_size: dto.batch_size.unwrap_or(100),
            enable_checkpoints: true,
            checkpoint_interval: dto.checkpoint_interval.unwrap_or(1000),
            parallel_processing: false,
            max_concurrency: 4,
        }
    }
}

/// DTO for projection statistics
#[derive(Debug, Clone, Serialize)]
pub struct ProjectionStatsDto {
    pub events_processed: u64,
    pub last_processed_at: Option<DateTime<Utc>>,
    pub errors_count: u64,
    pub last_error: Option<String>,
    pub avg_processing_time_ms: Option<f64>,
}

impl From<&ProjectionStats> for ProjectionStatsDto {
    fn from(stats: &ProjectionStats) -> Self {
        Self {
            events_processed: stats.events_processed(),
            last_processed_at: stats.last_processed_at(),
            errors_count: stats.errors_count(),
            last_error: None, // Not available in domain entity
            avg_processing_time_ms: Some(stats.avg_processing_time_ms()),
        }
    }
}

/// DTO for projection response
#[derive(Debug, Serialize)]
pub struct ProjectionDto {
    pub id: Uuid,
    pub name: String,
    pub projection_type: ProjectionTypeDto,
    pub tenant_id: String,
    pub status: ProjectionStatusDto,
    pub version: u32,
    pub event_types: Vec<String>,
    pub description: Option<String>,
    pub config: ProjectionConfigDto,
    pub stats: ProjectionStatsDto,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Projection> for ProjectionDto {
    fn from(projection: &Projection) -> Self {
        Self {
            id: projection.id(),
            name: projection.name().to_string(),
            projection_type: projection.projection_type().into(),
            tenant_id: projection.tenant_id().to_string(),
            status: projection.status().into(),
            version: projection.version(),
            event_types: projection.event_types().iter().map(|s| s.to_string()).collect(),
            description: projection.description().map(String::from),
            config: projection.config().clone().into(),
            stats: projection.stats().into(),
            created_at: projection.created_at(),
            updated_at: projection.updated_at(),
        }
    }
}

impl From<Projection> for ProjectionDto {
    fn from(projection: Projection) -> Self {
        ProjectionDto::from(&projection)
    }
}

/// Response for projection creation
#[derive(Debug, Serialize)]
pub struct CreateProjectionResponse {
    pub projection: ProjectionDto,
}

/// Response for listing projections
#[derive(Debug, Serialize)]
pub struct ListProjectionsResponse {
    pub projections: Vec<ProjectionDto>,
    pub count: usize,
}

/// Request to start a projection
#[derive(Debug, Deserialize)]
pub struct StartProjectionRequest {
    pub projection_id: Uuid,
}

/// Request to pause a projection
#[derive(Debug, Deserialize)]
pub struct PauseProjectionRequest {
    pub projection_id: Uuid,
}

/// Request to rebuild a projection
#[derive(Debug, Deserialize)]
pub struct RebuildProjectionRequest {
    pub projection_id: Uuid,
}
