package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// GetTenantUseCase handles retrieving a single tenant by ID.
type GetTenantUseCase struct {
	tenantRepo repositories.TenantRepository
}

// NewGetTenantUseCase creates a new GetTenantUseCase.
func NewGetTenantUseCase(tenantRepo repositories.TenantRepository) *GetTenantUseCase {
	return &GetTenantUseCase{tenantRepo: tenantRepo}
}

// Execute retrieves a tenant by ID and returns a response DTO.
func (uc *GetTenantUseCase) Execute(id string) (*dto.TenantResponse, error) {
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

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
