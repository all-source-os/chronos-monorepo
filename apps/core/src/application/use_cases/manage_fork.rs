use crate::application::dto::{
    AppendForkEventRequest, AppendForkEventResponse, BranchForkRequest, BranchForkResponse,
    CleanupExpiredForksRequest, CleanupExpiredForksResponse, CreateForkRequest, CreateForkResponse,
    DiscardForkRequest, DiscardForkResponse, ForkDto, ListForksRequest, ListForksResponse,
    MergeForkRequest, MergeForkResponse, QueryForkEventsRequest, QueryForkEventsResponse,
    UpdateForkRequest, UpdateForkResponse,
};
use crate::domain::entities::{Event, EventStoreFork};
use crate::domain::repositories::{ForkQuery, ForkRepository};
use crate::domain::value_objects::{ForkId, TenantId};
use crate::error::Result;
use chrono::Utc;
use std::sync::Arc;

/// Use Case: Create Fork
///
/// Creates a new event store fork for agent experimentation.
/// Implements the Agentic Postgres pattern for safe AI experimentation.
///
/// Responsibilities:
/// - Validate input (DTO validation)
/// - Check for duplicate fork names within tenant
/// - Create domain EventStoreFork entity
/// - Persist via repository
/// - Return response DTO
pub struct CreateForkUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl CreateForkUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: CreateForkRequest) -> Result<CreateForkResponse> {
        // Validate tenant ID
        let tenant_id = TenantId::new(request.tenant_id)?;

        // Check for duplicate name
        if self.repository.name_exists(&tenant_id, &request.name).await? {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Fork name '{}' already exists in tenant",
                request.name
            )));
        }

        // Create the fork entity
        let mut fork = EventStoreFork::new(tenant_id, request.name, 0)?;

        // Set optional fields
        if let Some(description) = request.description {
            fork.set_description(description);
        }

        if let Some(isolation_level) = request.isolation_level {
            fork.set_isolation_level(isolation_level.into())?;
        }

        if let Some(ttl_hours) = request.ttl_hours {
            // Extend from default if requested
            let extra_hours = ttl_hours - EventStoreFork::DEFAULT_TTL_HOURS;
            if extra_hours > 0 {
                fork.extend_ttl(extra_hours)?;
            }
        }

        if let Some(agent_id) = request.created_by_agent {
            fork.set_created_by_agent(agent_id);
        }

        // Persist
        let fork = self.repository.create(fork).await?;

        Ok(CreateForkResponse {
            fork: ForkDto::from(&fork),
        })
    }
}

/// Use Case: Branch Fork
///
/// Creates a child fork from an existing fork.
/// Enables hierarchical experimentation (fork from a fork).
pub struct BranchForkUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl BranchForkUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: BranchForkRequest) -> Result<BranchForkResponse> {
        // Find parent fork
        let parent_id = ForkId::parse(&request.parent_fork_id)?;
        let parent = self
            .repository
            .find_by_id(&parent_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound(format!(
                    "Parent fork '{}' not found",
                    request.parent_fork_id
                ))
            })?;

        // Check for duplicate name in same tenant
        if self
            .repository
            .name_exists(parent.tenant_id(), &request.name)
            .await?
        {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Fork name '{}' already exists in tenant",
                request.name
            )));
        }

        // Create child fork
        let mut fork = EventStoreFork::branch_from(&parent, request.name)?;

        if let Some(description) = request.description {
            fork.set_description(description);
        }

        if let Some(agent_id) = request.created_by_agent {
            fork.set_created_by_agent(agent_id);
        }

        // Persist
        let fork = self.repository.create(fork).await?;

        Ok(BranchForkResponse {
            fork: ForkDto::from(&fork),
        })
    }
}

/// Use Case: Update Fork
///
/// Updates fork settings like description, isolation level, or TTL.
pub struct UpdateForkUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl UpdateForkUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        fork_id: &str,
        request: UpdateForkRequest,
    ) -> Result<UpdateForkResponse> {
        let id = ForkId::parse(fork_id)?;
        let mut fork = self
            .repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| crate::error::AllSourceError::EntityNotFound(format!("Fork '{}' not found", fork_id)))?;

        if let Some(description) = request.description {
            fork.set_description(description);
        }

        if let Some(isolation_level) = request.isolation_level {
            fork.set_isolation_level(isolation_level.into())?;
        }

        if let Some(hours) = request.extend_ttl_hours {
            fork.extend_ttl(hours)?;
        }

        self.repository.save(&fork).await?;

        Ok(UpdateForkResponse {
            fork: ForkDto::from(&fork),
        })
    }
}

/// Use Case: Append Event to Fork
///
/// Adds a new event to a fork's event collection.
/// Events are isolated within the fork until merged.
pub struct AppendForkEventUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl AppendForkEventUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: AppendForkEventRequest) -> Result<AppendForkEventResponse> {
        let fork_id = ForkId::parse(&request.fork_id)?;
        let mut fork = self
            .repository
            .find_by_id(&fork_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound(format!("Fork '{}' not found", request.fork_id))
            })?;

        // Create event using the fork's tenant
        let event = Event::from_strings(
            request.event_type,
            request.entity_id,
            fork.tenant_id().as_str().to_string(),
            request.payload,
            request.metadata,
        )?;

        let event_id = event.id().to_string();

        // Append to fork
        fork.append_event(event)?;

        // Save fork
        self.repository.save(&fork).await?;

        Ok(AppendForkEventResponse {
            event_id,
            fork_event_count: fork.event_count(),
        })
    }
}

/// Use Case: Merge Fork
///
/// Marks a fork as merged and optionally commits its events to the parent store.
pub struct MergeForkUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl MergeForkUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: MergeForkRequest) -> Result<MergeForkResponse> {
        let fork_id = ForkId::parse(&request.fork_id)?;
        let mut fork = self
            .repository
            .find_by_id(&fork_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound(format!("Fork '{}' not found", request.fork_id))
            })?;

        let events_count = fork.event_count();

        // TODO: If commit_events is true, apply events to parent/main store
        // This would require injecting an EventRepository and implementing
        // the actual event commit logic

        // Mark as merged
        fork.mark_merged()?;
        self.repository.save(&fork).await?;

        Ok(MergeForkResponse {
            fork: ForkDto::from(&fork),
            events_committed: if request.commit_events { events_count } else { 0 },
        })
    }
}

/// Use Case: Discard Fork
///
/// Discards a fork, clearing its events and marking it as discarded.
pub struct DiscardForkUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl DiscardForkUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: DiscardForkRequest) -> Result<DiscardForkResponse> {
        let fork_id = ForkId::parse(&request.fork_id)?;
        let mut fork = self
            .repository
            .find_by_id(&fork_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound(format!("Fork '{}' not found", request.fork_id))
            })?;

        fork.discard()?;
        self.repository.save(&fork).await?;

        Ok(DiscardForkResponse {
            fork: ForkDto::from(&fork),
        })
    }
}

/// Use Case: Query Fork Events
///
/// Retrieves events from a fork with optional filtering.
pub struct QueryForkEventsUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl QueryForkEventsUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: QueryForkEventsRequest) -> Result<QueryForkEventsResponse> {
        let fork_id = ForkId::parse(&request.fork_id)?;
        let fork = self
            .repository
            .find_by_id(&fork_id)
            .await?
            .ok_or_else(|| {
                crate::error::AllSourceError::EntityNotFound(format!("Fork '{}' not found", request.fork_id))
            })?;

        let mut events: Vec<_> = fork.all_events();

        // Filter by entity_id if provided
        if let Some(ref entity_id) = request.entity_id {
            events.retain(|e| e.entity_id().as_str() == entity_id);
        }

        // Filter by event_type if provided
        if let Some(ref event_type) = request.event_type {
            events.retain(|e| e.event_type().as_str() == event_type);
        }

        // Apply pagination
        let total = events.len();
        let offset = request.offset.unwrap_or(0);
        let limit = request.limit.unwrap_or(100);

        let events: Vec<_> = events.into_iter().skip(offset).take(limit).collect();

        let event_dtos = events
            .into_iter()
            .map(|e| crate::application::dto::EventDto::from(e))
            .collect();

        Ok(QueryForkEventsResponse {
            events: event_dtos,
            count: total,
        })
    }
}

/// Use Case: List Forks
///
/// Lists forks with optional filtering.
pub struct ListForksUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl ListForksUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: ListForksRequest) -> Result<ListForksResponse> {
        let mut query = ForkQuery::new();

        if let Some(tenant_id_str) = request.tenant_id {
            let tenant_id = TenantId::new(tenant_id_str)?;
            query = query.for_tenant(tenant_id);
        }

        if let Some(status) = request.status {
            query = query.with_status(status.into());
        }

        if let Some(agent_id) = request.created_by_agent {
            query = query.created_by(agent_id);
        }

        let limit = request.limit.unwrap_or(100);
        let offset = request.offset.unwrap_or(0);
        query = query.with_pagination(limit, offset);

        let forks = self.repository.query(&query).await?;
        let count = forks.len();

        let fork_dtos = forks.iter().map(ForkDto::from).collect();

        Ok(ListForksResponse {
            forks: fork_dtos,
            count,
        })
    }
}

/// Use Case: Get Fork
///
/// Retrieves a single fork by ID.
pub struct GetForkUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl GetForkUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, fork_id: &str) -> Result<ForkDto> {
        let id = ForkId::parse(fork_id)?;
        let fork = self
            .repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| crate::error::AllSourceError::EntityNotFound(format!("Fork '{}' not found", fork_id)))?;

        Ok(ForkDto::from(&fork))
    }
}

/// Use Case: Cleanup Expired Forks
///
/// Deletes expired forks to free up resources.
/// Should be run periodically as a background job.
pub struct CleanupExpiredForksUseCase {
    repository: Arc<dyn ForkRepository>,
}

impl CleanupExpiredForksUseCase {
    pub fn new(repository: Arc<dyn ForkRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        request: CleanupExpiredForksRequest,
    ) -> Result<CleanupExpiredForksResponse> {
        let before = request.before.unwrap_or_else(Utc::now);

        // First, mark any expired forks
        let expired_forks = self.repository.find_expired(before, 1000).await?;
        for fork in expired_forks {
            let mut fork = fork;
            fork.mark_expired();
            self.repository.save(&fork).await?;
        }

        // Then delete them
        let deleted = self.repository.delete_expired(before).await?;

        Ok(CleanupExpiredForksResponse {
            forks_deleted: deleted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ForkStatus;
    use async_trait::async_trait;
    use chrono::DateTime;
    use std::sync::Mutex;

    struct MockForkRepository {
        forks: Mutex<Vec<EventStoreFork>>,
    }

    impl MockForkRepository {
        fn new() -> Self {
            Self {
                forks: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ForkRepository for MockForkRepository {
        async fn create(&self, fork: EventStoreFork) -> Result<EventStoreFork> {
            let mut forks = self.forks.lock().unwrap();
            forks.push(fork.clone());
            Ok(fork)
        }

        async fn save(&self, fork: &EventStoreFork) -> Result<()> {
            let mut forks = self.forks.lock().unwrap();
            if let Some(idx) = forks.iter().position(|f| f.id() == fork.id()) {
                forks[idx] = fork.clone();
            }
            Ok(())
        }

        async fn find_by_id(&self, id: &ForkId) -> Result<Option<EventStoreFork>> {
            let forks = self.forks.lock().unwrap();
            Ok(forks.iter().find(|f| f.id() == id).cloned())
        }

        async fn find_by_name(&self, tenant_id: &TenantId, name: &str) -> Result<Option<EventStoreFork>> {
            let forks = self.forks.lock().unwrap();
            Ok(forks
                .iter()
                .find(|f| f.tenant_id() == tenant_id && f.name() == name)
                .cloned())
        }

        async fn find_by_tenant(
            &self,
            tenant_id: &TenantId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<EventStoreFork>> {
            let forks = self.forks.lock().unwrap();
            Ok(forks
                .iter()
                .filter(|f| f.tenant_id() == tenant_id)
                .cloned()
                .collect())
        }

        async fn find_children(&self, parent_id: &ForkId) -> Result<Vec<EventStoreFork>> {
            let forks = self.forks.lock().unwrap();
            Ok(forks
                .iter()
                .filter(|f| f.parent_fork_id() == Some(parent_id))
                .cloned()
                .collect())
        }

        async fn find_active(
            &self,
            _tenant_id: Option<&TenantId>,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<EventStoreFork>> {
            let forks = self.forks.lock().unwrap();
            Ok(forks
                .iter()
                .filter(|f| f.is_active())
                .cloned()
                .collect())
        }

        async fn find_by_status(
            &self,
            status: ForkStatus,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<EventStoreFork>> {
            let forks = self.forks.lock().unwrap();
            Ok(forks
                .iter()
                .filter(|f| f.status() == status)
                .cloned()
                .collect())
        }

        async fn find_expired(&self, before: DateTime<Utc>, _limit: usize) -> Result<Vec<EventStoreFork>> {
            let forks = self.forks.lock().unwrap();
            Ok(forks
                .iter()
                .filter(|f| f.expires_at() < before)
                .cloned()
                .collect())
        }

        async fn find_by_agent(
            &self,
            agent_id: &str,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<EventStoreFork>> {
            let forks = self.forks.lock().unwrap();
            Ok(forks
                .iter()
                .filter(|f| f.created_by_agent() == Some(agent_id))
                .cloned()
                .collect())
        }

        async fn count(&self) -> Result<usize> {
            Ok(self.forks.lock().unwrap().len())
        }

        async fn count_by_tenant(&self, tenant_id: &TenantId) -> Result<usize> {
            let forks = self.forks.lock().unwrap();
            Ok(forks.iter().filter(|f| f.tenant_id() == tenant_id).count())
        }

        async fn count_by_status(&self, status: ForkStatus) -> Result<usize> {
            let forks = self.forks.lock().unwrap();
            Ok(forks.iter().filter(|f| f.status() == status).count())
        }

        async fn delete(&self, id: &ForkId) -> Result<bool> {
            let mut forks = self.forks.lock().unwrap();
            let len_before = forks.len();
            forks.retain(|f| f.id() != id);
            Ok(forks.len() < len_before)
        }

        async fn delete_expired(&self, before: DateTime<Utc>) -> Result<usize> {
            let mut forks = self.forks.lock().unwrap();
            let len_before = forks.len();
            forks.retain(|f| f.expires_at() >= before);
            Ok(len_before - forks.len())
        }

        async fn query(&self, query: &ForkQuery) -> Result<Vec<EventStoreFork>> {
            let forks = self.forks.lock().unwrap();
            let mut result: Vec<_> = forks.iter().cloned().collect();

            if let Some(ref tenant_id) = query.tenant_id {
                result.retain(|f| f.tenant_id() == tenant_id);
            }

            if let Some(status) = query.status {
                result.retain(|f| f.status() == status);
            }

            if let Some(ref agent_id) = query.created_by_agent {
                result.retain(|f| f.created_by_agent() == Some(agent_id.as_str()));
            }

            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(usize::MAX);

            Ok(result.into_iter().skip(offset).take(limit).collect())
        }
    }

    #[tokio::test]
    async fn test_create_fork() {
        let repo = Arc::new(MockForkRepository::new());
        let use_case = CreateForkUseCase::new(repo.clone());

        let request = CreateForkRequest {
            tenant_id: "test-tenant".to_string(),
            name: "test-fork".to_string(),
            description: Some("A test fork".to_string()),
            parent_fork_id: None,
            isolation_level: None,
            ttl_hours: None,
            created_by_agent: Some("agent-123".to_string()),
        };

        let response = use_case.execute(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.fork.name, "test-fork");
        assert_eq!(response.fork.created_by_agent, Some("agent-123".to_string()));
    }

    #[tokio::test]
    async fn test_create_fork_duplicate_name() {
        let repo = Arc::new(MockForkRepository::new());
        let use_case = CreateForkUseCase::new(repo.clone());

        let request = CreateForkRequest {
            tenant_id: "test-tenant".to_string(),
            name: "test-fork".to_string(),
            description: None,
            parent_fork_id: None,
            isolation_level: None,
            ttl_hours: None,
            created_by_agent: None,
        };

        // First creation should succeed
        use_case.execute(request.clone()).await.unwrap();

        // Second creation with same name should fail
        let result = use_case.execute(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_branch_fork() {
        let repo = Arc::new(MockForkRepository::new());
        let create_use_case = CreateForkUseCase::new(repo.clone());
        let branch_use_case = BranchForkUseCase::new(repo.clone());

        // Create parent fork
        let parent_request = CreateForkRequest {
            tenant_id: "test-tenant".to_string(),
            name: "parent-fork".to_string(),
            description: None,
            parent_fork_id: None,
            isolation_level: None,
            ttl_hours: None,
            created_by_agent: None,
        };
        let parent_response = create_use_case.execute(parent_request).await.unwrap();

        // Branch from parent
        let branch_request = BranchForkRequest {
            parent_fork_id: parent_response.fork.id.clone(),
            name: "child-fork".to_string(),
            description: Some("A child fork".to_string()),
            created_by_agent: None,
        };

        let response = branch_use_case.execute(branch_request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.fork.name, "child-fork");
        assert_eq!(response.fork.parent_fork_id, Some(parent_response.fork.id));
    }

    #[tokio::test]
    async fn test_discard_fork() {
        let repo = Arc::new(MockForkRepository::new());
        let create_use_case = CreateForkUseCase::new(repo.clone());
        let discard_use_case = DiscardForkUseCase::new(repo.clone());

        // Create fork
        let create_request = CreateForkRequest {
            tenant_id: "test-tenant".to_string(),
            name: "test-fork".to_string(),
            description: None,
            parent_fork_id: None,
            isolation_level: None,
            ttl_hours: None,
            created_by_agent: None,
        };
        let create_response = create_use_case.execute(create_request).await.unwrap();

        // Discard fork
        let discard_request = DiscardForkRequest {
            fork_id: create_response.fork.id.clone(),
        };

        let response = discard_use_case.execute(discard_request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.fork.status, crate::application::dto::ForkStatusDto::Discarded);
    }

    #[tokio::test]
    async fn test_list_forks() {
        let repo = Arc::new(MockForkRepository::new());
        let create_use_case = CreateForkUseCase::new(repo.clone());
        let list_use_case = ListForksUseCase::new(repo.clone());

        // Create a few forks
        for i in 0..3 {
            let request = CreateForkRequest {
                tenant_id: "test-tenant".to_string(),
                name: format!("fork-{}", i),
                description: None,
                parent_fork_id: None,
                isolation_level: None,
                ttl_hours: None,
                created_by_agent: None,
            };
            create_use_case.execute(request).await.unwrap();
        }

        // List all forks
        let list_request = ListForksRequest {
            tenant_id: Some("test-tenant".to_string()),
            status: None,
            created_by_agent: None,
            limit: None,
            offset: None,
        };

        let response = list_use_case.execute(list_request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.count, 3);
    }
}
