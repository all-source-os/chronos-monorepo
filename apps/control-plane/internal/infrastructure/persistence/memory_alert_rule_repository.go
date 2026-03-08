package persistence //nolint:dupl // typed repository — structural duplication is intentional

import (
	"sync"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
)

// MemoryAlertRuleRepository is an in-memory implementation of AlertRuleRepository
type MemoryAlertRuleRepository struct {
	mu    sync.RWMutex
	rules map[string]*entities.AlertRule
}

// NewMemoryAlertRuleRepository creates a new MemoryAlertRuleRepository
func NewMemoryAlertRuleRepository() *MemoryAlertRuleRepository {
	return &MemoryAlertRuleRepository{
		rules: make(map[string]*entities.AlertRule),
	}
}

// Save persists an alert rule
func (r *MemoryAlertRuleRepository) Save(rule *entities.AlertRule) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.rules[rule.ID] = rule
	return nil
}

// FindByID retrieves an alert rule by ID
func (r *MemoryAlertRuleRepository) FindByID(id string) (*entities.AlertRule, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	rule, ok := r.rules[id]
	if !ok {
		return nil, domain.ErrAlertRuleNotFound
	}
	return rule, nil
}

// FindAll retrieves all alert rules
func (r *MemoryAlertRuleRepository) FindAll() ([]*entities.AlertRule, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	result := make([]*entities.AlertRule, 0, len(r.rules))
	for _, rule := range r.rules {
		result = append(result, rule)
	}
	return result, nil
}

// Delete removes an alert rule by ID
func (r *MemoryAlertRuleRepository) Delete(id string) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if _, ok := r.rules[id]; !ok {
		return domain.ErrAlertRuleNotFound
	}
	delete(r.rules, id)
	return nil
}

// Exists checks if an alert rule exists
func (r *MemoryAlertRuleRepository) Exists(id string) (bool, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	_, ok := r.rules[id]
	return ok, nil
}
