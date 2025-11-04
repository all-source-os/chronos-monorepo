use crate::domain::value_objects::TenantId;
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Value Object: Tenant Quotas
///
/// Represents the resource limits for a tenant.
/// All quotas are immutable once set (change requires creating new quotas).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantQuotas {
    /// Maximum events per day (0 = unlimited)
    max_events_per_day: u64,
    /// Maximum storage in bytes (0 = unlimited)
    max_storage_bytes: u64,
    /// Maximum queries per hour (0 = unlimited)
    max_queries_per_hour: u64,
    /// Maximum API keys (0 = unlimited)
    max_api_keys: u32,
    /// Maximum projections (0 = unlimited)
    max_projections: u32,
    /// Maximum pipelines (0 = unlimited)
    max_pipelines: u32,
}

impl TenantQuotas {
    /// Create custom quotas
    pub fn new(
        max_events_per_day: u64,
        max_storage_bytes: u64,
        max_queries_per_hour: u64,
        max_api_keys: u32,
        max_projections: u32,
        max_pipelines: u32,
    ) -> Self {
        Self {
            max_events_per_day,
            max_storage_bytes,
            max_queries_per_hour,
            max_api_keys,
            max_projections,
            max_pipelines,
        }
    }

    /// Default quotas (standard tier)
    pub fn standard() -> Self {
        Self {
            max_events_per_day: 1_000_000,
            max_storage_bytes: 10_737_418_240, // 10 GB
            max_queries_per_hour: 100_000,
            max_api_keys: 10,
            max_projections: 50,
            max_pipelines: 20,
        }
    }

    /// Free tier quotas
    pub fn free_tier() -> Self {
        Self {
            max_events_per_day: 10_000,
            max_storage_bytes: 1_073_741_824, // 1 GB
            max_queries_per_hour: 1_000,
            max_api_keys: 2,
            max_projections: 5,
            max_pipelines: 2,
        }
    }

    /// Professional tier quotas
    pub fn professional() -> Self {
        Self {
            max_events_per_day: 1_000_000,
            max_storage_bytes: 107_374_182_400, // 100 GB
            max_queries_per_hour: 100_000,
            max_api_keys: 25,
            max_projections: 100,
            max_pipelines: 50,
        }
    }

    /// Unlimited quotas (enterprise tier)
    pub fn unlimited() -> Self {
        Self {
            max_events_per_day: 0,
            max_storage_bytes: 0,
            max_queries_per_hour: 0,
            max_api_keys: 0,
            max_projections: 0,
            max_pipelines: 0,
        }
    }

    // Getters
    pub fn max_events_per_day(&self) -> u64 {
        self.max_events_per_day
    }

    pub fn max_storage_bytes(&self) -> u64 {
        self.max_storage_bytes
    }

    pub fn max_queries_per_hour(&self) -> u64 {
        self.max_queries_per_hour
    }

    pub fn max_api_keys(&self) -> u32 {
        self.max_api_keys
    }

    pub fn max_projections(&self) -> u32 {
        self.max_projections
    }

    pub fn max_pipelines(&self) -> u32 {
        self.max_pipelines
    }

    /// Check if a resource is unlimited
    pub fn is_unlimited(&self, resource: QuotaResource) -> bool {
        match resource {
            QuotaResource::EventsPerDay => self.max_events_per_day == 0,
            QuotaResource::StorageBytes => self.max_storage_bytes == 0,
            QuotaResource::QueriesPerHour => self.max_queries_per_hour == 0,
            QuotaResource::ApiKeys => self.max_api_keys == 0,
            QuotaResource::Projections => self.max_projections == 0,
            QuotaResource::Pipelines => self.max_pipelines == 0,
        }
    }
}

impl Default for TenantQuotas {
    fn default() -> Self {
        Self::standard()
    }
}

/// Quota resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaResource {
    EventsPerDay,
    StorageBytes,
    QueriesPerHour,
    ApiKeys,
    Projections,
    Pipelines,
}

/// Tenant usage statistics
///
/// Tracks current resource usage for quota enforcement.
/// This is mutable state that changes as the tenant uses resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsage {
    events_today: u64,
    total_events: u64,
    storage_bytes: u64,
    queries_this_hour: u64,
    active_api_keys: u32,
    active_projections: u32,
    active_pipelines: u32,
    last_daily_reset: DateTime<Utc>,
    last_hourly_reset: DateTime<Utc>,
}

impl TenantUsage {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            events_today: 0,
            total_events: 0,
            storage_bytes: 0,
            queries_this_hour: 0,
            active_api_keys: 0,
            active_projections: 0,
            active_pipelines: 0,
            last_daily_reset: now,
            last_hourly_reset: now,
        }
    }

    // Getters
    pub fn events_today(&self) -> u64 {
        self.events_today
    }

    pub fn total_events(&self) -> u64 {
        self.total_events
    }

    pub fn storage_bytes(&self) -> u64 {
        self.storage_bytes
    }

    pub fn queries_this_hour(&self) -> u64 {
        self.queries_this_hour
    }

    pub fn active_api_keys(&self) -> u32 {
        self.active_api_keys
    }

    pub fn active_projections(&self) -> u32 {
        self.active_projections
    }

    pub fn active_pipelines(&self) -> u32 {
        self.active_pipelines
    }

    /// Increment event counter
    pub fn record_event(&mut self) {
        self.events_today += 1;
        self.total_events += 1;
    }

    /// Add storage usage
    pub fn add_storage(&mut self, bytes: u64) {
        self.storage_bytes += bytes;
    }

    /// Remove storage usage
    pub fn remove_storage(&mut self, bytes: u64) {
        self.storage_bytes = self.storage_bytes.saturating_sub(bytes);
    }

    /// Increment query counter
    pub fn record_query(&mut self) {
        self.queries_this_hour += 1;
    }

    /// Increment API key counter
    pub fn increment_api_keys(&mut self) {
        self.active_api_keys += 1;
    }

    /// Decrement API key counter
    pub fn decrement_api_keys(&mut self) {
        self.active_api_keys = self.active_api_keys.saturating_sub(1);
    }

    /// Increment projection counter
    pub fn increment_projections(&mut self) {
        self.active_projections += 1;
    }

    /// Decrement projection counter
    pub fn decrement_projections(&mut self) {
        self.active_projections = self.active_projections.saturating_sub(1);
    }

    /// Increment pipeline counter
    pub fn increment_pipelines(&mut self) {
        self.active_pipelines += 1;
    }

    /// Decrement pipeline counter
    pub fn decrement_pipelines(&mut self) {
        self.active_pipelines = self.active_pipelines.saturating_sub(1);
    }

    /// Reset daily counters if a day has passed
    pub fn reset_daily_if_needed(&mut self) {
        let now = Utc::now();
        let hours_since_reset = (now - self.last_daily_reset).num_hours();

        if hours_since_reset >= 24 {
            self.events_today = 0;
            self.last_daily_reset = now;
        }
    }

    /// Reset hourly counters if an hour has passed
    pub fn reset_hourly_if_needed(&mut self) {
        let now = Utc::now();
        let hours_since_reset = (now - self.last_hourly_reset).num_hours();

        if hours_since_reset >= 1 {
            self.queries_this_hour = 0;
            self.last_hourly_reset = now;
        }
    }

    /// Check and reset all time-based counters
    pub fn check_and_reset(&mut self) {
        self.reset_daily_if_needed();
        self.reset_hourly_if_needed();
    }
}

impl Default for TenantUsage {
    fn default() -> Self {
        Self::new()
    }
}

/// Domain Entity: Tenant
///
/// Represents a tenant in the multi-tenant event sourcing system.
/// Tenants are isolated units with their own events, quotas, and usage tracking.
///
/// Domain Rules:
/// - Tenant ID must be unique and valid
/// - Name cannot be empty
/// - Inactive tenants cannot ingest events or perform operations
/// - Quota limits must be enforced before operations
/// - Usage counters must be accurate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    id: TenantId,
    name: String,
    description: Option<String>,
    quotas: TenantQuotas,
    usage: TenantUsage,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    active: bool,
    metadata: serde_json::Value,
}

impl Tenant {
    /// Create a new tenant with validation
    pub fn new(id: TenantId, name: String, quotas: TenantQuotas) -> Result<Self> {
        Self::validate_name(&name)?;

        let now = Utc::now();
        Ok(Self {
            id,
            name,
            description: None,
            quotas,
            usage: TenantUsage::new(),
            created_at: now,
            updated_at: now,
            active: true,
            metadata: serde_json::json!({}),
        })
    }

    /// Reconstruct tenant from storage (bypasses validation)
    pub fn reconstruct(
        id: TenantId,
        name: String,
        description: Option<String>,
        quotas: TenantQuotas,
        usage: TenantUsage,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        active: bool,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            name,
            description,
            quotas,
            usage,
            created_at,
            updated_at,
            active,
            metadata,
        }
    }

    // Getters
    pub fn id(&self) -> &TenantId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn quotas(&self) -> &TenantQuotas {
        &self.quotas
    }

    pub fn usage(&self) -> &TenantUsage {
        &self.usage
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    // Domain behavior methods

    /// Update tenant name
    pub fn update_name(&mut self, name: String) -> Result<()> {
        Self::validate_name(&name)?;
        self.name = name;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update description
    pub fn update_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    /// Update quotas
    pub fn update_quotas(&mut self, quotas: TenantQuotas) {
        self.quotas = quotas;
        self.updated_at = Utc::now();
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }

    /// Activate tenant
    pub fn activate(&mut self) {
        self.active = true;
        self.updated_at = Utc::now();
    }

    /// Deactivate tenant
    pub fn deactivate(&mut self) {
        self.active = false;
        self.updated_at = Utc::now();
    }

    /// Check if tenant can ingest an event
    pub fn can_ingest_event(&mut self) -> Result<()> {
        if !self.active {
            return Err(crate::error::AllSourceError::ValidationError(
                "Tenant is inactive".to_string(),
            ));
        }

        self.usage.check_and_reset();

        // Check daily event quota
        if !self.quotas.is_unlimited(QuotaResource::EventsPerDay)
            && self.usage.events_today() >= self.quotas.max_events_per_day()
        {
            return Err(crate::error::AllSourceError::ValidationError(
                format!(
                    "Daily event quota exceeded: {}/{}",
                    self.usage.events_today(),
                    self.quotas.max_events_per_day()
                ),
            ));
        }

        // Check storage quota
        if !self.quotas.is_unlimited(QuotaResource::StorageBytes)
            && self.usage.storage_bytes() >= self.quotas.max_storage_bytes()
        {
            return Err(crate::error::AllSourceError::ValidationError(
                format!(
                    "Storage quota exceeded: {}/{}",
                    self.usage.storage_bytes(),
                    self.quotas.max_storage_bytes()
                ),
            ));
        }

        Ok(())
    }

    /// Check if tenant can execute a query
    pub fn can_execute_query(&mut self) -> Result<()> {
        if !self.active {
            return Err(crate::error::AllSourceError::ValidationError(
                "Tenant is inactive".to_string(),
            ));
        }

        self.usage.check_and_reset();

        // Check hourly query quota
        if !self.quotas.is_unlimited(QuotaResource::QueriesPerHour)
            && self.usage.queries_this_hour() >= self.quotas.max_queries_per_hour()
        {
            return Err(crate::error::AllSourceError::ValidationError(
                format!(
                    "Hourly query quota exceeded: {}/{}",
                    self.usage.queries_this_hour(),
                    self.quotas.max_queries_per_hour()
                ),
            ));
        }

        Ok(())
    }

    /// Check if tenant can create an API key
    pub fn can_create_api_key(&self) -> Result<()> {
        if !self.active {
            return Err(crate::error::AllSourceError::ValidationError(
                "Tenant is inactive".to_string(),
            ));
        }

        if !self.quotas.is_unlimited(QuotaResource::ApiKeys)
            && self.usage.active_api_keys() >= self.quotas.max_api_keys()
        {
            return Err(crate::error::AllSourceError::ValidationError(
                format!(
                    "API key quota exceeded: {}/{}",
                    self.usage.active_api_keys(),
                    self.quotas.max_api_keys()
                ),
            ));
        }

        Ok(())
    }

    /// Check if tenant can create a projection
    pub fn can_create_projection(&self) -> Result<()> {
        if !self.active {
            return Err(crate::error::AllSourceError::ValidationError(
                "Tenant is inactive".to_string(),
            ));
        }

        if !self.quotas.is_unlimited(QuotaResource::Projections)
            && self.usage.active_projections() >= self.quotas.max_projections()
        {
            return Err(crate::error::AllSourceError::ValidationError(
                format!(
                    "Projection quota exceeded: {}/{}",
                    self.usage.active_projections(),
                    self.quotas.max_projections()
                ),
            ));
        }

        Ok(())
    }

    /// Check if tenant can create a pipeline
    pub fn can_create_pipeline(&self) -> Result<()> {
        if !self.active {
            return Err(crate::error::AllSourceError::ValidationError(
                "Tenant is inactive".to_string(),
            ));
        }

        if !self.quotas.is_unlimited(QuotaResource::Pipelines)
            && self.usage.active_pipelines() >= self.quotas.max_pipelines()
        {
            return Err(crate::error::AllSourceError::ValidationError(
                format!(
                    "Pipeline quota exceeded: {}/{}",
                    self.usage.active_pipelines(),
                    self.quotas.max_pipelines()
                ),
            ));
        }

        Ok(())
    }

    /// Get mutable access to usage (for updating counters)
    pub fn usage_mut(&mut self) -> &mut TenantUsage {
        self.updated_at = Utc::now();
        &mut self.usage
    }

    // Validation

    fn validate_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Tenant name cannot be empty".to_string(),
            ));
        }

        if name.len() > 100 {
            return Err(crate::error::AllSourceError::InvalidInput(
                format!("Tenant name cannot exceed 100 characters, got {}", name.len()),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    #[test]
    fn test_create_tenant() {
        let tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::standard(),
        );

        assert!(tenant.is_ok());
        let tenant = tenant.unwrap();
        assert_eq!(tenant.name(), "Test Tenant");
        assert!(tenant.is_active());
        assert_eq!(tenant.usage().total_events(), 0);
    }

    #[test]
    fn test_reject_empty_name() {
        let result = Tenant::new(
            test_tenant_id(),
            "".to_string(),
            TenantQuotas::standard(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_too_long_name() {
        let long_name = "a".repeat(101);
        let result = Tenant::new(
            test_tenant_id(),
            long_name,
            TenantQuotas::standard(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_update_name() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Original Name".to_string(),
            TenantQuotas::standard(),
        )
        .unwrap();

        let result = tenant.update_name("New Name".to_string());
        assert!(result.is_ok());
        assert_eq!(tenant.name(), "New Name");
    }

    #[test]
    fn test_activate_deactivate() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::standard(),
        )
        .unwrap();

        assert!(tenant.is_active());

        tenant.deactivate();
        assert!(!tenant.is_active());

        tenant.activate();
        assert!(tenant.is_active());
    }

    #[test]
    fn test_can_ingest_event_when_active() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::unlimited(),
        )
        .unwrap();

        let result = tenant.can_ingest_event();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cannot_ingest_event_when_inactive() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::unlimited(),
        )
        .unwrap();

        tenant.deactivate();
        let result = tenant.can_ingest_event();
        assert!(result.is_err());
    }

    #[test]
    fn test_daily_quota_exceeded() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::new(10, 0, 0, 0, 0, 0), // Max 10 events per day
        )
        .unwrap();

        // Ingest 10 events (should work)
        for _ in 0..10 {
            assert!(tenant.can_ingest_event().is_ok());
            tenant.usage_mut().record_event();
        }

        // 11th event should fail
        let result = tenant.can_ingest_event();
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_quota_exceeded() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::new(0, 1000, 0, 0, 0, 0), // Max 1000 bytes storage
        )
        .unwrap();

        // Add 1000 bytes (should work)
        tenant.usage_mut().add_storage(1000);
        let result = tenant.can_ingest_event();
        assert!(result.is_err());
    }

    #[test]
    fn test_query_quota_exceeded() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::new(0, 0, 5, 0, 0, 0), // Max 5 queries per hour
        )
        .unwrap();

        // Execute 5 queries (should work)
        for _ in 0..5 {
            assert!(tenant.can_execute_query().is_ok());
            tenant.usage_mut().record_query();
        }

        // 6th query should fail
        let result = tenant.can_execute_query();
        assert!(result.is_err());
    }

    #[test]
    fn test_api_key_quota() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::new(0, 0, 0, 2, 0, 0), // Max 2 API keys
        )
        .unwrap();

        assert!(tenant.can_create_api_key().is_ok());
        tenant.usage_mut().increment_api_keys();

        assert!(tenant.can_create_api_key().is_ok());
        tenant.usage_mut().increment_api_keys();

        // 3rd API key should fail
        assert!(tenant.can_create_api_key().is_err());
    }

    #[test]
    fn test_projection_quota() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::new(0, 0, 0, 0, 3, 0), // Max 3 projections
        )
        .unwrap();

        for _ in 0..3 {
            assert!(tenant.can_create_projection().is_ok());
            tenant.usage_mut().increment_projections();
        }

        assert!(tenant.can_create_projection().is_err());
    }

    #[test]
    fn test_pipeline_quota() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::new(0, 0, 0, 0, 0, 2), // Max 2 pipelines
        )
        .unwrap();

        for _ in 0..2 {
            assert!(tenant.can_create_pipeline().is_ok());
            tenant.usage_mut().increment_pipelines();
        }

        assert!(tenant.can_create_pipeline().is_err());
    }

    #[test]
    fn test_unlimited_quotas() {
        let mut tenant = Tenant::new(
            test_tenant_id(),
            "Test Tenant".to_string(),
            TenantQuotas::unlimited(),
        )
        .unwrap();

        // Should never fail with unlimited quotas
        for _ in 0..1000 {
            tenant.usage_mut().record_event();
            tenant.usage_mut().add_storage(10000);
            tenant.usage_mut().record_query();
        }

        assert!(tenant.can_ingest_event().is_ok());
        assert!(tenant.can_execute_query().is_ok());
    }
}
