use crate::domain::value_objects::{EntityId, EventType, TenantId};

/// System stream namespace prefix.
///
/// All system streams use event types starting with `_system.` and
/// a reserved tenant ID of `_system`. This isolates operational metadata
/// from user event data while leveraging the same durable storage engine.
///
/// # Naming Convention
///
/// - Event types: `_system.<domain>.<action>` (e.g., `_system.tenant.created`)
/// - Entity IDs: `_system:<domain>:<id>` (e.g., `_system:tenant:acme-corp`)
/// - Tenant ID: `_system` (reserved, rejected for user tenants)
///
/// # Inspiration
///
/// - Kafka KRaft: `__consumer_offsets`, `__transaction_state`
/// - CockroachDB: `system.*` tables
/// - FoundationDB: `\xff` system keyspace
/// - EventStoreDB: `$` prefix for system streams
///
/// The reserved event type prefix for system streams.
pub const SYSTEM_EVENT_TYPE_PREFIX: &str = "_system.";

/// The reserved tenant ID for system metadata.
pub const SYSTEM_TENANT_ID: &str = "_system";

/// The reserved entity ID prefix for system streams.
pub const SYSTEM_ENTITY_ID_PREFIX: &str = "_system:";

/// Known system stream domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemDomain {
    /// Tenant lifecycle: create, update, suspend, reactivate, delete
    Tenant,
    /// Audit log: immutable append-only security/compliance log
    Audit,
    /// Configuration: key-value settings (log-compacted)
    Config,
    /// Schema registry: schema definitions and versions
    Schema,
    /// Policies: access policies and retention rules
    Policy,
    /// Consumer: durable subscription cursor positions
    Consumer,
    /// Auth: API keys, users, and authentication metadata
    Auth,
}

impl SystemDomain {
    /// Get the domain name as used in event types and entity IDs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tenant => "tenant",
            Self::Audit => "audit",
            Self::Config => "config",
            Self::Schema => "schema",
            Self::Policy => "policy",
            Self::Consumer => "consumer",
            Self::Auth => "auth",
        }
    }

    /// All known system domains.
    pub fn all() -> &'static [SystemDomain] {
        &[
            Self::Tenant,
            Self::Audit,
            Self::Config,
            Self::Schema,
            Self::Policy,
            Self::Consumer,
            Self::Auth,
        ]
    }
}

/// System event types for tenant lifecycle.
pub mod tenant_events {
    pub const CREATED: &str = "_system.tenant.created";
    pub const UPDATED: &str = "_system.tenant.updated";
    pub const SUSPENDED: &str = "_system.tenant.suspended";
    pub const REACTIVATED: &str = "_system.tenant.reactivated";
    pub const DELETED: &str = "_system.tenant.deleted";
    pub const QUOTA_UPDATED: &str = "_system.tenant.quota_updated";
    pub const USAGE_UPDATED: &str = "_system.tenant.usage_updated";
    pub const SCHEMA_ENFORCEMENT_UPDATED: &str = "_system.tenant.schema_enforcement_updated";
}

/// System event types for audit log.
pub mod audit_events {
    pub const RECORDED: &str = "_system.audit.recorded";
}

/// System event types for configuration.
pub mod config_events {
    pub const SET: &str = "_system.config.set";
    pub const DELETED: &str = "_system.config.deleted";
}

/// System event types for schema registry.
pub mod schema_events {
    pub const REGISTERED: &str = "_system.schema.registered";
    pub const UPDATED: &str = "_system.schema.updated";
    pub const DELETED: &str = "_system.schema.deleted";
}

/// System event types for policies.
pub mod policy_events {
    pub const CREATED: &str = "_system.policy.created";
    pub const UPDATED: &str = "_system.policy.updated";
    pub const DELETED: &str = "_system.policy.deleted";
}

/// System event types for authentication (API keys, users).
pub mod auth_events {
    pub const KEY_PROVISIONED: &str = "_system.auth.key_provisioned";
    pub const KEY_REVOKED: &str = "_system.auth.key_revoked";
}

/// System event types for consumer cursor tracking.
pub mod consumer_events {
    pub const REGISTERED: &str = "_system.consumer.registered";
    pub const ACK_UPDATED: &str = "_system.consumer.ack_updated";
    pub const DELETED: &str = "_system.consumer.deleted";
}

/// Check whether an event type string belongs to the system namespace.
pub fn is_system_event_type(event_type: &str) -> bool {
    event_type.starts_with(SYSTEM_EVENT_TYPE_PREFIX)
}

/// Check whether an entity ID string belongs to the system namespace.
pub fn is_system_entity_id(entity_id: &str) -> bool {
    entity_id.starts_with(SYSTEM_ENTITY_ID_PREFIX)
}

/// Check whether a tenant ID is the reserved system tenant.
pub fn is_system_tenant_id(tenant_id: &str) -> bool {
    tenant_id == SYSTEM_TENANT_ID
}

/// Build a system entity ID for a given domain and resource ID.
///
/// Example: `system_entity_id(SystemDomain::Tenant, "acme-corp")` → `_system:tenant:acme-corp`
pub fn system_entity_id(domain: SystemDomain, resource_id: &str) -> String {
    format!(
        "{}{}:{}",
        SYSTEM_ENTITY_ID_PREFIX,
        domain.as_str(),
        resource_id
    )
}

/// Get the system tenant ID as a `TenantId` value object.
pub fn system_tenant_id() -> TenantId {
    TenantId::new_unchecked(SYSTEM_TENANT_ID.to_string())
}

/// Create a system event type from a raw string constant.
///
/// Uses `new_unchecked` because system event type constants are compile-time validated.
pub fn system_event_type(event_type_str: &str) -> EventType {
    debug_assert!(
        event_type_str.starts_with(SYSTEM_EVENT_TYPE_PREFIX),
        "System event type must start with '{SYSTEM_EVENT_TYPE_PREFIX}'"
    );
    EventType::new_unchecked(event_type_str.to_string())
}

/// Create a system entity ID value object.
pub fn system_entity_id_value(domain: SystemDomain, resource_id: &str) -> EntityId {
    EntityId::new_unchecked(system_entity_id(domain, resource_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_event_type_prefix() {
        assert!(is_system_event_type("_system.tenant.created"));
        assert!(is_system_event_type("_system.config.set"));
        assert!(!is_system_event_type("order.placed"));
        assert!(!is_system_event_type("system.tenant.created"));
    }

    #[test]
    fn test_system_entity_id_prefix() {
        assert!(is_system_entity_id("_system:tenant:acme"));
        assert!(is_system_entity_id("_system:config:max_conn"));
        assert!(!is_system_entity_id("user-123"));
        assert!(!is_system_entity_id("system:tenant:acme"));
    }

    #[test]
    fn test_system_tenant_id() {
        assert!(is_system_tenant_id("_system"));
        assert!(!is_system_tenant_id("default"));
        assert!(!is_system_tenant_id("acme-corp"));
    }

    #[test]
    fn test_system_entity_id_construction() {
        let id = system_entity_id(SystemDomain::Tenant, "acme-corp");
        assert_eq!(id, "_system:tenant:acme-corp");

        let id = system_entity_id(SystemDomain::Config, "max_connections");
        assert_eq!(id, "_system:config:max_connections");

        let id = system_entity_id(SystemDomain::Audit, "tenant-1");
        assert_eq!(id, "_system:audit:tenant-1");
    }

    #[test]
    fn test_system_domain_all() {
        let domains = SystemDomain::all();
        assert_eq!(domains.len(), 7);
    }

    #[test]
    fn test_system_event_type_constants_are_valid() {
        // All system event type constants should pass EventType validation
        let constants = [
            tenant_events::CREATED,
            tenant_events::UPDATED,
            tenant_events::SUSPENDED,
            tenant_events::REACTIVATED,
            tenant_events::DELETED,
            tenant_events::QUOTA_UPDATED,
            tenant_events::USAGE_UPDATED,
            audit_events::RECORDED,
            config_events::SET,
            config_events::DELETED,
            schema_events::REGISTERED,
            schema_events::UPDATED,
            schema_events::DELETED,
            policy_events::CREATED,
            policy_events::UPDATED,
            policy_events::DELETED,
            auth_events::KEY_PROVISIONED,
            auth_events::KEY_REVOKED,
            consumer_events::REGISTERED,
            consumer_events::ACK_UPDATED,
            consumer_events::DELETED,
        ];

        for constant in constants {
            let result = EventType::new(constant.to_string());
            assert!(
                result.is_ok(),
                "System event type '{}' failed validation: {:?}",
                constant,
                result.err()
            );
        }
    }

    #[test]
    fn test_system_tenant_id_is_valid() {
        let result = TenantId::new(SYSTEM_TENANT_ID.to_string());
        assert!(result.is_ok(), "System tenant ID should be valid");
    }

    #[test]
    fn test_system_entity_ids_are_valid() {
        for domain in SystemDomain::all() {
            let id_str = system_entity_id(*domain, "test-resource");
            let result = EntityId::new(id_str.clone());
            assert!(
                result.is_ok(),
                "System entity ID '{}' failed validation: {:?}",
                id_str,
                result.err()
            );
        }
    }

    #[test]
    fn test_system_event_type_helper() {
        let et = system_event_type(tenant_events::CREATED);
        assert_eq!(et.as_str(), "_system.tenant.created");
    }

    #[test]
    fn test_system_entity_id_value_helper() {
        let eid = system_entity_id_value(SystemDomain::Tenant, "acme");
        assert_eq!(eid.as_str(), "_system:tenant:acme");
    }
}
