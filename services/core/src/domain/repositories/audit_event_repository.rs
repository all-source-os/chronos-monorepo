/// Audit Event Repository Trait
///
/// Defines the contract for persisting and querying audit events.
/// Implementations must provide both write (append-only) and query capabilities.

use async_trait::async_trait;
use crate::domain::entities::{AuditEvent, AuditEventId, AuditAction, AuditCategory};
use crate::domain::value_objects::TenantId;
use crate::error::Result;
use chrono::{DateTime, Utc};

/// Query parameters for filtering audit events
#[derive(Debug, Clone)]
pub struct AuditEventQuery {
    /// Filter by tenant (required for isolation)
    pub tenant_id: TenantId,

    /// Filter by time range (optional)
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,

    /// Filter by action (optional)
    pub action: Option<AuditAction>,

    /// Filter by category (optional)
    pub category: Option<AuditCategory>,

    /// Filter by actor (optional)
    pub actor_identifier: Option<String>,

    /// Filter by resource (optional)
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,

    /// Filter security events only
    pub security_events_only: bool,

    /// Pagination
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl AuditEventQuery {
    /// Create a new query for a tenant
    pub fn new(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            start_time: None,
            end_time: None,
            action: None,
            category: None,
            actor_identifier: None,
            resource_type: None,
            resource_id: None,
            security_events_only: false,
            limit: None,
            offset: None,
        }
    }

    /// Filter by time range
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    /// Filter by action
    pub fn with_action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Filter by category
    pub fn with_category(mut self, category: AuditCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Filter by actor
    pub fn with_actor(mut self, actor_identifier: String) -> Self {
        self.actor_identifier = Some(actor_identifier);
        self
    }

    /// Filter by resource
    pub fn with_resource(mut self, resource_type: String, resource_id: String) -> Self {
        self.resource_type = Some(resource_type);
        self.resource_id = Some(resource_id);
        self
    }

    /// Filter security events only
    pub fn security_only(mut self) -> Self {
        self.security_events_only = true;
        self
    }

    /// Set pagination
    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
}

/// Audit Event Repository Trait
///
/// **Design Principles**:
/// - **Append-only**: Audit events are immutable, never updated/deleted
/// - **Tenant isolation**: All queries scoped to tenant
/// - **Performance**: Indexed queries for fast retrieval
/// - **Compliance**: Support compliance requirements (SOC 2, GDPR, HIPAA)
#[async_trait]
pub trait AuditEventRepository: Send + Sync {
    /// Append an audit event (immutable, cannot be modified after creation)
    ///
    /// # Arguments
    /// - `event`: The audit event to persist
    ///
    /// # Returns
    /// - `Result<()>`: Success or error
    async fn append(&self, event: AuditEvent) -> Result<()>;

    /// Append multiple audit events in a batch
    ///
    /// # Arguments
    /// - `events`: Vec of audit events to persist
    ///
    /// # Returns
    /// - `Result<()>`: Success or error
    async fn append_batch(&self, events: Vec<AuditEvent>) -> Result<()>;

    /// Get an audit event by ID
    ///
    /// # Arguments
    /// - `id`: The audit event ID
    ///
    /// # Returns
    /// - `Result<Option<AuditEvent>>`: The event if found
    async fn get_by_id(&self, id: &AuditEventId) -> Result<Option<AuditEvent>>;

    /// Query audit events with filters
    ///
    /// # Arguments
    /// - `query`: Query parameters for filtering
    ///
    /// # Returns
    /// - `Result<Vec<AuditEvent>>`: Matching events ordered by timestamp (newest first)
    async fn query(&self, query: AuditEventQuery) -> Result<Vec<AuditEvent>>;

    /// Count audit events matching query
    ///
    /// # Arguments
    /// - `query`: Query parameters for filtering
    ///
    /// # Returns
    /// - `Result<usize>`: Number of matching events
    async fn count(&self, query: AuditEventQuery) -> Result<usize>;

    /// Get all audit events for a tenant (paginated)
    ///
    /// # Arguments
    /// - `tenant_id`: The tenant ID
    /// - `limit`: Maximum number of events to return
    /// - `offset`: Number of events to skip
    ///
    /// # Returns
    /// - `Result<Vec<AuditEvent>>`: Events ordered by timestamp (newest first)
    async fn get_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>>;

    /// Get security events for a tenant (failed logins, permission denials, etc.)
    ///
    /// # Arguments
    /// - `tenant_id`: The tenant ID
    /// - `limit`: Maximum number of events to return
    ///
    /// # Returns
    /// - `Result<Vec<AuditEvent>>`: Security events ordered by timestamp (newest first)
    async fn get_security_events(
        &self,
        tenant_id: &TenantId,
        limit: usize,
    ) -> Result<Vec<AuditEvent>>;

    /// Get recent audit events for a specific actor
    ///
    /// # Arguments
    /// - `tenant_id`: The tenant ID (for isolation)
    /// - `actor_identifier`: Actor identifier (e.g., "user:123", "api_key:456")
    /// - `limit`: Maximum number of events to return
    ///
    /// # Returns
    /// - `Result<Vec<AuditEvent>>`: Events ordered by timestamp (newest first)
    async fn get_by_actor(
        &self,
        tenant_id: &TenantId,
        actor_identifier: &str,
        limit: usize,
    ) -> Result<Vec<AuditEvent>>;

    /// Purge old audit events (for GDPR compliance / data retention)
    ///
    /// # Arguments
    /// - `tenant_id`: The tenant ID
    /// - `older_than`: Delete events older than this timestamp
    ///
    /// # Returns
    /// - `Result<usize>`: Number of events deleted
    ///
    /// **Note**: This violates append-only principle but is required for compliance
    async fn purge_old_events(
        &self,
        tenant_id: &TenantId,
        older_than: DateTime<Utc>,
    ) -> Result<usize>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_query_builder() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        let query = AuditEventQuery::new(tenant_id.clone())
            .with_action(AuditAction::Login)
            .with_pagination(100, 0)
            .security_only();

        assert_eq!(query.tenant_id, tenant_id);
        assert_eq!(query.action, Some(AuditAction::Login));
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(0));
        assert!(query.security_events_only);
    }

    #[test]
    fn test_audit_event_query_with_time_range() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let start = Utc::now();
        let end = Utc::now();

        let query = AuditEventQuery::new(tenant_id)
            .with_time_range(start, end);

        assert_eq!(query.start_time, Some(start));
        assert_eq!(query.end_time, Some(end));
    }

    #[test]
    fn test_audit_event_query_with_resource() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        let query = AuditEventQuery::new(tenant_id)
            .with_resource("event_stream".to_string(), "stream-123".to_string());

        assert_eq!(query.resource_type, Some("event_stream".to_string()));
        assert_eq!(query.resource_id, Some("stream-123".to_string()));
    }

    #[test]
    fn test_audit_event_query_with_actor() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();

        let query = AuditEventQuery::new(tenant_id)
            .with_actor("user:john.doe".to_string());

        assert_eq!(query.actor_identifier, Some("user:john.doe".to_string()));
    }
}
