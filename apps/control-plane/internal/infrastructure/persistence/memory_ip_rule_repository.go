package persistence //nolint:dupl // typed repository — structural duplication is intentional

import (
	"sync"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
)

// MemoryIPRuleRepository is an in-memory implementation of IPRuleRepository
type MemoryIPRuleRepository struct {
	mu    sync.RWMutex
	rules map[string]*entities.IPRule
}

// NewMemoryIPRuleRepository creates a new MemoryIPRuleRepository
func NewMemoryIPRuleRepository() *MemoryIPRuleRepository {
	return &MemoryIPRuleRepository{
		rules: make(map[string]*entities.IPRule),
	}
}

// Save persists an IP rule
func (r *MemoryIPRuleRepository) Save(rule *entities.IPRule) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.rules[rule.ID] = rule
	return nil
}

// FindByID retrieves an IP rule by ID
func (r *MemoryIPRuleRepository) FindByID(id string) (*entities.IPRule, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	rule, ok := r.rules[id]
	if !ok {
		return nil, domain.ErrIPRuleNotFound
	}
	return rule, nil
}

// FindAll retrieves all IP rules
func (r *MemoryIPRuleRepository) FindAll() ([]*entities.IPRule, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	result := make([]*entities.IPRule, 0, len(r.rules))
	for _, rule := range r.rules {
		result = append(result, rule)
	}
	return result, nil
}

// Delete removes an IP rule by ID
func (r *MemoryIPRuleRepository) Delete(id string) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if _, ok := r.rules[id]; !ok {
		return domain.ErrIPRuleNotFound
	}
	delete(r.rules, id)
	return nil
}

// Exists checks if an IP rule exists
func (r *MemoryIPRuleRepository) Exists(id string) (bool, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	_, ok := r.rules[id]
	return ok, nil
}
