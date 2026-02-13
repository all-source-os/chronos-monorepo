package usecases

import (
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// ListOperationsRequest contains filter options for querying operations.
type ListOperationsRequest struct {
	Type     entities.OperationType
	Status   entities.OperationStatus
	TenantID string
	Limit    int
}

// ListOperationsUseCase queries operation history from the DB with filters.
type ListOperationsUseCase struct {
	opRepo repositories.OperationRepository
}

// NewListOperationsUseCase creates a new ListOperationsUseCase.
func NewListOperationsUseCase(opRepo repositories.OperationRepository) *ListOperationsUseCase {
	return &ListOperationsUseCase{opRepo: opRepo}
}

// Execute queries operations with the provided filters. Only one filter is applied at a time, with priority: type > status > tenant. If no filters, returns recent operations.
func (uc *ListOperationsUseCase) Execute(req ListOperationsRequest) ([]*entities.Operation, error) {
	limit := req.Limit
	if limit <= 0 {
		limit = 50
	}

	if req.Type != "" {
		return uc.opRepo.FindByType(req.Type)
	}

	if req.Status != "" {
		return uc.opRepo.FindByStatus(req.Status)
	}

	if req.TenantID != "" {
		return uc.opRepo.FindByTenant(req.TenantID)
	}

	return uc.opRepo.FindRecent(limit)
}
