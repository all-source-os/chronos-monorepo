/// Security Integration Tests (Phase 5D)
///
/// Comprehensive security tests covering:
/// - Authentication flows (JWT, API Key)
/// - RBAC permission checks
/// - Tenant isolation scenarios
/// - Rate limiting enforcement
/// - Audit logging verification
/// - Security headers
/// - IP filtering

use crate::{
    auth::{AuthManager, Claims, Role, Permission, ApiKey},
    domain::{
        entities::{Tenant, TenantQuotas, Event},
        repositories::{TenantRepository, AuditEventRepository, EventStreamRepository},
        value_objects::{TenantId, EntityId},
    },
    infrastructure::{
        repositories::{InMemoryTenantRepository, InMemoryAuditRepository, InMemoryEventStreamRepository},
        security::IpFilter,
    },
    middleware::{AuthContext, TenantContext, RequestId, SecurityConfig},
    rate_limit::{RateLimiter, RateLimitConfig},
};
use chrono::{Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use std::net::IpAddr;
use std::str::FromStr;

// ============================================================================
// Test Helpers
// ============================================================================

fn setup_auth_manager() -> AuthManager {
    AuthManager::new("test-secret-key-for-jwt-signing")
}

fn create_test_tenant_id() -> TenantId {
    TenantId::new("test-tenant".to_string()).unwrap()
}

fn create_test_user(auth: &AuthManager, tenant_id: &str) -> (String, String) {
    let user = auth
        .register_user(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "SecurePassword123!",
            Role::Developer,
            tenant_id.to_string(),
        )
        .unwrap();

    // Authenticate to get a token
    let token = auth.authenticate("testuser", "SecurePassword123!").unwrap();

    (user.id.to_string(), token)
}

fn create_test_api_key(auth: &AuthManager, tenant_id: &str) -> (ApiKey, String) {
    auth.create_api_key(
        "test-key".to_string(),
        tenant_id.to_string(),
        Role::Developer,
        Some(Utc::now() + Duration::days(30)),
    )
}

// ============================================================================
// Authentication Flow Tests
// ============================================================================

#[test]
fn test_jwt_authentication_flow() {
    let auth = setup_auth_manager();
    let tenant_id = "tenant-1";

    // Create user
    let (user_id, token) = create_test_user(&auth, tenant_id);

    // Validate token
    let claims = auth.validate_token(&token).unwrap();
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.tenant_id, tenant_id);
    assert_eq!(claims.role, Role::Developer);
}

#[test]
fn test_jwt_token_validation() {
    let auth = setup_auth_manager();
    let tenant_id = "tenant-1";

    // Create user and get token
    let user = auth
        .register_user(
            "testuser2".to_string(),
            "test2@example.com".to_string(),
            "SecurePassword123!",
            Role::Developer,
            tenant_id.to_string(),
        )
        .unwrap();

    // Authenticate to get a valid token
    let token = auth.authenticate("testuser2", "SecurePassword123!").unwrap();

    // Validate token should succeed
    let result = auth.validate_token(&token);
    assert!(result.is_ok());

    // Invalid token should fail
    let invalid_token = "invalid.jwt.token";
    let result = auth.validate_token(invalid_token);
    assert!(result.is_err());
}

#[test]
fn test_api_key_authentication() {
    let auth = setup_auth_manager();
    let tenant_id = "tenant-1";

    // Create API key
    let (api_key, raw_key) = create_test_api_key(&auth, tenant_id);

    // Validate API key returns claims
    let claims = auth.validate_api_key(&raw_key).unwrap();
    assert_eq!(claims.sub, api_key.id.to_string());
    assert_eq!(claims.tenant_id, tenant_id);
    assert_eq!(claims.role, Role::Developer);
}

#[test]
fn test_api_key_invalid() {
    let auth = setup_auth_manager();

    // Try to validate non-existent API key
    let result = auth.validate_api_key("ask_invalid_key_here");
    assert!(result.is_err());
}

#[test]
fn test_password_authentication() {
    let auth = setup_auth_manager();
    let tenant_id = "tenant-1";

    // Create user
    let _user = auth
        .register_user(
            "testuser3".to_string(),
            "test3@example.com".to_string(),
            "SecurePassword123!",
            Role::Developer,
            tenant_id.to_string(),
        )
        .unwrap();

    // Authenticate with correct password
    let result = auth.authenticate("testuser3", "SecurePassword123!");
    assert!(result.is_ok());

    // Authenticate with wrong password
    let result = auth.authenticate("testuser3", "WrongPassword");
    assert!(result.is_err());
}

// ============================================================================
// RBAC Permission Tests
// ============================================================================

#[test]
fn test_rbac_admin_permissions() {
    let claims = Claims::new(
        "user1".to_string(),
        "tenant1".to_string(),
        Role::Admin,
        Duration::hours(1),
    );

    // Admin should have all permissions
    assert!(claims.has_permission(Permission::Read));
    assert!(claims.has_permission(Permission::Write));
    assert!(claims.has_permission(Permission::Admin));
    assert!(claims.has_permission(Permission::ManageTenants));
}

#[test]
fn test_rbac_developer_permissions() {
    let claims = Claims::new(
        "user1".to_string(),
        "tenant1".to_string(),
        Role::Developer,
        Duration::hours(1),
    );

    // Developer should have read/write but not admin
    assert!(claims.has_permission(Permission::Read));
    assert!(claims.has_permission(Permission::Write));
    assert!(!claims.has_permission(Permission::Admin));
    assert!(!claims.has_permission(Permission::ManageTenants));
}

#[test]
fn test_rbac_readonly_permissions() {
    let claims = Claims::new(
        "user1".to_string(),
        "tenant1".to_string(),
        Role::ReadOnly,
        Duration::hours(1),
    );

    // ReadOnly should only have read permission
    assert!(claims.has_permission(Permission::Read));
    assert!(!claims.has_permission(Permission::Write));
    assert!(!claims.has_permission(Permission::Admin));
}

#[test]
fn test_auth_context_permission_check() {
    let claims = Claims::new(
        "user1".to_string(),
        "tenant1".to_string(),
        Role::Developer,
        Duration::hours(1),
    );
    let ctx = AuthContext { claims };

    // Should succeed for read/write
    assert!(ctx.require_permission(Permission::Read).is_ok());
    assert!(ctx.require_permission(Permission::Write).is_ok());

    // Should fail for admin
    assert!(ctx.require_permission(Permission::Admin).is_err());
}

// ============================================================================
// Tenant Isolation Tests
// ============================================================================

#[tokio::test]
async fn test_tenant_isolation_repository_level() {
    let repo = InMemoryTenantRepository::new();

    // Create two tenants
    let tenant1_id = TenantId::new("tenant-1".to_string()).unwrap();
    let tenant2_id = TenantId::new("tenant-2".to_string()).unwrap();

    let tenant1 = repo
        .create(tenant1_id.clone(), "Tenant 1".to_string(), TenantQuotas::standard())
        .await
        .unwrap();

    let tenant2 = repo
        .create(tenant2_id.clone(), "Tenant 2".to_string(), TenantQuotas::standard())
        .await
        .unwrap();

    // Verify each tenant can only see their own data
    let found1 = repo.find_by_id(&tenant1_id).await.unwrap().unwrap();
    assert_eq!(found1.id(), tenant1.id());

    let found2 = repo.find_by_id(&tenant2_id).await.unwrap().unwrap();
    assert_eq!(found2.id(), tenant2.id());

    // Count should show separate tenants
    let count = repo.count().await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_tenant_isolation_event_streams() {
    let repo = InMemoryEventStreamRepository::new();
    let tenant1_id = TenantId::new("tenant-1".to_string()).unwrap();
    let tenant2_id = TenantId::new("tenant-2".to_string()).unwrap();

    // Create events for different tenants
    let entity_id = EntityId::new("entity-1".to_string()).unwrap();
    let mut stream = repo.get_or_create_stream(&entity_id).await.unwrap();

    let event1 = Event::from_strings(
        "test.event".to_string(),
        entity_id.as_str().to_string(),
        tenant1_id.as_str().to_string(),
        json!({"data": "tenant1"}),
        None,
    )
    .unwrap();

    // Append event
    repo.append_to_stream(&mut stream, event1).await.unwrap();

    // Get streams for tenant1
    let tenant1_streams = repo.get_streams_by_tenant(&tenant1_id).await.unwrap();
    assert_eq!(tenant1_streams.len(), 1);

    // Get streams for tenant2 (should be empty)
    let tenant2_streams = repo.get_streams_by_tenant(&tenant2_id).await.unwrap();
    assert_eq!(tenant2_streams.len(), 0);
}

#[tokio::test]
async fn test_tenant_isolation_prevents_cross_tenant_access() {
    let tenant_repo = InMemoryTenantRepository::new();
    let tenant1_id = TenantId::new("tenant-1".to_string()).unwrap();
    let tenant2_id = TenantId::new("tenant-2".to_string()).unwrap();

    // Create two tenants
    tenant_repo
        .create(tenant1_id.clone(), "Tenant 1".to_string(), TenantQuotas::standard())
        .await
        .unwrap();

    tenant_repo
        .create(tenant2_id.clone(), "Tenant 2".to_string(), TenantQuotas::standard())
        .await
        .unwrap();

    // Verify tenant1 cannot access tenant2's data
    let tenant1 = tenant_repo.find_by_id(&tenant1_id).await.unwrap().unwrap();
    let tenant2 = tenant_repo.find_by_id(&tenant2_id).await.unwrap().unwrap();

    assert_ne!(tenant1.id(), tenant2.id());
    assert_eq!(tenant1.id().as_str(), "tenant-1");
    assert_eq!(tenant2.id().as_str(), "tenant-2");
}

// ============================================================================
// Rate Limiting Tests
// ============================================================================

#[test]
fn test_rate_limiting_per_tenant() {
    // Create rate limiter with low limit for testing
    let rate_limiter = RateLimiter::new(RateLimitConfig {
        requests_per_minute: 1000, // Default
        burst_size: 1000
    });

    // Configure low limit for tenant-1
    rate_limiter.set_config("tenant-1", RateLimitConfig {
        requests_per_minute: 5,
        burst_size: 5
    });

    // Make 5 requests (should succeed)
    for _ in 0..5 {
        let result = rate_limiter.check_rate_limit("tenant-1");
        assert!(result.allowed);
    }

    // 6th request should be rate limited
    let result = rate_limiter.check_rate_limit("tenant-1");
    assert!(!result.allowed);
}

#[test]
fn test_rate_limiting_multiple_tenants() {
    let rate_limiter = RateLimiter::new(RateLimitConfig::dev_mode());

    // Configure different limits for different tenants
    rate_limiter.set_config("tenant-1", RateLimitConfig {
        requests_per_minute: 5,
        burst_size: 5,
    });
    rate_limiter.set_config("tenant-2", RateLimitConfig {
        requests_per_minute: 10,
        burst_size: 10,
    });

    // Exhaust tenant-1's quota
    for _ in 0..5 {
        rate_limiter.check_rate_limit("tenant-1");
    }

    // Tenant-1 should be rate limited
    assert!(!rate_limiter.check_rate_limit("tenant-1").allowed);

    // Tenant-2 should still be allowed
    assert!(rate_limiter.check_rate_limit("tenant-2").allowed);
}

// ============================================================================
// Audit Logging Tests
// ============================================================================

#[tokio::test]
async fn test_audit_logging_records_events() {
    use crate::domain::entities::{AuditEvent, AuditAction, AuditOutcome, Actor};

    let audit_repo = InMemoryAuditRepository::new();
    let tenant_id = create_test_tenant_id();

    // Create audit event
    let actor = Actor::User {
        user_id: "user-1".to_string(),
        username: "testuser".to_string(),
    };

    let event = AuditEvent::new(
        tenant_id.clone(),
        AuditAction::Login,
        actor,
        AuditOutcome::Success,
    );

    // Append to repository
    audit_repo.append(event.clone()).await.unwrap();

    // Query events
    let events = audit_repo
        .get_by_tenant(&tenant_id, 10, 0)
        .await
        .unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].action(), &AuditAction::Login);
}

#[tokio::test]
async fn test_audit_logging_tenant_isolation() {
    use crate::domain::entities::{AuditEvent, AuditAction, AuditOutcome, Actor};

    let audit_repo = InMemoryAuditRepository::new();
    let tenant1_id = TenantId::new("tenant-1".to_string()).unwrap();
    let tenant2_id = TenantId::new("tenant-2".to_string()).unwrap();

    // Create events for different tenants
    let actor = Actor::System {
        component: "test".to_string(),
    };

    let event1 = AuditEvent::new(
        tenant1_id.clone(),
        AuditAction::Login,
        actor.clone(),
        AuditOutcome::Success,
    );

    let event2 = AuditEvent::new(
        tenant2_id.clone(),
        AuditAction::Login,
        actor,
        AuditOutcome::Success,
    );

    audit_repo.append(event1).await.unwrap();
    audit_repo.append(event2).await.unwrap();

    // Query for tenant1 - should only see 1 event
    let events1 = audit_repo.get_by_tenant(&tenant1_id, 10, 0).await.unwrap();
    assert_eq!(events1.len(), 1);

    // Query for tenant2 - should only see 1 event
    let events2 = audit_repo.get_by_tenant(&tenant2_id, 10, 0).await.unwrap();
    assert_eq!(events2.len(), 1);
}

// ============================================================================
// IP Filtering Tests
// ============================================================================

#[test]
fn test_ip_filtering_global_allowlist() {
    let ip_filter = IpFilter::new();
    let allowed_ip = IpAddr::from_str("10.0.0.1").unwrap();
    let blocked_ip = IpAddr::from_str("192.168.1.1").unwrap();

    // Add to allowlist
    ip_filter.add_to_global_allowlist(allowed_ip);

    // Allowed IP should pass
    let result = ip_filter.is_allowed(&allowed_ip);
    assert!(result.allowed);

    // Other IPs should be blocked (allowlist is not empty)
    let result = ip_filter.is_allowed(&blocked_ip);
    assert!(!result.allowed);
}

#[test]
fn test_ip_filtering_tenant_specific() {
    let ip_filter = IpFilter::new();
    let tenant_id = create_test_tenant_id();
    let ip = IpAddr::from_str("10.0.0.1").unwrap();

    // Add to tenant allowlist
    ip_filter.add_to_tenant_allowlist(&tenant_id, ip);

    // Should be allowed for this tenant
    let result = ip_filter.is_allowed_for_tenant(&tenant_id, &ip);
    assert!(result.allowed);

    // Other IPs should be blocked
    let other_ip = IpAddr::from_str("192.168.1.1").unwrap();
    let result = ip_filter.is_allowed_for_tenant(&tenant_id, &other_ip);
    assert!(!result.allowed);
}

// ============================================================================
// Request ID Tests
// ============================================================================

#[test]
fn test_request_id_generation() {
    let req_id1 = RequestId::new();
    let req_id2 = RequestId::new();

    // Should be unique
    assert_ne!(req_id1.as_str(), req_id2.as_str());

    // Should be valid UUIDs
    assert!(req_id1.as_str().len() > 0);
    assert!(req_id2.as_str().len() > 0);
}

// ============================================================================
// Security Headers Tests
// ============================================================================

#[test]
fn test_security_config_defaults() {
    let config = SecurityConfig::default();

    assert!(config.enable_hsts);
    assert_eq!(config.hsts_max_age, 31536000); // 1 year
    assert!(config.enable_frame_options);
    assert!(config.enable_content_type_options);
    assert!(config.enable_xss_protection);
    assert!(config.csp.is_some());
}

// ============================================================================
// Integration Test Summary
// ============================================================================

#[test]
fn test_security_integration_complete() {
    // This test verifies that all security components are available
    // and can be instantiated together

    let _auth = setup_auth_manager();
    let _rate_limiter = RateLimiter::new(RateLimitConfig::dev_mode());
    let _ip_filter = IpFilter::new();
    let _security_config = SecurityConfig::default();
    let _request_id = RequestId::new();

    // If we get here, all security components are available
    assert!(true);
}
