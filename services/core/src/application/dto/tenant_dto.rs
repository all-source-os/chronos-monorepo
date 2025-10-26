use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::domain::entities::{Tenant, TenantQuotas};

/// DTO for creating a new tenant
#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub tenant_id: String,
    pub name: String,
    pub quotas: Option<TenantQuotasDto>,
}

/// DTO for updating a tenant
#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub quotas: Option<TenantQuotasDto>,
}

/// DTO for tenant quotas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuotasDto {
    pub max_events_per_day: Option<usize>,
    pub max_storage_bytes: Option<u64>,
    pub max_queries_per_hour: Option<usize>,
    pub max_api_keys: Option<usize>,
    pub max_projections: Option<usize>,
    pub max_pipelines: Option<usize>,
}

impl From<TenantQuotas> for TenantQuotasDto {
    fn from(quotas: TenantQuotas) -> Self {
        Self {
            max_events_per_day: if quotas.max_events_per_day() == 0 {
                None
            } else {
                Some(quotas.max_events_per_day() as usize)
            },
            max_storage_bytes: if quotas.max_storage_bytes() == 0 {
                None
            } else {
                Some(quotas.max_storage_bytes())
            },
            max_queries_per_hour: if quotas.max_queries_per_hour() == 0 {
                None
            } else {
                Some(quotas.max_queries_per_hour() as usize)
            },
            max_api_keys: if quotas.max_api_keys() == 0 {
                None
            } else {
                Some(quotas.max_api_keys() as usize)
            },
            max_projections: if quotas.max_projections() == 0 {
                None
            } else {
                Some(quotas.max_projections() as usize)
            },
            max_pipelines: if quotas.max_pipelines() == 0 {
                None
            } else {
                Some(quotas.max_pipelines() as usize)
            },
        }
    }
}

impl From<TenantQuotasDto> for TenantQuotas {
    fn from(dto: TenantQuotasDto) -> Self {
        TenantQuotas::new(
            dto.max_events_per_day.map(|v| v as u64).unwrap_or(0),
            dto.max_storage_bytes.unwrap_or(0),
            dto.max_queries_per_hour.map(|v| v as u64).unwrap_or(0),
            dto.max_api_keys.map(|v| v as u32).unwrap_or(0),
            dto.max_projections.map(|v| v as u32).unwrap_or(0),
            dto.max_pipelines.map(|v| v as u32).unwrap_or(0),
        )
    }
}

/// DTO for tenant response
#[derive(Debug, Serialize)]
pub struct TenantDto {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub is_active: bool,
    pub quotas: TenantQuotasDto,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Tenant> for TenantDto {
    fn from(tenant: &Tenant) -> Self {
        Self {
            id: Uuid::new_v4(), // Tenant uses TenantId as ID, not UUID
            tenant_id: tenant.id().to_string(),
            name: tenant.name().to_string(),
            is_active: tenant.is_active(),
            quotas: tenant.quotas().clone().into(),
            created_at: tenant.created_at(),
            updated_at: tenant.updated_at(),
        }
    }
}

impl From<Tenant> for TenantDto {
    fn from(tenant: Tenant) -> Self {
        TenantDto::from(&tenant)
    }
}

/// Response for tenant creation
#[derive(Debug, Serialize)]
pub struct CreateTenantResponse {
    pub tenant: TenantDto,
}

/// Response for tenant update
#[derive(Debug, Serialize)]
pub struct UpdateTenantResponse {
    pub tenant: TenantDto,
}

/// Response for listing tenants
#[derive(Debug, Serialize)]
pub struct ListTenantsResponse {
    pub tenants: Vec<TenantDto>,
    pub count: usize,
}
