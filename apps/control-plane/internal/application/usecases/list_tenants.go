package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// ListTenantsRequest holds pagination and filter parameters.
type ListTenantsRequest struct {
	Offset int
	Limit  int
	Status string // optional: "active", "suspended", "deleted"; empty means all
}

// ListTenantsResponse wraps the paginated result.
type ListTenantsResponse struct {
	Tenants []*dto.TenantResponse `json:"tenants"`
	Total   int                   `json:"total"`
}

// ListTenantsUseCase handles listing tenants with pagination and status filter.
type ListTenantsUseCase struct {
	tenantRepo repositories.TenantRepository
}

// NewListTenantsUseCase creates a new ListTenantsUseCase.
func NewListTenantsUseCase(tenantRepo repositories.TenantRepository) *ListTenantsUseCase {
	return &ListTenantsUseCase{tenantRepo: tenantRepo}
}

// Execute retrieves tenants with optional status filter and pagination.
func (uc *ListTenantsUseCase) Execute(req ListTenantsRequest) (*ListTenantsResponse, error) {
	var tenants []*entities.Tenant
	var err error

	// Filter by status if specified
	switch entities.TenantStatus(req.Status) {
	case entities.TenantStatusActive:
		tenants, err = uc.tenantRepo.FindActive()
	default:
		tenants, err = uc.tenantRepo.FindAll()
	}
	if err != nil {
		return nil, err
	}

	// Apply in-memory status filter for non-active statuses (suspended, deleted)
	if req.Status != "" && req.Status != string(entities.TenantStatusActive) {
		filtered := make([]*entities.Tenant, 0)
		for _, t := range tenants {
			if string(t.Status) == req.Status {
				filtered = append(filtered, t)
			}
		}
		tenants = filtered
	}

	total := len(tenants)

	// Apply pagination
	if req.Offset > 0 {
		if req.Offset >= len(tenants) {
			tenants = nil
		} else {
			tenants = tenants[req.Offset:]
		}
	}
	if req.Limit > 0 && req.Limit < len(tenants) {
		tenants = tenants[:req.Limit]
	}

	// Convert to DTOs
	responses := make([]*dto.TenantResponse, len(tenants))
	for i, t := range tenants {
		responses[i] = &dto.TenantResponse{
			ID:          t.ID,
			Name:        t.Name,
			Description: t.Description,
			Status:      string(t.Status),
			CreatedAt:   t.CreatedAt,
			UpdatedAt:   t.UpdatedAt,
			Metadata:    t.Metadata,
		}
	}

	return &ListTenantsResponse{
		Tenants: responses,
		Total:   total,
	}, nil
}
