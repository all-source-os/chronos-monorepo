package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// CreateTenantUseCase handles tenant creation
type CreateTenantUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewCreateTenantUseCase creates a new CreateTenantUseCase
func NewCreateTenantUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *CreateTenantUseCase {
	return &CreateTenantUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute creates a new tenant
func (uc *CreateTenantUseCase) Execute(req dto.CreateTenantRequest) (*dto.TenantResponse, error) {
	// Check if tenant already exists
	exists, err := uc.tenantRepo.Exists(req.ID)
	if err != nil {
		return nil, err
	}
	if exists {
		return nil, domain.ErrTenantAlreadyExists
	}

	// Create domain entity
	tenant, err := entities.NewTenant(req.ID, req.Name, req.Description)
	if err != nil {
		return nil, err
	}

	// Add metadata
	if req.Metadata != nil {
		tenant.Metadata = req.Metadata
	}

	// Persist tenant
	if err := uc.tenantRepo.Save(tenant); err != nil {
		return nil, err
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.created", "create", "POST", "/tenants")
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	_ = uc.auditRepo.Log(auditEvent)

	// Return response
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
