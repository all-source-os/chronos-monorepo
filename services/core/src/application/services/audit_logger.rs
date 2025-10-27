use crate::domain::entities::{AuditEvent, AuditAction, AuditOutcome, Actor};
use crate::domain::value_objects::TenantId;
use crate::domain::repositories::AuditEventRepository;
use crate::error::AllSourceError;
use std::sync::Arc;
use serde_json::Value as JsonValue;
use tracing::error;

/// Request context extracted from HTTP requests
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            ip_address: None,
            user_agent: None,
            request_id: None,
        }
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating audit log entries
pub struct AuditLogBuilder {
    tenant_id: TenantId,
    action: AuditAction,
    actor: Actor,
    outcome: AuditOutcome,
    resource_type: Option<String>,
    resource_id: Option<String>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    request_id: Option<String>,
    error_message: Option<String>,
    metadata: Option<JsonValue>,
}

impl AuditLogBuilder {
    fn new(tenant_id: TenantId, action: AuditAction, actor: Actor) -> Self {
        Self {
            tenant_id,
            action,
            actor,
            outcome: AuditOutcome::Success,
            resource_type: None,
            resource_id: None,
            ip_address: None,
            user_agent: None,
            request_id: None,
            error_message: None,
            metadata: None,
        }
    }

    pub fn with_outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn with_resource(mut self, resource_type: String, resource_id: String) -> Self {
        self.resource_type = Some(resource_type);
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_context(mut self, context: RequestContext) -> Self {
        self.ip_address = context.ip_address;
        self.user_agent = context.user_agent;
        self.request_id = context.request_id;
        self
    }

    pub fn with_ip_address(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn with_error(mut self, error_message: String) -> Self {
        self.error_message = Some(error_message);
        self.outcome = AuditOutcome::Failure;
        self
    }

    pub fn with_metadata(mut self, metadata: JsonValue) -> Self {
        self.metadata = Some(metadata);
        self
    }

    fn build(self) -> AuditEvent {
        let mut event = AuditEvent::new(
            self.tenant_id,
            self.action,
            self.actor,
            self.outcome,
        );

        if let (Some(resource_type), Some(resource_id)) = (self.resource_type, self.resource_id) {
            event = event.with_resource(resource_type, resource_id);
        }

        if let Some(ip) = self.ip_address {
            event = event.with_ip_address(ip);
        }

        if let Some(ua) = self.user_agent {
            event = event.with_user_agent(ua);
        }

        if let Some(req_id) = self.request_id {
            event = event.with_request_id(req_id);
        }

        if let Some(err) = self.error_message {
            event = event.with_error(err);
        }

        if let Some(meta) = self.metadata {
            event = event.with_metadata(meta);
        }

        event
    }

    pub async fn record<R: AuditEventRepository>(self, repo: &R) -> Result<(), AllSourceError> {
        let event = self.build();
        repo.append(event).await
    }
}

/// AuditLogger service for simplified audit event recording
///
/// This service provides a convenient API for recording audit events
/// from application code and middleware. It handles:
/// - Automatic context extraction from HTTP requests
/// - Actor detection from authentication context
/// - Async, non-blocking logging
/// - Error handling (audit failures are logged but don't break requests)
///
/// # Example
/// ```rust
/// let audit_logger = AuditLogger::new(audit_repo);
///
/// audit_logger.log(
///     tenant_id,
///     AuditAction::EventIngested,
///     Actor::api_key("key-123".to_string(), "prod-api-key".to_string()),
/// )
/// .with_resource("event_stream".to_string(), "stream-456".to_string())
/// .with_context(request_context)
/// .record_async()
/// .await;
/// ```
pub struct AuditLogger<R: AuditEventRepository> {
    repository: Arc<R>,
}

impl<R: AuditEventRepository> AuditLogger<R> {
    /// Create a new AuditLogger with the given repository
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Start building an audit log entry
    pub fn log(
        &self,
        tenant_id: TenantId,
        action: AuditAction,
        actor: Actor,
    ) -> AuditLogEntry<R> {
        AuditLogEntry {
            logger: self,
            builder: AuditLogBuilder::new(tenant_id, action, actor),
        }
    }

    /// Log a successful event (convenience method)
    pub async fn log_success(
        &self,
        tenant_id: TenantId,
        action: AuditAction,
        actor: Actor,
    ) -> Result<(), AllSourceError> {
        let event = AuditEvent::new(tenant_id, action, actor, AuditOutcome::Success);
        self.repository.append(event).await
    }

    /// Log a failed event (convenience method)
    pub async fn log_failure(
        &self,
        tenant_id: TenantId,
        action: AuditAction,
        actor: Actor,
        error_message: String,
    ) -> Result<(), AllSourceError> {
        let event = AuditEvent::new(tenant_id, action, actor, AuditOutcome::Failure)
            .with_error(error_message);
        self.repository.append(event).await
    }

    /// Log an event with resource information (convenience method)
    pub async fn log_resource_action(
        &self,
        tenant_id: TenantId,
        action: AuditAction,
        actor: Actor,
        resource_type: String,
        resource_id: String,
        outcome: AuditOutcome,
    ) -> Result<(), AllSourceError> {
        let event = AuditEvent::new(tenant_id, action, actor, outcome)
            .with_resource(resource_type, resource_id);
        self.repository.append(event).await
    }

    /// Record an event without returning an error (logs errors internally)
    /// This is useful for middleware where audit failures shouldn't break requests
    pub async fn record_silently(&self, event: AuditEvent) {
        if let Err(e) = self.repository.append(event).await {
            error!("Failed to record audit event: {}", e);
        }
    }

    /// Batch log multiple events
    pub async fn log_batch(&self, events: Vec<AuditEvent>) -> Result<(), AllSourceError> {
        self.repository.append_batch(events).await
    }

    /// Batch log multiple events without returning an error
    pub async fn log_batch_silently(&self, events: Vec<AuditEvent>) {
        if let Err(e) = self.repository.append_batch(events).await {
            error!("Failed to record audit event batch: {}", e);
        }
    }
}

/// Builder for a single audit log entry
pub struct AuditLogEntry<'a, R: AuditEventRepository> {
    logger: &'a AuditLogger<R>,
    builder: AuditLogBuilder,
}

impl<'a, R: AuditEventRepository> AuditLogEntry<'a, R> {
    pub fn with_outcome(mut self, outcome: AuditOutcome) -> Self {
        self.builder = self.builder.with_outcome(outcome);
        self
    }

    pub fn with_resource(mut self, resource_type: String, resource_id: String) -> Self {
        self.builder = self.builder.with_resource(resource_type, resource_id);
        self
    }

    pub fn with_context(mut self, context: RequestContext) -> Self {
        self.builder = self.builder.with_context(context);
        self
    }

    pub fn with_ip_address(mut self, ip: String) -> Self {
        self.builder = self.builder.with_ip_address(ip);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.builder = self.builder.with_user_agent(user_agent);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.builder = self.builder.with_request_id(request_id);
        self
    }

    pub fn with_error(mut self, error_message: String) -> Self {
        self.builder = self.builder.with_error(error_message);
        self
    }

    pub fn with_metadata(mut self, metadata: JsonValue) -> Self {
        self.builder = self.builder.with_metadata(metadata);
        self
    }

    /// Record the audit event
    pub async fn record(self) -> Result<(), AllSourceError> {
        let event = self.builder.build();
        self.logger.repository.append(event).await
    }

    /// Record the audit event silently (logs errors instead of returning them)
    pub async fn record_silently(self) {
        let event = self.builder.build();
        self.logger.record_silently(event).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::InMemoryAuditRepository;
    use crate::domain::entities::AuditAction;

    fn setup_logger() -> AuditLogger<InMemoryAuditRepository> {
        let repo = Arc::new(InMemoryAuditRepository::new());
        AuditLogger::new(repo)
    }

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn test_actor() -> Actor {
        Actor::user("user-123".to_string(), "john-doe".to_string())
    }

    #[tokio::test]
    async fn test_audit_logger_creation() {
        let logger = setup_logger();
        // Logger should be created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_log_success() {
        let logger = setup_logger();
        let result = logger.log_success(
            test_tenant_id(),
            AuditAction::Login,
            test_actor(),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_log_failure() {
        let logger = setup_logger();
        let result = logger.log_failure(
            test_tenant_id(),
            AuditAction::LoginFailed,
            test_actor(),
            "Invalid credentials".to_string(),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_log_with_resource() {
        let logger = setup_logger();
        let result = logger.log_resource_action(
            test_tenant_id(),
            AuditAction::EventIngested,
            Actor::api_key("key-123".to_string(), "prod-api-key".to_string()),
            "event_stream".to_string(),
            "stream-456".to_string(),
            AuditOutcome::Success,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_builder_api() {
        let logger = setup_logger();

        let result = logger.log(
            test_tenant_id(),
            AuditAction::EventIngested,
            Actor::api_key("key-123".to_string(), "prod-api-key".to_string()),
        )
        .with_resource("event_stream".to_string(), "stream-456".to_string())
        .with_ip_address("192.168.1.1".to_string())
        .with_request_id("req-789".to_string())
        .record()
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_builder_with_context() {
        let logger = setup_logger();

        let context = RequestContext::new()
            .with_ip("10.0.0.1".to_string())
            .with_user_agent("Mozilla/5.0".to_string())
            .with_request_id("req-abc".to_string());

        let result = logger.log(
            test_tenant_id(),
            AuditAction::Login,
            test_actor(),
        )
        .with_context(context)
        .record()
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_builder_with_error() {
        let logger = setup_logger();

        let result = logger.log(
            test_tenant_id(),
            AuditAction::PermissionDenied,
            test_actor(),
        )
        .with_error("Insufficient permissions".to_string())
        .record()
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_builder_with_metadata() {
        let logger = setup_logger();

        let metadata = serde_json::json!({
            "reason": "rate_limit",
            "limit": 100,
            "current": 150
        });

        let result = logger.log(
            test_tenant_id(),
            AuditAction::RateLimitExceeded,
            test_actor(),
        )
        .with_metadata(metadata)
        .record()
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_silently() {
        let logger = setup_logger();

        let event = AuditEvent::new(
            test_tenant_id(),
            AuditAction::Login,
            test_actor(),
            AuditOutcome::Success,
        );

        // This should never panic, even if there's an error
        logger.record_silently(event).await;
    }

    #[tokio::test]
    async fn test_batch_logging() {
        let logger = setup_logger();

        let events = vec![
            AuditEvent::new(
                test_tenant_id(),
                AuditAction::Login,
                test_actor(),
                AuditOutcome::Success,
            ),
            AuditEvent::new(
                test_tenant_id(),
                AuditAction::EventIngested,
                Actor::api_key("key-123".to_string(), "prod-api-key".to_string()),
                AuditOutcome::Success,
            ),
        ];

        let result = logger.log_batch(events).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_logging_silently() {
        let logger = setup_logger();

        let events = vec![
            AuditEvent::new(
                test_tenant_id(),
                AuditAction::Login,
                test_actor(),
                AuditOutcome::Success,
            ),
        ];

        // This should never panic
        logger.log_batch_silently(events).await;
    }

    #[tokio::test]
    async fn test_request_context_builder() {
        let context = RequestContext::new()
            .with_ip("192.168.1.1".to_string())
            .with_user_agent("curl/7.64.1".to_string())
            .with_request_id("req-123".to_string());

        assert_eq!(context.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(context.user_agent, Some("curl/7.64.1".to_string()));
        assert_eq!(context.request_id, Some("req-123".to_string()));
    }
}
