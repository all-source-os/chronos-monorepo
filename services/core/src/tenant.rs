use crate::error::{AllSourceError, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Tenant quotas and limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuotas {
    /// Maximum events per day (0 = unlimited)
    pub max_events_per_day: u64,
    /// Maximum storage in bytes (0 = unlimited)
    pub max_storage_bytes: u64,
    /// Maximum queries per hour (0 = unlimited)
    pub max_queries_per_hour: u64,
    /// Maximum API keys (0 = unlimited)
    pub max_api_keys: u32,
    /// Maximum projections (0 = unlimited)
    pub max_projections: u32,
    /// Maximum pipelines (0 = unlimited)
    pub max_pipelines: u32,
}

impl Default for TenantQuotas {
    fn default() -> Self {
        Self {
            max_events_per_day: 1_000_000,    // 1M events/day
            max_storage_bytes: 10_737_418_240, // 10 GB
            max_queries_per_hour: 100_000,     // 100K queries/hour
            max_api_keys: 10,
            max_projections: 50,
            max_pipelines: 20,
        }
    }
}

impl TenantQuotas {
    /// Unlimited quotas
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
}

/// Tenant usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsage {
    /// Events ingested today
    pub events_today: u64,
    /// Total events stored
    pub total_events: u64,
    /// Storage used in bytes
    pub storage_bytes: u64,
    /// Queries in current hour
    pub queries_this_hour: u64,
    /// Active API keys
    pub active_api_keys: u32,
    /// Active projections
    pub active_projections: u32,
    /// Active pipelines
    pub active_pipelines: u32,
    /// Last reset time for daily counters
    pub last_daily_reset: DateTime<Utc>,
    /// Last reset time for hourly counters
    pub last_hourly_reset: DateTime<Utc>,
}

impl Default for TenantUsage {
    fn default() -> Self {
        Self {
            events_today: 0,
            total_events: 0,
            storage_bytes: 0,
            queries_this_hour: 0,
            active_api_keys: 0,
            active_projections: 0,
            active_pipelines: 0,
            last_daily_reset: Utc::now(),
            last_hourly_reset: Utc::now(),
        }
    }
}

impl TenantUsage {
    /// Reset daily counters if needed
    pub fn reset_daily_if_needed(&mut self) {
        let now = Utc::now();
        let hours_since_reset = (now - self.last_daily_reset).num_hours();

        if hours_since_reset >= 24 {
            self.events_today = 0;
            self.last_daily_reset = now;
        }
    }

    /// Reset hourly counters if needed
    pub fn reset_hourly_if_needed(&mut self) {
        let now = Utc::now();
        let hours_since_reset = (now - self.last_hourly_reset).num_hours();

        if hours_since_reset >= 1 {
            self.queries_this_hour = 0;
            self.last_hourly_reset = now;
        }
    }

    /// Check and reset counters
    pub fn check_and_reset(&mut self) {
        self.reset_daily_if_needed();
        self.reset_hourly_if_needed();
    }
}

/// Tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub quotas: TenantQuotas,
    pub usage: TenantUsage,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
    /// Custom metadata
    pub metadata: serde_json::Value,
}

impl Tenant {
    /// Create new tenant
    pub fn new(id: String, name: String, quotas: TenantQuotas) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description: None,
            quotas,
            usage: TenantUsage::default(),
            created_at: now,
            updated_at: now,
            active: true,
            metadata: serde_json::json!({}),
        }
    }

    /// Check if tenant can ingest more events
    pub fn can_ingest_event(&mut self) -> Result<()> {
        if !self.active {
            return Err(AllSourceError::ValidationError(
                "Tenant is inactive".to_string(),
            ));
        }

        self.usage.check_and_reset();

        if self.quotas.max_events_per_day > 0
            && self.usage.events_today >= self.quotas.max_events_per_day
        {
            return Err(AllSourceError::ValidationError(
                "Daily event quota exceeded".to_string(),
            ));
        }

        if self.quotas.max_storage_bytes > 0
            && self.usage.storage_bytes >= self.quotas.max_storage_bytes
        {
            return Err(AllSourceError::ValidationError(
                "Storage quota exceeded".to_string(),
            ));
        }

        Ok(())
    }

    /// Record event ingestion
    pub fn record_event(&mut self, size_bytes: u64) {
        self.usage.events_today += 1;
        self.usage.total_events += 1;
        self.usage.storage_bytes += size_bytes;
        self.updated_at = Utc::now();
    }

    /// Check if tenant can execute query
    pub fn can_query(&mut self) -> Result<()> {
        if !self.active {
            return Err(AllSourceError::ValidationError(
                "Tenant is inactive".to_string(),
            ));
        }

        self.usage.check_and_reset();

        if self.quotas.max_queries_per_hour > 0
            && self.usage.queries_this_hour >= self.quotas.max_queries_per_hour
        {
            return Err(AllSourceError::ValidationError(
                "Hourly query quota exceeded".to_string(),
            ));
        }

        Ok(())
    }

    /// Record query execution
    pub fn record_query(&mut self) {
        self.usage.queries_this_hour += 1;
        self.updated_at = Utc::now();
    }

    /// Get quota utilization percentage
    pub fn quota_utilization(&self) -> serde_json::Value {
        let events_pct = if self.quotas.max_events_per_day > 0 {
            (self.usage.events_today as f64 / self.quotas.max_events_per_day as f64) * 100.0
        } else {
            0.0
        };

        let storage_pct = if self.quotas.max_storage_bytes > 0 {
            (self.usage.storage_bytes as f64 / self.quotas.max_storage_bytes as f64) * 100.0
        } else {
            0.0
        };

        let queries_pct = if self.quotas.max_queries_per_hour > 0 {
            (self.usage.queries_this_hour as f64 / self.quotas.max_queries_per_hour as f64) * 100.0
        } else {
            0.0
        };

        serde_json::json!({
            "events_today": {
                "used": self.usage.events_today,
                "limit": self.quotas.max_events_per_day,
                "percentage": events_pct.min(100.0)
            },
            "storage": {
                "used_bytes": self.usage.storage_bytes,
                "limit_bytes": self.quotas.max_storage_bytes,
                "percentage": storage_pct.min(100.0)
            },
            "queries_this_hour": {
                "used": self.usage.queries_this_hour,
                "limit": self.quotas.max_queries_per_hour,
                "percentage": queries_pct.min(100.0)
            }
        })
    }
}

/// Tenant manager
pub struct TenantManager {
    tenants: Arc<DashMap<String, Tenant>>,
}

impl TenantManager {
    /// Create new tenant manager
    pub fn new() -> Self {
        let manager = Self {
            tenants: Arc::new(DashMap::new()),
        };

        // Create default tenant
        let default_tenant = Tenant::new(
            "default".to_string(),
            "Default Tenant".to_string(),
            TenantQuotas::unlimited(),
        );
        manager.tenants.insert("default".to_string(), default_tenant);

        manager
    }

    /// Create tenant
    pub fn create_tenant(
        &self,
        id: String,
        name: String,
        quotas: TenantQuotas,
    ) -> Result<Tenant> {
        if self.tenants.contains_key(&id) {
            return Err(AllSourceError::ValidationError(
                "Tenant ID already exists".to_string(),
            ));
        }

        let tenant = Tenant::new(id.clone(), name, quotas);
        self.tenants.insert(id, tenant.clone());

        Ok(tenant)
    }

    /// Get tenant
    pub fn get_tenant(&self, tenant_id: &str) -> Result<Tenant> {
        self.tenants
            .get(tenant_id)
            .map(|t| t.clone())
            .ok_or_else(|| AllSourceError::ValidationError("Tenant not found".to_string()))
    }

    /// Update tenant quotas
    pub fn update_quotas(&self, tenant_id: &str, quotas: TenantQuotas) -> Result<()> {
        if let Some(mut tenant) = self.tenants.get_mut(tenant_id) {
            tenant.quotas = quotas;
            tenant.updated_at = Utc::now();
            Ok(())
        } else {
            Err(AllSourceError::ValidationError(
                "Tenant not found".to_string(),
            ))
        }
    }

    /// Deactivate tenant
    pub fn deactivate_tenant(&self, tenant_id: &str) -> Result<()> {
        if tenant_id == "default" {
            return Err(AllSourceError::ValidationError(
                "Cannot deactivate default tenant".to_string(),
            ));
        }

        if let Some(mut tenant) = self.tenants.get_mut(tenant_id) {
            tenant.active = false;
            tenant.updated_at = Utc::now();
            Ok(())
        } else {
            Err(AllSourceError::ValidationError(
                "Tenant not found".to_string(),
            ))
        }
    }

    /// Activate tenant
    pub fn activate_tenant(&self, tenant_id: &str) -> Result<()> {
        if let Some(mut tenant) = self.tenants.get_mut(tenant_id) {
            tenant.active = true;
            tenant.updated_at = Utc::now();
            Ok(())
        } else {
            Err(AllSourceError::ValidationError(
                "Tenant not found".to_string(),
            ))
        }
    }

    /// Delete tenant
    pub fn delete_tenant(&self, tenant_id: &str) -> Result<()> {
        if tenant_id == "default" {
            return Err(AllSourceError::ValidationError(
                "Cannot delete default tenant".to_string(),
            ));
        }

        self.tenants
            .remove(tenant_id)
            .ok_or_else(|| AllSourceError::ValidationError("Tenant not found".to_string()))?;

        Ok(())
    }

    /// List all tenants
    pub fn list_tenants(&self) -> Vec<Tenant> {
        self.tenants.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Check if tenant can ingest event
    pub fn check_can_ingest(&self, tenant_id: &str) -> Result<()> {
        if let Some(mut tenant) = self.tenants.get_mut(tenant_id) {
            tenant.can_ingest_event()
        } else {
            Err(AllSourceError::ValidationError(
                "Tenant not found".to_string(),
            ))
        }
    }

    /// Record event ingestion
    pub fn record_ingestion(&self, tenant_id: &str, size_bytes: u64) -> Result<()> {
        if let Some(mut tenant) = self.tenants.get_mut(tenant_id) {
            tenant.record_event(size_bytes);
            Ok(())
        } else {
            Err(AllSourceError::ValidationError(
                "Tenant not found".to_string(),
            ))
        }
    }

    /// Check if tenant can query
    pub fn check_can_query(&self, tenant_id: &str) -> Result<()> {
        if let Some(mut tenant) = self.tenants.get_mut(tenant_id) {
            tenant.can_query()
        } else {
            Err(AllSourceError::ValidationError(
                "Tenant not found".to_string(),
            ))
        }
    }

    /// Record query execution
    pub fn record_query(&self, tenant_id: &str) -> Result<()> {
        if let Some(mut tenant) = self.tenants.get_mut(tenant_id) {
            tenant.record_query();
            Ok(())
        } else {
            Err(AllSourceError::ValidationError(
                "Tenant not found".to_string(),
            ))
        }
    }

    /// Get tenant statistics
    pub fn get_stats(&self, tenant_id: &str) -> Result<serde_json::Value> {
        let tenant = self.get_tenant(tenant_id)?;

        Ok(serde_json::json!({
            "tenant_id": tenant.id,
            "name": tenant.name,
            "active": tenant.active,
            "usage": tenant.usage,
            "quotas": tenant.quotas,
            "utilization": tenant.quota_utilization(),
            "created_at": tenant.created_at,
            "updated_at": tenant.updated_at
        }))
    }
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let manager = TenantManager::new();

        let tenant = manager
            .create_tenant(
                "test-tenant".to_string(),
                "Test Tenant".to_string(),
                TenantQuotas::default(),
            )
            .unwrap();

        assert_eq!(tenant.id, "test-tenant");
        assert_eq!(tenant.name, "Test Tenant");
        assert!(tenant.active);
    }

    #[test]
    fn test_quota_enforcement() {
        let mut tenant = Tenant::new(
            "test".to_string(),
            "Test".to_string(),
            TenantQuotas {
                max_events_per_day: 10,
                max_storage_bytes: 1000,
                ..Default::default()
            },
        );

        // Should allow first 10 events
        for _ in 0..10 {
            assert!(tenant.can_ingest_event().is_ok());
            tenant.record_event(50);
        }

        // Should reject 11th event (quota exceeded)
        assert!(tenant.can_ingest_event().is_err());
    }

    #[test]
    fn test_tenant_deactivation() {
        let manager = TenantManager::new();

        manager
            .create_tenant(
                "test".to_string(),
                "Test".to_string(),
                TenantQuotas::default(),
            )
            .unwrap();

        manager.deactivate_tenant("test").unwrap();

        let tenant = manager.get_tenant("test").unwrap();
        assert!(!tenant.active);
    }

    #[test]
    fn test_quota_utilization() {
        let mut tenant = Tenant::new(
            "test".to_string(),
            "Test".to_string(),
            TenantQuotas {
                max_events_per_day: 100,
                max_storage_bytes: 1000,
                max_queries_per_hour: 50,
                ..Default::default()
            },
        );

        tenant.record_event(500); // 50% storage
        tenant.record_event(250); // 75% storage, 2% events

        let util = tenant.quota_utilization();
        assert_eq!(util["events_today"]["used"], 2);
        assert_eq!(util["storage"]["used_bytes"], 750);
    }
}
