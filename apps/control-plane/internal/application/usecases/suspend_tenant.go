package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// SuspendTenantUseCase handles suspending a tenant (Admin only).
type SuspendTenantUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
}

// NewSuspendTenantUseCase creates a new SuspendTenantUseCase.
func NewSuspendTenantUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *SuspendTenantUseCase {
	return &SuspendTenantUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// Execute suspends a tenant. Requires Admin role.
func (uc *SuspendTenantUseCase) Execute(ctx context.Context, id string, role entities.Role) (*dto.TenantResponse, error) {
	// Check Admin role
	if role != entities.RoleAdmin {
		return nil, domain.ErrForbidden
	}

	// Find existing tenant
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	// Suspend in local DB
	tenant.Suspend()
	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, err
	}

	// Notify Core to deactivate tenant
	if uc.coreClient != nil {
		_, _ = uc.coreClient.DeactivateTenant(ctx, id) //nolint:errcheck // Core notification is best-effort
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.suspended", "suspend", "POST", "/tenants/"+id+"/suspend") //nolint:errcheck // audit event creation is non-critical
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

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
	coreClient clients.CoreClient
}

// NewActivateTenantUseCase creates a new ActivateTenantUseCase.
func NewActivateTenantUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *ActivateTenantUseCase {
	return &ActivateTenantUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// Execute activates a suspended tenant. Requires Admin role.
func (uc *ActivateTenantUseCase) Execute(ctx context.Context, id string, role entities.Role) (*dto.TenantResponse, error) {
	// Check Admin role
	if role != entities.RoleAdmin {
		return nil, domain.ErrForbidden
	}

	// Find existing tenant
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	// Activate in local DB
	tenant.Activate()
	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, err
	}

	// Notify Core to activate tenant
	if uc.coreClient != nil {
		_, _ = uc.coreClient.ActivateTenant(ctx, id) //nolint:errcheck // Core notification is best-effort
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.activated", "activate", "POST", "/tenants/"+id+"/activate") //nolint:errcheck // audit event creation is non-critical
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

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
