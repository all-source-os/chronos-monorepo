package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// SuspendTenantUseCase handles suspending a tenant (Admin only).
type SuspendTenantUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewSuspendTenantUseCase creates a new SuspendTenantUseCase.
func NewSuspendTenantUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *SuspendTenantUseCase {
	return &SuspendTenantUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute suspends a tenant. Requires Admin role.
func (uc *SuspendTenantUseCase) Execute(ctx context.Context, id string, role entities.Role) (*dto.TenantResponse, error) { //nolint:dupl // structurally similar to Activate by design
	// Check Admin role
	if role != entities.RoleAdmin {
		return nil, domain.ErrForbidden
	}

	// Find existing tenant
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	// Suspend — CoreTenantRepository.Update() delegates to Core directly (single write)
	tenant.Suspend()
	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, err
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.suspended", "suspend", "POST", "/tenants/"+id+"/suspend") //nolint:errcheck
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &dto.TenantResponse{
		ID:          tenant.ID,
		Name:        tenant.Name,
		Description: tenant.Description,
		Status:      string(tenant.Status),
		CreatedAt:   tenant.CreatedAt,
		UpdatedAt:   tenant.UpdatedAt,
		Metadata:    tenant.Metadata,
	}, nil
}

// ActivateTenantUseCase handles re-activating a suspended tenant (Admin only).
type ActivateTenantUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewActivateTenantUseCase creates a new ActivateTenantUseCase.
func NewActivateTenantUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *ActivateTenantUseCase {
	return &ActivateTenantUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute activates a suspended tenant. Requires Admin role.
func (uc *ActivateTenantUseCase) Execute(ctx context.Context, id string, role entities.Role) (*dto.TenantResponse, error) { //nolint:dupl // structurally similar to Suspend by design
	// Check Admin role
	if role != entities.RoleAdmin {
		return nil, domain.ErrForbidden
	}

	// Find existing tenant
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	// Activate — CoreTenantRepository.Update() delegates to Core directly (single write)
	tenant.Activate()
	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, err
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.activated", "activate", "POST", "/tenants/"+id+"/activate") //nolint:errcheck
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &dto.TenantResponse{
		ID:          tenant.ID,
		Name:        tenant.Name,
		Description: tenant.Description,
		Status:      string(tenant.Status),
		CreatedAt:   tenant.CreatedAt,
		UpdatedAt:   tenant.UpdatedAt,
		Metadata:    tenant.Metadata,
	}, nil
}
