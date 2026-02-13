package repositories

import "github.com/allsource/control-plane/internal/domain/entities"

// OperationRepository defines the interface for operation persistence
type OperationRepository interface {
	// Save persists an operation
	Save(op *entities.Operation) error

	// FindByID retrieves an operation by ID
	FindByID(id string) (*entities.Operation, error)

	// FindByType retrieves operations by type
	FindByType(opType entities.OperationType) ([]*entities.Operation, error)

	// FindByStatus retrieves operations by status
	FindByStatus(status entities.OperationStatus) ([]*entities.Operation, error)

	// FindByTenant retrieves operations by tenant ID
	FindByTenant(tenantID string) ([]*entities.Operation, error)

	// Update updates an existing operation
	Update(op *entities.Operation) error

	// FindRecent retrieves the most recent operations
	FindRecent(limit int) ([]*entities.Operation, error)
}
