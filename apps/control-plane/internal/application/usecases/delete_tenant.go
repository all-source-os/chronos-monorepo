package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// DeleteTenantUseCase handles soft-deleting a tenant (Admin only).
type DeleteTenantUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewDeleteTenantUseCase creates a new DeleteTenantUseCase.
func NewDeleteTenantUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *DeleteTenantUseCase {
	return &DeleteTenantUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute soft-deletes a tenant. Requires Admin role.
func (uc *DeleteTenantUseCase) Execute(ctx context.Context, id string, role entities.Role) error {
	// Check Admin role
	if role != entities.RoleAdmin {
		return domain.ErrForbidden
	}

	// Find existing tenant
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return err
	}

	// Mark as deleted (domain validation: prevents deleting "default" tenant)
	if err := tenant.MarkDeleted(); err != nil {
		return err
	}

	// Persist — CoreTenantRepository.Update() delegates to Core directly (single write)
	if err := uc.tenantRepo.Update(tenant); err != nil {
		return err
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.deleted", "delete", "DELETE", "/tenants/"+id) //nolint:errcheck
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return nil
}
