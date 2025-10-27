/// PostgreSQL Tenant Repository
///
/// Production-grade persistent tenant management using PostgreSQL.
/// Provides ACID guarantees, complex queries, and long-term storage.

#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use sqlx::{PgPool, Row};
#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use serde_json::Value as JsonValue;

#[cfg(feature = "postgres")]
use crate::domain::entities::{Tenant, TenantQuotas, TenantUsage};
#[cfg(feature = "postgres")]
use crate::domain::repositories::TenantRepository;
#[cfg(feature = "postgres")]
use crate::domain::value_objects::TenantId;
#[cfg(feature = "postgres")]
use crate::error::{AllSourceError, Result};

#[cfg(feature = "postgres")]
/// PostgreSQL tenant repository
pub struct PostgresTenantRepository {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresTenantRepository {
    /// Create new PostgreSQL tenant repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run migrations (creates tenants table)
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| AllSourceError::StorageError(format!("Migration failed: {}", e)))?;
        Ok(())
    }

    /// Helper: Convert database row to Tenant
    fn row_to_tenant(row: &sqlx::postgres::PgRow) -> Result<Tenant> {
        // Extract ID
        let id_str: String = row.try_get("id")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get id: {}", e)))?;
        let id = TenantId::new(id_str)?;

        // Extract basic fields
        let name: String = row.try_get("name")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get name: {}", e)))?;
        let description: Option<String> = row.try_get("description").ok();
        let active: bool = row.try_get("active")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get active: {}", e)))?;
        let metadata: JsonValue = row.try_get("metadata")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get metadata: {}", e)))?;

        // Extract timestamps
        let created_at: DateTime<Utc> = row.try_get("created_at")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get created_at: {}", e)))?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get updated_at: {}", e)))?;

        // Extract quotas
        let quota_max_events_per_day: i64 = row.try_get("quota_max_events_per_day")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get quota_max_events_per_day: {}", e)))?;
        let quota_max_storage_bytes: i64 = row.try_get("quota_max_storage_bytes")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get quota_max_storage_bytes: {}", e)))?;
        let quota_max_queries_per_hour: i64 = row.try_get("quota_max_queries_per_hour")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get quota_max_queries_per_hour: {}", e)))?;
        let quota_max_api_keys: i32 = row.try_get("quota_max_api_keys")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get quota_max_api_keys: {}", e)))?;
        let quota_max_projections: i32 = row.try_get("quota_max_projections")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get quota_max_projections: {}", e)))?;
        let quota_max_pipelines: i32 = row.try_get("quota_max_pipelines")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get quota_max_pipelines: {}", e)))?;

        let quotas = TenantQuotas::new(
            quota_max_events_per_day as u64,
            quota_max_storage_bytes as u64,
            quota_max_queries_per_hour as u64,
            quota_max_api_keys as u32,
            quota_max_projections as u32,
            quota_max_pipelines as u32,
        );

        // Extract usage - we need to reconstruct TenantUsage from individual fields
        // Since TenantUsage doesn't have a reconstruct method, we'll need to use a different approach
        // Let's serialize the usage fields into JSON and deserialize into TenantUsage
        let usage_events_today: i64 = row.try_get("usage_events_today")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get usage_events_today: {}", e)))?;
        let usage_total_events: i64 = row.try_get("usage_total_events")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get usage_total_events: {}", e)))?;
        let usage_storage_bytes: i64 = row.try_get("usage_storage_bytes")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get usage_storage_bytes: {}", e)))?;
        let usage_queries_this_hour: i64 = row.try_get("usage_queries_this_hour")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get usage_queries_this_hour: {}", e)))?;
        let usage_active_api_keys: i32 = row.try_get("usage_active_api_keys")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get usage_active_api_keys: {}", e)))?;
        let usage_active_projections: i32 = row.try_get("usage_active_projections")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get usage_active_projections: {}", e)))?;
        let usage_active_pipelines: i32 = row.try_get("usage_active_pipelines")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get usage_active_pipelines: {}", e)))?;
        let usage_last_daily_reset: DateTime<Utc> = row.try_get("usage_last_daily_reset")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get usage_last_daily_reset: {}", e)))?;
        let usage_last_hourly_reset: DateTime<Utc> = row.try_get("usage_last_hourly_reset")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get usage_last_hourly_reset: {}", e)))?;

        // Create a JSON object to deserialize into TenantUsage
        let usage_json = serde_json::json!({
            "events_today": usage_events_today as u64,
            "total_events": usage_total_events as u64,
            "storage_bytes": usage_storage_bytes as u64,
            "queries_this_hour": usage_queries_this_hour as u64,
            "active_api_keys": usage_active_api_keys as u32,
            "active_projections": usage_active_projections as u32,
            "active_pipelines": usage_active_pipelines as u32,
            "last_daily_reset": usage_last_daily_reset,
            "last_hourly_reset": usage_last_hourly_reset,
        });

        let usage: TenantUsage = serde_json::from_value(usage_json)
            .map_err(|e| AllSourceError::StorageError(format!("Failed to deserialize usage: {}", e)))?;

        // Reconstruct tenant
        Ok(Tenant::reconstruct(
            id,
            name,
            description,
            quotas,
            usage,
            created_at,
            updated_at,
            active,
            metadata,
        ))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl TenantRepository for PostgresTenantRepository {
    async fn create(&self, id: TenantId, name: String, quotas: TenantQuotas) -> Result<Tenant> {
        // Check if tenant already exists
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tenants WHERE id = $1)"
        )
        .bind(id.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to check tenant existence: {}", e)))?;

        if exists {
            return Err(AllSourceError::TenantAlreadyExists(id.as_str().to_string()));
        }

        // Create tenant entity (validates name)
        let tenant = Tenant::new(id, name, quotas)?;

        // Insert into database
        sqlx::query(
            r#"
            INSERT INTO tenants (
                id, name, description,
                quota_max_events_per_day, quota_max_storage_bytes, quota_max_queries_per_hour,
                quota_max_api_keys, quota_max_projections, quota_max_pipelines,
                usage_events_today, usage_total_events, usage_storage_bytes, usage_queries_this_hour,
                usage_active_api_keys, usage_active_projections, usage_active_pipelines,
                usage_last_daily_reset, usage_last_hourly_reset,
                active, metadata, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
            "#,
        )
        .bind(tenant.id().as_str())
        .bind(tenant.name())
        .bind(tenant.description())
        .bind(tenant.quotas().max_events_per_day() as i64)
        .bind(tenant.quotas().max_storage_bytes() as i64)
        .bind(tenant.quotas().max_queries_per_hour() as i64)
        .bind(tenant.quotas().max_api_keys() as i32)
        .bind(tenant.quotas().max_projections() as i32)
        .bind(tenant.quotas().max_pipelines() as i32)
        .bind(tenant.usage().events_today() as i64)
        .bind(tenant.usage().total_events() as i64)
        .bind(tenant.usage().storage_bytes() as i64)
        .bind(tenant.usage().queries_this_hour() as i64)
        .bind(tenant.usage().active_api_keys() as i32)
        .bind(tenant.usage().active_projections() as i32)
        .bind(tenant.usage().active_pipelines() as i32)
        .bind(tenant.created_at())
        .bind(tenant.created_at())
        .bind(tenant.is_active())
        .bind(tenant.metadata())
        .bind(tenant.created_at())
        .bind(tenant.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to create tenant: {}", e)))?;

        Ok(tenant)
    }

    async fn save(&self, tenant: &Tenant) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tenants SET
                name = $2,
                description = $3,
                quota_max_events_per_day = $4,
                quota_max_storage_bytes = $5,
                quota_max_queries_per_hour = $6,
                quota_max_api_keys = $7,
                quota_max_projections = $8,
                quota_max_pipelines = $9,
                usage_events_today = $10,
                usage_total_events = $11,
                usage_storage_bytes = $12,
                usage_queries_this_hour = $13,
                usage_active_api_keys = $14,
                usage_active_projections = $15,
                usage_active_pipelines = $16,
                usage_last_daily_reset = $17,
                usage_last_hourly_reset = $18,
                active = $19,
                metadata = $20,
                updated_at = $21
            WHERE id = $1
            "#,
        )
        .bind(tenant.id().as_str())
        .bind(tenant.name())
        .bind(tenant.description())
        .bind(tenant.quotas().max_events_per_day() as i64)
        .bind(tenant.quotas().max_storage_bytes() as i64)
        .bind(tenant.quotas().max_queries_per_hour() as i64)
        .bind(tenant.quotas().max_api_keys() as i32)
        .bind(tenant.quotas().max_projections() as i32)
        .bind(tenant.quotas().max_pipelines() as i32)
        .bind(tenant.usage().events_today() as i64)
        .bind(tenant.usage().total_events() as i64)
        .bind(tenant.usage().storage_bytes() as i64)
        .bind(tenant.usage().queries_this_hour() as i64)
        .bind(tenant.usage().active_api_keys() as i32)
        .bind(tenant.usage().active_projections() as i32)
        .bind(tenant.usage().active_pipelines() as i32)
        .bind(tenant.created_at())
        .bind(tenant.created_at())
        .bind(tenant.is_active())
        .bind(tenant.metadata())
        .bind(tenant.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to save tenant: {}", e)))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>> {
        let row = sqlx::query(
            "SELECT * FROM tenants WHERE id = $1"
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to find tenant by id: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_tenant(&r)?)),
            None => Ok(None),
        }
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Tenant>> {
        let row = sqlx::query(
            "SELECT * FROM tenants WHERE LOWER(name) = LOWER($1) LIMIT 1"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to find tenant by name: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_tenant(&r)?)),
            None => Ok(None),
        }
    }

    async fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        let rows = sqlx::query(
            "SELECT * FROM tenants ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to find all tenants: {}", e)))?;

        let mut tenants = Vec::new();
        for row in rows {
            tenants.push(Self::row_to_tenant(&row)?);
        }

        Ok(tenants)
    }

    async fn find_active(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        let rows = sqlx::query(
            "SELECT * FROM tenants WHERE active = TRUE ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to find active tenants: {}", e)))?;

        let mut tenants = Vec::new();
        for row in rows {
            tenants.push(Self::row_to_tenant(&row)?);
        }

        Ok(tenants)
    }

    async fn count(&self) -> Result<usize> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tenants"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to count tenants: {}", e)))?;

        Ok(count as usize)
    }

    async fn count_active(&self) -> Result<usize> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tenants WHERE active = TRUE"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to count active tenants: {}", e)))?;

        Ok(count as usize)
    }

    async fn delete(&self, id: &TenantId) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM tenants WHERE id = $1"
        )
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to delete tenant: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn update_quotas(&self, id: &TenantId, quotas: TenantQuotas) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE tenants SET
                quota_max_events_per_day = $2,
                quota_max_storage_bytes = $3,
                quota_max_queries_per_hour = $4,
                quota_max_api_keys = $5,
                quota_max_projections = $6,
                quota_max_pipelines = $7,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id.as_str())
        .bind(quotas.max_events_per_day() as i64)
        .bind(quotas.max_storage_bytes() as i64)
        .bind(quotas.max_queries_per_hour() as i64)
        .bind(quotas.max_api_keys() as i32)
        .bind(quotas.max_projections() as i32)
        .bind(quotas.max_pipelines() as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to update quotas: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn update_usage(&self, id: &TenantId, usage: TenantUsage) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE tenants SET
                usage_events_today = $2,
                usage_total_events = $3,
                usage_storage_bytes = $4,
                usage_queries_this_hour = $5,
                usage_active_api_keys = $6,
                usage_active_projections = $7,
                usage_active_pipelines = $8,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id.as_str())
        .bind(usage.events_today() as i64)
        .bind(usage.total_events() as i64)
        .bind(usage.storage_bytes() as i64)
        .bind(usage.queries_this_hour() as i64)
        .bind(usage.active_api_keys() as i32)
        .bind(usage.active_projections() as i32)
        .bind(usage.active_pipelines() as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to update usage: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn activate(&self, id: &TenantId) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE tenants SET active = TRUE, updated_at = NOW() WHERE id = $1"
        )
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to activate tenant: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    async fn deactivate(&self, id: &TenantId) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE tenants SET active = FALSE, updated_at = NOW() WHERE id = $1"
        )
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to deactivate tenant: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;

    // Note: These tests require a running PostgreSQL database
    // Run with: cargo test --features postgres

    #[tokio::test]
    #[ignore] // Requires PostgreSQL
    async fn test_postgres_tenant_repository() {
        // This test would require a test database
        // In production, use testcontainers or similar
    }
}
