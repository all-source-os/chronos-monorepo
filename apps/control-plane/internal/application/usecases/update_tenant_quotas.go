package usecases

import (
	"context"
	"fmt"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// UpdateTenantQuotasUseCase handles updating a tenant's resource quotas (Admin only).
type UpdateTenantQuotasUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewUpdateTenantQuotasUseCase creates a new UpdateTenantQuotasUseCase.
func NewUpdateTenantQuotasUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *UpdateTenantQuotasUseCase {
	return &UpdateTenantQuotasUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute updates a tenant's quotas. Requires Admin role.
func (uc *UpdateTenantQuotasUseCase) Execute(ctx context.Context, id string, req dto.UpdateTenantQuotasRequest, role entities.Role) (*dto.TenantResponse, error) {
	// Check Admin role
	if role != entities.RoleAdmin {
		return nil, domain.ErrForbidden
	}

	// Find existing tenant
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	// Build new quotas from existing, applying only fields that were provided
	newQuotas := tenant.Quotas
	if req.EventLimit != nil {
		newQuotas.EventLimit = *req.EventLimit
	}
	if req.QueryLimit != nil {
		newQuotas.QueryLimit = *req.QueryLimit
	}
	if req.StorageLimitMB != nil {
		newQuotas.StorageLimitMB = *req.StorageLimitMB
	}

	// Apply quotas (validates internally)
	if err := tenant.UpdateQuotas(newQuotas); err != nil {
		return nil, fmt.Errorf("%w: %s", domain.ErrInvalidInput, err.Error())
	}

	// Persist
	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, err
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.quotas_updated", "update_quotas", "PUT", "/admin/tenants/"+id+"/quotas") //nolint:errcheck
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	auditEvent.AddMetadata("event_limit", newQuotas.EventLimit)
	auditEvent.AddMetadata("query_limit", newQuotas.QueryLimit)
	auditEvent.AddMetadata("storage_limit_mb", newQuotas.StorageLimitMB)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &dto.TenantResponse{
		ID:          tenant.ID,
		Name:        tenant.Name,
		Description: tenant.Description,
		Status:      string(tenant.Status),
		HomeRegion:  tenant.EffectiveHomeRegion(),
		Quotas: &dto.TenantQuotasResponse{
			EventLimit:     tenant.Quotas.EventLimit,
			QueryLimit:     tenant.Quotas.QueryLimit,
			StorageLimitMB: tenant.Quotas.StorageLimitMB,
		},
		CreatedAt: tenant.CreatedAt,
		UpdatedAt: tenant.UpdatedAt,
		Metadata:  tenant.Metadata,
	}, nil
}
