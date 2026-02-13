package persistence

import (
	"sync"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
)

// MemoryConfigRepository is an in-memory implementation of ConfigRepository
type MemoryConfigRepository struct {
	mu      sync.RWMutex
	entries map[string]*entities.ConfigEntry
}

// NewMemoryConfigRepository creates a new MemoryConfigRepository
func NewMemoryConfigRepository() *MemoryConfigRepository {
	return &MemoryConfigRepository{
		entries: make(map[string]*entities.ConfigEntry),
	}
}

// Save persists a config entry
func (r *MemoryConfigRepository) Save(entry *entities.ConfigEntry) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.entries[entry.Key] = entry
	return nil
}

// FindByKey retrieves a config entry by key
func (r *MemoryConfigRepository) FindByKey(key string) (*entities.ConfigEntry, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	entry, ok := r.entries[key]
	if !ok {
		return nil, domain.ErrConfigNotFound
	}
	return entry, nil
}

// FindAll retrieves all config entries
func (r *MemoryConfigRepository) FindAll() ([]*entities.ConfigEntry, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	result := make([]*entities.ConfigEntry, 0, len(r.entries))
	for _, entry := range r.entries {
		result = append(result, entry)
	}
	return result, nil
}

// FindByCategory retrieves config entries by category
func (r *MemoryConfigRepository) FindByCategory(category string) ([]*entities.ConfigEntry, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	var result []*entities.ConfigEntry
	for _, entry := range r.entries {
		if entry.Category == category {
			result = append(result, entry)
		}
	}
	return result, nil
}

// Delete removes a config entry by key
func (r *MemoryConfigRepository) Delete(key string) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if _, ok := r.entries[key]; !ok {
		return domain.ErrConfigNotFound
	}
	delete(r.entries, key)
	return nil
}

// Exists checks if a config entry exists
func (r *MemoryConfigRepository) Exists(key string) (bool, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	_, ok := r.entries[key]
	return ok, nil
}
