/// Audit Event Domain Entity
///
/// Represents an immutable audit log entry for security, compliance, and debugging.
/// Every authenticated operation in the system generates an audit event.
///
/// # Design Principles
/// - **Immutability**: Audit events cannot be modified after creation
/// - **Completeness**: All relevant context captured (who, what, when, where, result)
/// - **Security**: Sensitive data (passwords, tokens) never logged
/// - **Compliance**: Meets SOC 2, GDPR, HIPAA audit requirements

use crate::domain::value_objects::{EntityId, TenantId};
use crate::error::{AllSourceError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Audit Event ID (UUID v4)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuditEventId(Uuid);

impl AuditEventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for AuditEventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Action types for audit events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // Authentication
    Login,
    Logout,
    LoginFailed,
    TokenRefreshed,
    PasswordChanged,

    // API Keys
    ApiKeyCreated,
    ApiKeyRevoked,
    ApiKeyUsed,

    // Event Operations
    EventIngested,
    EventQueried,
    EventStreamCreated,

    // Tenant Management
    TenantCreated,
    TenantUpdated,
    TenantActivated,
    TenantDeactivated,
    TenantDeleted,

    // Schema Management
    SchemaRegistered,
    SchemaUpdated,
    SchemaDeleted,

    // Projection Management
    ProjectionCreated,
    ProjectionUpdated,
    ProjectionStarted,
    ProjectionStopped,
    ProjectionDeleted,

    // Pipeline Management
    PipelineCreated,
    PipelineUpdated,
    PipelineDeleted,

    // User Management
    UserCreated,
    UserUpdated,
    UserDeleted,
    RoleChanged,

    // Security
    PermissionDenied,
    RateLimitExceeded,
    IpBlocked,
    SuspiciousActivity,

    // System
    ConfigurationChanged,
    BackupCreated,
    BackupRestored,
}

impl AuditAction {
    /// Get action category for filtering
    pub fn category(&self) -> AuditCategory {
        match self {
            Self::Login | Self::Logout | Self::LoginFailed | Self::TokenRefreshed | Self::PasswordChanged => {
                AuditCategory::Authentication
            }
            Self::ApiKeyCreated | Self::ApiKeyRevoked | Self::ApiKeyUsed => {
                AuditCategory::ApiKey
            }
            Self::EventIngested | Self::EventQueried | Self::EventStreamCreated => {
                AuditCategory::Event
            }
            Self::TenantCreated | Self::TenantUpdated | Self::TenantActivated | Self::TenantDeactivated | Self::TenantDeleted => {
                AuditCategory::Tenant
            }
            Self::SchemaRegistered | Self::SchemaUpdated | Self::SchemaDeleted => {
                AuditCategory::Schema
            }
            Self::ProjectionCreated | Self::ProjectionUpdated | Self::ProjectionStarted | Self::ProjectionStopped | Self::ProjectionDeleted => {
                AuditCategory::Projection
            }
            Self::PipelineCreated | Self::PipelineUpdated | Self::PipelineDeleted => {
                AuditCategory::Pipeline
            }
            Self::UserCreated | Self::UserUpdated | Self::UserDeleted | Self::RoleChanged => {
                AuditCategory::User
            }
            Self::PermissionDenied | Self::RateLimitExceeded | Self::IpBlocked | Self::SuspiciousActivity => {
                AuditCategory::Security
            }
            Self::ConfigurationChanged | Self::BackupCreated | Self::BackupRestored => {
                AuditCategory::System
            }
        }
    }

    /// Check if action represents a security concern
    pub fn is_security_event(&self) -> bool {
        matches!(
            self,
            Self::LoginFailed
                | Self::PermissionDenied
                | Self::RateLimitExceeded
                | Self::IpBlocked
                | Self::SuspiciousActivity
        )
    }
}

/// Audit event categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Authentication,
    ApiKey,
    Event,
    Tenant,
    Schema,
    Projection,
    Pipeline,
    User,
    Security,
    System,
}

/// Audit event outcome
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Success,
    Failure,
    PartialSuccess,
}

/// Actor who performed the action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Actor {
    User {
        user_id: String,
        username: String,
    },
    ApiKey {
        key_id: String,
        key_name: String,
    },
    System {
        component: String,
    },
}

impl Actor {
    pub fn user(user_id: String, username: String) -> Self {
        Self::User { user_id, username }
    }

    pub fn api_key(key_id: String, key_name: String) -> Self {
        Self::ApiKey { key_id, key_name }
    }

    pub fn system(component: String) -> Self {
        Self::System { component }
    }

    /// Get actor identifier for logging
    pub fn identifier(&self) -> String {
        match self {
            Self::User { user_id, .. } => format!("user:{}", user_id),
            Self::ApiKey { key_id, .. } => format!("api_key:{}", key_id),
            Self::System { component } => format!("system:{}", component),
        }
    }
}

/// Audit Event Domain Entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    id: AuditEventId,

    /// Tenant this audit event belongs to
    tenant_id: TenantId,

    /// Timestamp (UTC)
    timestamp: DateTime<Utc>,

    /// Action performed
    action: AuditAction,

    /// Actor who performed the action
    actor: Actor,

    /// Resource affected (optional)
    resource_type: Option<String>,
    resource_id: Option<String>,

    /// Outcome of the action
    outcome: AuditOutcome,

    /// IP address of the requester
    ip_address: Option<String>,

    /// User agent
    user_agent: Option<String>,

    /// Request ID for correlation
    request_id: Option<String>,

    /// Error message (if failure)
    error_message: Option<String>,

    /// Additional metadata (JSON)
    metadata: Option<JsonValue>,
}

impl AuditEvent {
    /// Create a new audit event
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: TenantId,
        action: AuditAction,
        actor: Actor,
        outcome: AuditOutcome,
    ) -> Self {
        Self {
            id: AuditEventId::new(),
            tenant_id,
            timestamp: Utc::now(),
            action,
            actor,
            resource_type: None,
            resource_id: None,
            outcome,
            ip_address: None,
            user_agent: None,
            request_id: None,
            error_message: None,
            metadata: None,
        }
    }

    /// Builder pattern methods

    pub fn with_resource(mut self, resource_type: String, resource_id: String) -> Self {
        self.resource_type = Some(resource_type);
        self.resource_id = Some(resource_id);
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

    pub fn with_error(mut self, error: String) -> Self {
        self.error_message = Some(error);
        self
    }

    pub fn with_metadata(mut self, metadata: JsonValue) -> Self {
        self.metadata = Some(metadata);
        self
    }

    // Getters

    pub fn id(&self) -> &AuditEventId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn timestamp(&self) -> &DateTime<Utc> {
        &self.timestamp
    }

    pub fn action(&self) -> &AuditAction {
        &self.action
    }

    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    pub fn resource_type(&self) -> Option<&str> {
        self.resource_type.as_deref()
    }

    pub fn resource_id(&self) -> Option<&str> {
        self.resource_id.as_deref()
    }

    pub fn outcome(&self) -> &AuditOutcome {
        &self.outcome
    }

    pub fn ip_address(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }

    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }

    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn metadata(&self) -> Option<&JsonValue> {
        self.metadata.as_ref()
    }

    /// Check if this is a security event
    pub fn is_security_event(&self) -> bool {
        self.action.is_security_event()
    }

    /// Get action category
    pub fn category(&self) -> AuditCategory {
        self.action.category()
    }

    /// Check if audit event represents a failure
    pub fn is_failure(&self) -> bool {
        matches!(self.outcome, AuditOutcome::Failure)
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        let actor_desc = match &self.actor {
            Actor::User { username, .. } => format!("User '{}'", username),
            Actor::ApiKey { key_name, .. } => format!("API Key '{}'", key_name),
            Actor::System { component } => format!("System component '{}'", component),
        };

        let resource_desc = if let (Some(r_type), Some(r_id)) = (&self.resource_type, &self.resource_id) {
            format!(" on {} '{}'", r_type, r_id)
        } else {
            String::new()
        };

        let outcome_desc = match self.outcome {
            AuditOutcome::Success => "succeeded",
            AuditOutcome::Failure => "failed",
            AuditOutcome::PartialSuccess => "partially succeeded",
        };

        format!(
            "{} performed {:?}{} ({})",
            actor_desc, self.action, resource_desc, outcome_desc
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_audit_event() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let actor = Actor::user("user-123".to_string(), "john.doe".to_string());

        let event = AuditEvent::new(
            tenant_id.clone(),
            AuditAction::Login,
            actor,
            AuditOutcome::Success,
        );

        assert_eq!(event.tenant_id(), &tenant_id);
        assert_eq!(event.action(), &AuditAction::Login);
        assert_eq!(event.outcome(), &AuditOutcome::Success);
        assert!(!event.is_failure());
    }

    #[test]
    fn test_audit_event_with_resource() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let actor = Actor::api_key("key-123".to_string(), "prod-api-key".to_string());

        let event = AuditEvent::new(
            tenant_id,
            AuditAction::EventIngested,
            actor,
            AuditOutcome::Success,
        )
        .with_resource("event_stream".to_string(), "stream-123".to_string())
        .with_ip_address("192.168.1.1".to_string())
        .with_request_id("req-456".to_string());

        assert_eq!(event.resource_type(), Some("event_stream"));
        assert_eq!(event.resource_id(), Some("stream-123"));
        assert_eq!(event.ip_address(), Some("192.168.1.1"));
        assert_eq!(event.request_id(), Some("req-456"));
    }

    #[test]
    fn test_audit_event_with_error() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let actor = Actor::user("user-456".to_string(), "jane.doe".to_string());

        let event = AuditEvent::new(
            tenant_id,
            AuditAction::LoginFailed,
            actor,
            AuditOutcome::Failure,
        )
        .with_error("Invalid password".to_string())
        .with_ip_address("10.0.0.1".to_string());

        assert!(event.is_failure());
        assert_eq!(event.error_message(), Some("Invalid password"));
        assert!(event.is_security_event());
    }

    #[test]
    fn test_audit_event_with_metadata() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let actor = Actor::system("backup-service".to_string());

        let metadata = json!({
            "backup_size_bytes": 1024000,
            "compression": "gzip",
            "location": "s3://backups/2025-10-26"
        });

        let event = AuditEvent::new(
            tenant_id,
            AuditAction::BackupCreated,
            actor,
            AuditOutcome::Success,
        )
        .with_metadata(metadata.clone());

        assert_eq!(event.metadata(), Some(&metadata));
    }

    #[test]
    fn test_actor_identifier() {
        let user_actor = Actor::user("user-123".to_string(), "john".to_string());
        assert_eq!(user_actor.identifier(), "user:user-123");

        let api_actor = Actor::api_key("key-456".to_string(), "prod-key".to_string());
        assert_eq!(api_actor.identifier(), "api_key:key-456");

        let system_actor = Actor::system("compaction".to_string());
        assert_eq!(system_actor.identifier(), "system:compaction");
    }

    #[test]
    fn test_audit_action_category() {
        assert_eq!(AuditAction::Login.category(), AuditCategory::Authentication);
        assert_eq!(AuditAction::EventIngested.category(), AuditCategory::Event);
        assert_eq!(AuditAction::TenantCreated.category(), AuditCategory::Tenant);
        assert_eq!(AuditAction::PermissionDenied.category(), AuditCategory::Security);
    }

    #[test]
    fn test_security_event_detection() {
        assert!(AuditAction::LoginFailed.is_security_event());
        assert!(AuditAction::PermissionDenied.is_security_event());
        assert!(AuditAction::RateLimitExceeded.is_security_event());
        assert!(AuditAction::IpBlocked.is_security_event());
        assert!(!AuditAction::Login.is_security_event());
        assert!(!AuditAction::EventIngested.is_security_event());
    }

    #[test]
    fn test_audit_event_description() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let actor = Actor::user("user-123".to_string(), "john.doe".to_string());

        let event = AuditEvent::new(
            tenant_id,
            AuditAction::EventIngested,
            actor,
            AuditOutcome::Success,
        )
        .with_resource("event_stream".to_string(), "stream-456".to_string());

        let desc = event.description();
        assert!(desc.contains("john.doe"));
        assert!(desc.contains("EventIngested"));
        assert!(desc.contains("event_stream"));
        assert!(desc.contains("stream-456"));
        assert!(desc.contains("succeeded"));
    }

    #[test]
    fn test_audit_event_id() {
        let id1 = AuditEventId::new();
        let id2 = AuditEventId::new();

        assert_ne!(id1, id2); // UUIDs should be unique

        let uuid = Uuid::new_v4();
        let id3 = AuditEventId::from_uuid(uuid);
        assert_eq!(id3.as_uuid(), &uuid);
    }

    #[test]
    fn test_audit_event_serde() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let actor = Actor::user("user-123".to_string(), "john".to_string());

        let event = AuditEvent::new(
            tenant_id,
            AuditAction::Login,
            actor,
            AuditOutcome::Success,
        );

        // Serialize
        let json = serde_json::to_string(&event).unwrap();

        // Deserialize
        let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.id(), deserialized.id());
        assert_eq!(event.action(), deserialized.action());
        assert_eq!(event.outcome(), deserialized.outcome());
    }
}
