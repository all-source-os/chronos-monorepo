use crate::application::dto::{
    CreateTenantRequest, CreateTenantResponse, UpdateTenantRequest, UpdateTenantResponse,
    ListTenantsResponse, TenantDto,
};
use crate::domain::entities::Tenant;
use crate::domain::value_objects::TenantId;
use crate::error::Result;

/// Use Case: Create Tenant
///
/// Creates a new tenant with specified quotas.
pub struct CreateTenantUseCase;

impl CreateTenantUseCase {
    pub fn execute(request: CreateTenantRequest) -> Result<CreateTenantResponse> {
        // Validate and create tenant ID
        let tenant_id = TenantId::new(request.tenant_id)?;

        // Create tenant with quotas (or use default)
        let tenant = if let Some(quotas_dto) = request.quotas {
            Tenant::new(tenant_id, request.name, quotas_dto.into())?
        } else {
            // Use free tier by default
            Tenant::new(tenant_id, request.name, crate::domain::entities::TenantQuotas::free_tier())?
        };

        Ok(CreateTenantResponse {
            tenant: TenantDto::from(&tenant),
        })
    }
}

/// Use Case: Update Tenant
///
/// Updates tenant information and/or quotas.
pub struct UpdateTenantUseCase;

impl UpdateTenantUseCase {
    pub fn execute(mut tenant: Tenant, request: UpdateTenantRequest) -> Result<UpdateTenantResponse> {
        // Update name if provided
        if let Some(name) = request.name {
            tenant.update_name(name)?;
        }

        // Update quotas if provided
        if let Some(quotas_dto) = request.quotas {
            tenant.update_quotas(quotas_dto.into());
        }

        Ok(UpdateTenantResponse {
            tenant: TenantDto::from(&tenant),
        })
    }
}

/// Use Case: Activate Tenant
///
/// Activates a previously deactivated tenant.
pub struct ActivateTenantUseCase;

impl ActivateTenantUseCase {
    pub fn execute(mut tenant: Tenant) -> Result<TenantDto> {
        tenant.activate();
        Ok(TenantDto::from(&tenant))
    }
}

/// Use Case: Deactivate Tenant
///
/// Deactivates a tenant, preventing event ingestion and queries.
pub struct DeactivateTenantUseCase;

impl DeactivateTenantUseCase {
    pub fn execute(mut tenant: Tenant) -> Result<TenantDto> {
        tenant.deactivate();
        Ok(TenantDto::from(&tenant))
    }
}

/// Use Case: List Tenants
///
/// Returns a list of all tenants (would typically come from a repository).
pub struct ListTenantsUseCase;

impl ListTenantsUseCase {
    pub fn execute(tenants: Vec<Tenant>) -> ListTenantsResponse {
        let tenant_dtos: Vec<TenantDto> = tenants.iter().map(TenantDto::from).collect();
        let count = tenant_dtos.len();

        ListTenantsResponse {
            tenants: tenant_dtos,
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::dto::TenantQuotasDto;

    #[test]
    fn test_create_tenant_with_default_quotas() {
        let request = CreateTenantRequest {
            tenant_id: "test-tenant".to_string(),
            name: "Test Tenant".to_string(),
            quotas: None,
        };

        let response = CreateTenantUseCase::execute(request);
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.tenant.name, "Test Tenant");
        assert_eq!(response.tenant.tenant_id, "test-tenant");
        assert!(response.tenant.is_active);
    }

    #[test]
    fn test_create_tenant_with_custom_quotas() {
        let request = CreateTenantRequest {
            tenant_id: "premium-tenant".to_string(),
            name: "Premium Tenant".to_string(),
            quotas: Some(TenantQuotasDto {
                max_events_per_day: Some(1_000_000),
                max_storage_bytes: Some(10_000_000_000),
                max_queries_per_hour: Some(10_000),
                max_api_keys: Some(50),
                max_projections: Some(100),
                max_pipelines: Some(50),
            }),
        };

        let response = CreateTenantUseCase::execute(request);
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.tenant.quotas.max_events_per_day, Some(1_000_000));
    }

    #[test]
    fn test_update_tenant_name() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let tenant = Tenant::new(
            tenant_id,
            "Old Name".to_string(),
            crate::domain::entities::TenantQuotas::free_tier(),
        )
        .unwrap();

        let request = UpdateTenantRequest {
            name: Some("New Name".to_string()),
            quotas: None,
        };

        let response = UpdateTenantUseCase::execute(tenant, request);
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.tenant.name, "New Name");
    }

    #[test]
    fn test_activate_deactivate_tenant() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let tenant = Tenant::new(
            tenant_id,
            "Test".to_string(),
            crate::domain::entities::TenantQuotas::free_tier(),
        )
        .unwrap();

        // Deactivate
        let result = DeactivateTenantUseCase::execute(tenant.clone());
        assert!(result.is_ok());
        assert!(!result.unwrap().is_active);

        // Activate
        let mut deactivated = tenant.clone();
        deactivated.deactivate();
        let result = ActivateTenantUseCase::execute(deactivated);
        assert!(result.is_ok());
        assert!(result.unwrap().is_active);
    }

    #[test]
    fn test_list_tenants() {
        let tenants = vec![
            Tenant::new(
                TenantId::new("tenant-1".to_string()).unwrap(),
                "Tenant 1".to_string(),
                crate::domain::entities::TenantQuotas::free_tier(),
            )
            .unwrap(),
            Tenant::new(
                TenantId::new("tenant-2".to_string()).unwrap(),
                "Tenant 2".to_string(),
                crate::domain::entities::TenantQuotas::standard(),
            )
            .unwrap(),
        ];

        let response = ListTenantsUseCase::execute(tenants);
        assert_eq!(response.count, 2);
        assert_eq!(response.tenants.len(), 2);
    }
}
