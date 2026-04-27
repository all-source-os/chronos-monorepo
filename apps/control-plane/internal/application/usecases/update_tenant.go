package usecases

import (
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// UpdateTenantUseCase handles updating a tenant's mutable fields.
type UpdateTenantUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewUpdateTenantUseCase creates a new UpdateTenantUseCase.
func NewUpdateTenantUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *UpdateTenantUseCase {
	return &UpdateTenantUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute updates a tenant's name, description, and/or metadata.
func (uc *UpdateTenantUseCase) Execute(id string, req dto.UpdateTenantRequest) (*dto.TenantResponse, error) {
	// Find existing tenant
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	// Apply updates
	if req.Name != "" {
		if err := entities.ValidateTenantName(req.Name); err != nil {
			return nil, err
		}
		tenant.Name = req.Name
	}
	if req.Description != "" {
		tenant.Description = req.Description
	}
	if req.Metadata != nil {
		tenant.Metadata = req.Metadata
	}
	tenant.UpdatedAt = time.Now()

	// Persist
	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, err
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.updated", "update", "PUT", "/tenants/"+id) //nolint:errcheck // audit event creation is non-critical
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return &dto.TenantResponse{
		ID:          tenant.ID,
		Name:        tenant.Name,
		Description: tenant.Description,
		Status:      string(tenant.Status),
		HomeRegion:  tenant.EffectiveHomeRegion(),
		CreatedAt:   tenant.CreatedAt,
		UpdatedAt:   tenant.UpdatedAt,
		Metadata:    tenant.Metadata,
	}, nil
}
