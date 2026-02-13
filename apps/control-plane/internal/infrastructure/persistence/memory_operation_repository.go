package persistence

import (
	"sort"
	"sync"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
)

// MemoryOperationRepository is an in-memory implementation of OperationRepository
type MemoryOperationRepository struct {
	operations map[string]*entities.Operation
	mu         sync.RWMutex
}

// NewMemoryOperationRepository creates a new MemoryOperationRepository
func NewMemoryOperationRepository() *MemoryOperationRepository {
	return &MemoryOperationRepository{
		operations: make(map[string]*entities.Operation),
	}
}

// Save persists an operation
func (r *MemoryOperationRepository) Save(op *entities.Operation) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	r.operations[op.ID] = op
	return nil
}

// FindByID retrieves an operation by ID
func (r *MemoryOperationRepository) FindByID(id string) (*entities.Operation, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	op, exists := r.operations[id]
	if !exists {
		return nil, domain.ErrOperationNotFound
	}
	return op, nil
}

// FindByType retrieves operations by type
func (r *MemoryOperationRepository) FindByType(opType entities.OperationType) ([]*entities.Operation, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var ops []*entities.Operation
	for _, op := range r.operations {
		if op.Type == opType {
			ops = append(ops, op)
		}
	}
	sortByCreatedDesc(ops)
	return ops, nil
}

// FindByStatus retrieves operations by status
func (r *MemoryOperationRepository) FindByStatus(status entities.OperationStatus) ([]*entities.Operation, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var ops []*entities.Operation
	for _, op := range r.operations {
		if op.Status == status {
			ops = append(ops, op)
		}
	}
	sortByCreatedDesc(ops)
	return ops, nil
}

// FindByTenant retrieves operations by tenant ID
func (r *MemoryOperationRepository) FindByTenant(tenantID string) ([]*entities.Operation, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var ops []*entities.Operation
	for _, op := range r.operations {
		if op.TenantID == tenantID {
			ops = append(ops, op)
		}
	}
	sortByCreatedDesc(ops)
	return ops, nil
}

// Update updates an existing operation
func (r *MemoryOperationRepository) Update(op *entities.Operation) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.operations[op.ID]; !exists {
		return domain.ErrOperationNotFound
	}
	r.operations[op.ID] = op
	return nil
}

// FindRecent retrieves the most recent operations
func (r *MemoryOperationRepository) FindRecent(limit int) ([]*entities.Operation, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	ops := make([]*entities.Operation, 0, len(r.operations))
	for _, op := range r.operations {
		ops = append(ops, op)
	}
	sortByCreatedDesc(ops)

	if limit > 0 && limit < len(ops) {
		ops = ops[:limit]
	}
	return ops, nil
}

func sortByCreatedDesc(ops []*entities.Operation) {
	sort.Slice(ops, func(i, j int) bool {
		return ops[i].CreatedAt.After(ops[j].CreatedAt)
	})
}
