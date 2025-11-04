use std::sync::Arc;
use crate::domain::entities::Event;
use crate::domain::repositories::EventRepository;
use crate::application::dto::{IngestEventRequest, IngestEventResponse};
use crate::error::Result;

/// Use Case: Ingest Event
///
/// This use case handles the ingestion of a single event into the event store.
/// It coordinates between the domain layer (Event entity) and the repository.
///
/// Responsibilities:
/// - Validate input (DTO validation)
/// - Create domain Event entity (with domain validation)
/// - Persist via repository
/// - Return response DTO
pub struct IngestEventUseCase {
    repository: Arc<dyn EventRepository>,
}

impl IngestEventUseCase {
    pub fn new(repository: Arc<dyn EventRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, request: IngestEventRequest) -> Result<IngestEventResponse> {
        // Create domain event using from_strings (validates and converts to value objects)
        let tenant_id = request.tenant_id.unwrap_or_else(|| "default".to_string());

        let event = Event::from_strings(
            request.event_type,
            request.entity_id,
            tenant_id,
            request.payload,
            request.metadata,
        )?;

        // Persist via repository
        self.repository.save(&event).await?;

        // Return response
        Ok(IngestEventResponse::from_event(&event))
    }
}

/// Use Case: Batch Ingest Events
///
/// Optimized use case for ingesting multiple events at once.
pub struct IngestEventsBatchUseCase {
    repository: Arc<dyn EventRepository>,
}

impl IngestEventsBatchUseCase {
    pub fn new(repository: Arc<dyn EventRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        requests: Vec<IngestEventRequest>,
    ) -> Result<Vec<IngestEventResponse>> {
        // Create all domain events (validates and converts to value objects)
        let mut events = Vec::with_capacity(requests.len());
        let mut responses = Vec::with_capacity(requests.len());

        for request in requests {
            let tenant_id = request.tenant_id.unwrap_or_else(|| "default".to_string());

            let event = Event::from_strings(
                request.event_type,
                request.entity_id,
                tenant_id,
                request.payload,
                request.metadata,
            )?;

            responses.push(IngestEventResponse::from_event(&event));
            events.push(event);
        }

        // Batch persist
        self.repository.save_batch(&events).await?;

        Ok(responses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use uuid::Uuid;
    use chrono::Utc;

    // Mock repository for testing
    struct MockEventRepository {
        events: std::sync::Mutex<Vec<Event>>,
    }

    impl MockEventRepository {
        fn new() -> Self {
            Self {
                events: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl EventRepository for MockEventRepository {
        async fn save(&self, event: &Event) -> Result<()> {
            let mut events = self.events.lock().unwrap();
            events.push(Event::reconstruct_from_strings(
                event.id(),
                event.event_type_str().to_string(),
                event.entity_id_str().to_string(),
                event.tenant_id_str().to_string(),
                event.payload().clone(),
                event.timestamp(),
                event.metadata().cloned(),
                event.version(),
            ));
            Ok(())
        }

        async fn save_batch(&self, events: &[Event]) -> Result<()> {
            for event in events {
                self.save(event).await?;
            }
            Ok(())
        }

        async fn find_by_id(&self, _id: Uuid) -> Result<Option<Event>> {
            unimplemented!()
        }

        async fn find_by_entity(&self, _entity_id: &str, _tenant_id: &str) -> Result<Vec<Event>> {
            unimplemented!()
        }

        async fn find_by_type(&self, _event_type: &str, _tenant_id: &str) -> Result<Vec<Event>> {
            unimplemented!()
        }

        async fn find_by_time_range(
            &self,
            _tenant_id: &str,
            _start: chrono::DateTime<Utc>,
            _end: chrono::DateTime<Utc>,
        ) -> Result<Vec<Event>> {
            unimplemented!()
        }

        async fn find_by_entity_as_of(
            &self,
            _entity_id: &str,
            _tenant_id: &str,
            _as_of: chrono::DateTime<Utc>,
        ) -> Result<Vec<Event>> {
            unimplemented!()
        }

        async fn count(&self, _tenant_id: &str) -> Result<usize> {
            unimplemented!()
        }

        async fn health_check(&self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_ingest_event_use_case() {
        let repo = Arc::new(MockEventRepository::new());
        let use_case = IngestEventUseCase::new(repo.clone());

        let request = IngestEventRequest {
            event_type: "user.created".to_string(),
            entity_id: "user-123".to_string(),
            tenant_id: Some("tenant-1".to_string()),
            payload: json!({"name": "Alice"}),
            metadata: None,
        };

        let response = use_case.execute(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(repo.events.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_ingest_event_with_default_tenant() {
        let repo = Arc::new(MockEventRepository::new());
        let use_case = IngestEventUseCase::new(repo.clone());

        let request = IngestEventRequest {
            event_type: "order.placed".to_string(),
            entity_id: "order-456".to_string(),
            tenant_id: None, // Should default to "default"
            payload: json!({"amount": 100}),
            metadata: None,
        };

        let response = use_case.execute(request).await;
        assert!(response.is_ok());

        let events = repo.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].tenant_id_str(), "default");
    }

    #[tokio::test]
    async fn test_batch_ingest() {
        let repo = Arc::new(MockEventRepository::new());
        let use_case = IngestEventsBatchUseCase::new(repo.clone());

        let requests = vec![
            IngestEventRequest {
                event_type: "event.1".to_string(),
                entity_id: "e1".to_string(),
                tenant_id: Some("t1".to_string()),
                payload: json!({}),
                metadata: None,
            },
            IngestEventRequest {
                event_type: "event.2".to_string(),
                entity_id: "e2".to_string(),
                tenant_id: Some("t1".to_string()),
                payload: json!({}),
                metadata: None,
            },
        ];

        let responses = use_case.execute(requests).await;
        assert!(responses.is_ok());
        assert_eq!(responses.unwrap().len(), 2);
        assert_eq!(repo.events.lock().unwrap().len(), 2);
    }
}
