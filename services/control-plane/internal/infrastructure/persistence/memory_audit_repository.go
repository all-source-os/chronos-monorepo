package persistence

import (
	"github.com/allsource/control-plane/internal/domain/entities"
	"sync"
	"time"
)

// MemoryAuditRepository is an in-memory implementation of AuditRepository
type MemoryAuditRepository struct {
	events []entities.AuditEvent
	mu     sync.RWMutex
}

// NewMemoryAuditRepository creates a new MemoryAuditRepository
func NewMemoryAuditRepository() *MemoryAuditRepository {
	return &MemoryAuditRepository{
		events: make([]entities.AuditEvent, 0),
	}
}

// Log persists an audit event
func (r *MemoryAuditRepository) Log(event *entities.AuditEvent) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	r.events = append(r.events, *event)
	return nil
}

// FindByUser retrieves audit events for a specific user
func (r *MemoryAuditRepository) FindByUser(userID string, limit int) ([]*entities.AuditEvent, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.AuditEvent, 0)
	count := 0

	// Iterate in reverse to get most recent first
	for i := len(r.events) - 1; i >= 0 && count < limit; i-- {
		if r.events[i].UserID == userID {
			event := r.events[i]
			result = append(result, &event)
			count++
		}
	}

	return result, nil
}

// FindByTenant retrieves audit events for a specific tenant
func (r *MemoryAuditRepository) FindByTenant(tenantID string, limit int) ([]*entities.AuditEvent, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.AuditEvent, 0)
	count := 0

	// Iterate in reverse to get most recent first
	for i := len(r.events) - 1; i >= 0 && count < limit; i-- {
		if r.events[i].TenantID == tenantID {
			event := r.events[i]
			result = append(result, &event)
			count++
		}
	}

	return result, nil
}

// FindByTimeRange retrieves audit events within a time range
func (r *MemoryAuditRepository) FindByTimeRange(start, end time.Time) ([]*entities.AuditEvent, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.AuditEvent, 0)

	for i := range r.events {
		if r.events[i].Timestamp.After(start) && r.events[i].Timestamp.Before(end) {
			event := r.events[i]
			result = append(result, &event)
		}
	}

	return result, nil
}

// FindErrors retrieves audit events that represent errors
func (r *MemoryAuditRepository) FindErrors(limit int) ([]*entities.AuditEvent, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.AuditEvent, 0)
	count := 0

	// Iterate in reverse to get most recent first
	for i := len(r.events) - 1; i >= 0 && count < limit; i-- {
		if r.events[i].IsError() {
			event := r.events[i]
			result = append(result, &event)
			count++
		}
	}

	return result, nil
}
