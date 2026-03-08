package persistence //nolint:dupl // typed repository — structural duplication is intentional

import (
	"sync"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
)

// MemorySLORepository is an in-memory implementation of SLORepository
type MemorySLORepository struct {
	mu   sync.RWMutex
	slos map[string]*entities.SLO
}

// NewMemorySLORepository creates a new MemorySLORepository
func NewMemorySLORepository() *MemorySLORepository {
	return &MemorySLORepository{
		slos: make(map[string]*entities.SLO),
	}
}

// Save persists an SLO
func (r *MemorySLORepository) Save(slo *entities.SLO) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.slos[slo.ID] = slo
	return nil
}

// FindByID retrieves an SLO by ID
func (r *MemorySLORepository) FindByID(id string) (*entities.SLO, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	slo, ok := r.slos[id]
	if !ok {
		return nil, domain.ErrSLONotFound
	}
	return slo, nil
}

// FindAll retrieves all SLOs
func (r *MemorySLORepository) FindAll() ([]*entities.SLO, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	result := make([]*entities.SLO, 0, len(r.slos))
	for _, slo := range r.slos {
		result = append(result, slo)
	}
	return result, nil
}

// Delete removes an SLO by ID
func (r *MemorySLORepository) Delete(id string) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if _, ok := r.slos[id]; !ok {
		return domain.ErrSLONotFound
	}
	delete(r.slos, id)
	return nil
}

// Exists checks if an SLO exists
func (r *MemorySLORepository) Exists(id string) (bool, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	_, ok := r.slos[id]
	return ok, nil
}
