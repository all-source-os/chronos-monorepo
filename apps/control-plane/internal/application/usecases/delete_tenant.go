package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// DeleteTenantUseCase handles soft-deleting a tenant (Admin only).
type DeleteTenantUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
}

// NewDeleteTenantUseCase creates a new DeleteTenantUseCase.
func NewDeleteTenantUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *DeleteTenantUseCase {
	return &DeleteTenantUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
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

	// Persist the soft-delete status
	if err := uc.tenantRepo.Update(tenant); err != nil {
		return err
	}

	// Notify Core to delete tenant
	if uc.coreClient != nil {
		_ = uc.coreClient.DeleteTenant(ctx, id) //nolint:errcheck // Core notification is best-effort
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.deleted", "delete", "DELETE", "/tenants/"+id) //nolint:errcheck // audit event creation is non-critical
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return nil
}
