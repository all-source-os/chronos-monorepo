use crate::application::dto::{
    CreateProjectionRequest, CreateProjectionResponse, UpdateProjectionRequest,
    ListProjectionsResponse, ProjectionDto, ProjectionTypeDto, ProjectionConfigDto,
};
use crate::domain::entities::{Projection, ProjectionType, ProjectionConfig};
use crate::domain::value_objects::{TenantId, EventType};
use crate::error::Result;

/// Use Case: Create Projection
///
/// Creates a new projection for processing events.
pub struct CreateProjectionUseCase;

impl CreateProjectionUseCase {
    pub fn execute(request: CreateProjectionRequest) -> Result<CreateProjectionResponse> {
        // Validate tenant ID
        let tenant_id = TenantId::new(request.tenant_id)?;

        // Convert projection type
        let projection_type = ProjectionType::from(request.projection_type);

        // Create projection
        let mut projection = Projection::new_v1(
            tenant_id,
            request.name,
            projection_type,
        )?;

        // Set config if provided
        if let Some(config_dto) = request.config {
            let config = ProjectionConfig::from(config_dto);
            projection.update_config(config);
        }

        // Set description if provided
        if let Some(description) = request.description {
            projection.set_description(description)?;
        }

        // Add event types
        for event_type_str in request.event_types {
            let event_type = EventType::new(event_type_str)?;
            projection.add_event_type(event_type)?;
        }

        Ok(CreateProjectionResponse {
            projection: ProjectionDto::from(&projection),
        })
    }
}

/// Use Case: Update Projection
///
/// Updates projection configuration and metadata.
pub struct UpdateProjectionUseCase;

impl UpdateProjectionUseCase {
    pub fn execute(mut projection: Projection, request: UpdateProjectionRequest) -> Result<ProjectionDto> {
        // Update description if provided
        if let Some(description) = request.description {
            projection.set_description(description)?;
        }

        // Update config if provided
        if let Some(config_dto) = request.config {
            projection.update_config(ProjectionConfig::from(config_dto));
        }

        // Update event types if provided
        if let Some(event_types) = request.event_types {
            // Clear existing and add new ones
            let existing = projection.event_types().to_vec();
            for event_type in existing {
                projection.remove_event_type(&event_type);
            }
            for event_type_str in event_types {
                let event_type = EventType::new(event_type_str)?;
                projection.add_event_type(event_type)?;
            }
        }

        Ok(ProjectionDto::from(&projection))
    }
}

/// Use Case: Start Projection
///
/// Starts a created or paused projection.
pub struct StartProjectionUseCase;

impl StartProjectionUseCase {
    pub fn execute(mut projection: Projection) -> Result<ProjectionDto> {
        projection.start()?;
        Ok(ProjectionDto::from(&projection))
    }
}

/// Use Case: Pause Projection
///
/// Pauses a running projection.
pub struct PauseProjectionUseCase;

impl PauseProjectionUseCase {
    pub fn execute(mut projection: Projection) -> Result<ProjectionDto> {
        projection.pause()?;
        Ok(ProjectionDto::from(&projection))
    }
}

/// Use Case: Stop Projection
///
/// Stops a projection completely.
pub struct StopProjectionUseCase;

impl StopProjectionUseCase {
    pub fn execute(mut projection: Projection) -> Result<ProjectionDto> {
        projection.stop();
        Ok(ProjectionDto::from(&projection))
    }
}

/// Use Case: Rebuild Projection
///
/// Starts rebuilding a projection from scratch.
pub struct RebuildProjectionUseCase;

impl RebuildProjectionUseCase {
    pub fn execute(mut projection: Projection) -> Result<ProjectionDto> {
        projection.start_rebuild();
        Ok(ProjectionDto::from(&projection))
    }
}

/// Use Case: List Projections
///
/// Returns a list of all projections for a tenant.
pub struct ListProjectionsUseCase;

impl ListProjectionsUseCase {
    pub fn execute(projections: Vec<Projection>) -> ListProjectionsResponse {
        let projection_dtos: Vec<ProjectionDto> = projections.iter().map(ProjectionDto::from).collect();
        let count = projection_dtos.len();

        ListProjectionsResponse {
            projections: projection_dtos,
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_projection() {
        let request = CreateProjectionRequest {
            name: "user-snapshots".to_string(),
            projection_type: ProjectionTypeDto::EntitySnapshot,
            tenant_id: "tenant-1".to_string(),
            event_types: vec!["user.created".to_string(), "user.updated".to_string()],
            description: Some("User state snapshots".to_string()),
            config: Some(ProjectionConfigDto {
                batch_size: Some(500),
                checkpoint_interval: Some(5000),
            }),
        };

        let response = CreateProjectionUseCase::execute(request);
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.projection.name, "user-snapshots");
        assert_eq!(response.projection.event_types.len(), 2);
    }

    #[test]
    fn test_projection_lifecycle() {
        let tenant_id = TenantId::new("tenant-1".to_string()).unwrap();
        let mut projection = Projection::new_v1(
            tenant_id,
            "test-projection".to_string(),
            ProjectionType::EventCounter,
        )
        .unwrap();

        projection.add_event_type(crate::domain::value_objects::EventType::new("test.event".to_string()).unwrap()).unwrap();

        // Start
        let result = StartProjectionUseCase::execute(projection.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, crate::application::dto::ProjectionStatusDto::Running);

        // Pause
        projection.start().unwrap();
        let result = PauseProjectionUseCase::execute(projection.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, crate::application::dto::ProjectionStatusDto::Paused);

        // Rebuild
        let result = RebuildProjectionUseCase::execute(projection.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, crate::application::dto::ProjectionStatusDto::Rebuilding);
    }

    #[test]
    fn test_update_projection() {
        let tenant_id = TenantId::new("tenant-1".to_string()).unwrap();
        let mut projection = Projection::new_v1(
            tenant_id,
            "test-projection".to_string(),
            ProjectionType::Custom,
        )
        .unwrap();

        projection.add_event_type(crate::domain::value_objects::EventType::new("old.event".to_string()).unwrap()).unwrap();

        let request = UpdateProjectionRequest {
            description: Some("Updated description".to_string()),
            config: Some(ProjectionConfigDto {
                batch_size: Some(1000),
                checkpoint_interval: Some(10000),
            }),
            event_types: Some(vec!["new.event".to_string()]),
        };

        let result = UpdateProjectionUseCase::execute(projection, request);
        assert!(result.is_ok());

        let updated = result.unwrap();
        assert_eq!(updated.description, Some("Updated description".to_string()));
        assert_eq!(updated.event_types, vec!["new.event".to_string()]);
        assert_eq!(updated.config.batch_size, Some(1000));
    }

    #[test]
    fn test_list_projections() {
        let tenant_id = TenantId::new("tenant-1".to_string()).unwrap();
        let projections = vec![
            Projection::new_v1(
                tenant_id.clone(),
                "projection-1".to_string(),
                ProjectionType::EntitySnapshot,
            )
            .unwrap(),
            Projection::new_v1(
                tenant_id,
                "projection-2".to_string(),
                ProjectionType::EventCounter,
            )
            .unwrap(),
        ];

        let response = ListProjectionsUseCase::execute(projections);
        assert_eq!(response.count, 2);
        assert_eq!(response.projections.len(), 2);
    }
}
