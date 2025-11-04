use crate::domain::entities::{Tenant, TenantQuotas, TenantUsage};
use crate::domain::value_objects::TenantId;
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Repository trait for tenant management
///
/// Provides persistent storage and retrieval operations for tenants.
/// All implementations must enforce tenant isolation and data integrity.
///
/// # Responsibilities
/// - CRUD operations for tenants
/// - Tenant activation/deactivation
/// - Quota management
/// - Usage tracking
/// - Tenant querying and filtering
///
/// # Thread Safety
/// Implementations must be thread-safe (Send + Sync).
///
/// # Example
/// ```rust
/// use allsource_core::domain::repositories::TenantRepository;
/// use allsource_core::infrastructure::repositories::InMemoryTenantRepository;
/// use allsource_core::domain::value_objects::TenantId;
/// use allsource_core::domain::entities::TenantQuotas;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let repo = InMemoryTenantRepository::new();
///
///     // Create tenant
///     let tenant_id = TenantId::new("acme-corp".to_string())?;
///     let quotas = TenantQuotas::standard();
///     let tenant = repo.create(tenant_id.clone(), "ACME Corp".to_string(), quotas).await?;
///
///     // Find tenant
///     let found = repo.find_by_id(&tenant_id).await?;
///     assert!(found.is_some());
///
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait TenantRepository: Send + Sync {
    /// Create a new tenant
    ///
    /// # Arguments
    /// * `id` - Unique tenant identifier
    /// * `name` - Human-readable tenant name
    /// * `quotas` - Resource quotas for the tenant
    ///
    /// # Returns
    /// The created tenant
    ///
    /// # Errors
    /// - `TenantAlreadyExists` - If a tenant with this ID already exists
    /// - `ValidationError` - If the name is invalid
    /// - `StorageError` - If the operation fails
    async fn create(&self, id: TenantId, name: String, quotas: TenantQuotas) -> Result<Tenant>;

    /// Save or update a tenant
    ///
    /// If the tenant doesn't exist, it will be created.
    /// If it exists, it will be updated.
    ///
    /// # Arguments
    /// * `tenant` - The tenant to save
    ///
    /// # Errors
    /// - `ValidationError` - If tenant data is invalid
    /// - `StorageError` - If the operation fails
    async fn save(&self, tenant: &Tenant) -> Result<()>;

    /// Find a tenant by ID
    ///
    /// # Arguments
    /// * `id` - The tenant ID to search for
    ///
    /// # Returns
    /// `Some(Tenant)` if found, `None` otherwise
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>>;

    /// Find a tenant by name (case-insensitive)
    ///
    /// # Arguments
    /// * `name` - The tenant name to search for
    ///
    /// # Returns
    /// `Some(Tenant)` if found, `None` otherwise
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn find_by_name(&self, name: &str) -> Result<Option<Tenant>>;

    /// Get all tenants with pagination
    ///
    /// # Arguments
    /// * `limit` - Maximum number of tenants to return
    /// * `offset` - Number of tenants to skip
    ///
    /// # Returns
    /// Vector of tenants, ordered by creation date (newest first)
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>>;

    /// Get active tenants only
    ///
    /// # Arguments
    /// * `limit` - Maximum number of tenants to return
    /// * `offset` - Number of tenants to skip
    ///
    /// # Returns
    /// Vector of active tenants
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn find_active(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>>;

    /// Count total number of tenants
    ///
    /// # Returns
    /// Total number of tenants in the system
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn count(&self) -> Result<usize>;

    /// Count active tenants
    ///
    /// # Returns
    /// Number of active tenants
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn count_active(&self) -> Result<usize>;

    /// Delete a tenant
    ///
    /// # Warning
    /// This is a destructive operation. Consider deactivating instead.
    ///
    /// # Arguments
    /// * `id` - The tenant ID to delete
    ///
    /// # Returns
    /// `true` if the tenant was deleted, `false` if it didn't exist
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn delete(&self, id: &TenantId) -> Result<bool>;

    /// Update tenant quotas
    ///
    /// # Arguments
    /// * `id` - The tenant ID
    /// * `quotas` - New quotas to apply
    ///
    /// # Returns
    /// `true` if updated, `false` if tenant not found
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn update_quotas(&self, id: &TenantId, quotas: TenantQuotas) -> Result<bool>;

    /// Update tenant usage statistics
    ///
    /// # Arguments
    /// * `id` - The tenant ID
    /// * `usage` - New usage statistics
    ///
    /// # Returns
    /// `true` if updated, `false` if tenant not found
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn update_usage(&self, id: &TenantId, usage: TenantUsage) -> Result<bool>;

    /// Activate a tenant
    ///
    /// # Arguments
    /// * `id` - The tenant ID to activate
    ///
    /// # Returns
    /// `true` if activated, `false` if tenant not found
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn activate(&self, id: &TenantId) -> Result<bool>;

    /// Deactivate a tenant
    ///
    /// Deactivated tenants cannot ingest events or perform operations.
    ///
    /// # Arguments
    /// * `id` - The tenant ID to deactivate
    ///
    /// # Returns
    /// `true` if deactivated, `false` if tenant not found
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn deactivate(&self, id: &TenantId) -> Result<bool>;

    /// Check if a tenant exists
    ///
    /// # Arguments
    /// * `id` - The tenant ID to check
    ///
    /// # Returns
    /// `true` if the tenant exists, `false` otherwise
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn exists(&self, id: &TenantId) -> Result<bool> {
        Ok(self.find_by_id(id).await?.is_some())
    }

    /// Check if a tenant is active
    ///
    /// # Arguments
    /// * `id` - The tenant ID to check
    ///
    /// # Returns
    /// `true` if the tenant exists and is active, `false` otherwise
    ///
    /// # Errors
    /// - `StorageError` - If the operation fails
    async fn is_active(&self, id: &TenantId) -> Result<bool> {
        match self.find_by_id(id).await? {
            Some(tenant) => Ok(tenant.is_active()),
            None => Ok(false),
        }
    }
}

/// Query filter for finding tenants
#[derive(Debug, Clone, Default)]
pub struct TenantQuery {
    pub active_only: bool,
    pub name_contains: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl TenantQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_only(mut self) -> Self {
        self.active_only = true;
        self
    }

    pub fn with_name_filter(mut self, name: String) -> Self {
        self.name_contains = Some(name);
        self
    }

    pub fn created_after(mut self, date: DateTime<Utc>) -> Self {
        self.created_after = Some(date);
        self
    }

    pub fn created_before(mut self, date: DateTime<Utc>) -> Self {
        self.created_before = Some(date);
        self
    }

    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_query_builder() {
        let query = TenantQuery::new()
            .active_only()
            .with_name_filter("acme".to_string())
            .with_pagination(10, 0);

        assert!(query.active_only);
        assert_eq!(query.name_contains, Some("acme".to_string()));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_tenant_query_with_dates() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);

        let query = TenantQuery::new()
            .created_after(yesterday)
            .created_before(now);

        assert!(query.created_after.is_some());
        assert!(query.created_before.is_some());
    }
}
