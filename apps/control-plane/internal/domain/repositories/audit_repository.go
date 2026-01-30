// Package repositories defines repository interfaces for domain persistence.
package repositories

import (
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
)

// AuditRepository defines the interface for audit event persistence
type AuditRepository interface {
	// Log persists an audit event
	Log(event *entities.AuditEvent) error

	// FindByUser retrieves audit events for a specific user
	FindByUser(userID string, limit int) ([]*entities.AuditEvent, error)

	// FindByTenant retrieves audit events for a specific tenant
	FindByTenant(tenantID string, limit int) ([]*entities.AuditEvent, error)

	// FindByTimeRange retrieves audit events within a time range
	FindByTimeRange(start, end time.Time) ([]*entities.AuditEvent, error)

	// FindErrors retrieves audit events that represent errors
	FindErrors(limit int) ([]*entities.AuditEvent, error)
}
