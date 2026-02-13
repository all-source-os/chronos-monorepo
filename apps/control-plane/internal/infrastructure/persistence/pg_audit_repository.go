package persistence

import (
	"context"
	"encoding/json"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

// PgAuditRepository is a PostgreSQL implementation of AuditRepository
type PgAuditRepository struct {
	pool *pgxpool.Pool
}

// NewPgAuditRepository creates a new PgAuditRepository
func NewPgAuditRepository(pool *pgxpool.Pool) *PgAuditRepository {
	return &PgAuditRepository{pool: pool}
}

// Log persists an audit event
func (r *PgAuditRepository) Log(event *entities.AuditEvent) error {
	metadata, err := json.Marshal(event.Metadata)
	if err != nil {
		return err
	}

	_, err = r.pool.Exec(context.Background(),
		`INSERT INTO control_plane.audit_events
		 (timestamp, event_type, user_id, username, tenant_id, action, resource, resource_id,
		  method, path, status_code, duration, ip_address, user_agent, error, metadata)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)`,
		event.Timestamp, event.EventType, event.UserID, event.Username, event.TenantID,
		event.Action, event.Resource, event.ResourceID, event.Method, event.Path,
		event.StatusCode, event.Duration, event.IPAddress, event.UserAgent, event.Error, metadata,
	)
	return err
}

// FindByUser retrieves audit events for a specific user
func (r *PgAuditRepository) FindByUser(userID string, limit int) ([]*entities.AuditEvent, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT timestamp, event_type, user_id, username, tenant_id, action, resource, resource_id,
		        method, path, status_code, duration, ip_address, user_agent, error, metadata
		 FROM control_plane.audit_events
		 WHERE user_id = $1
		 ORDER BY timestamp DESC
		 LIMIT $2`, userID, limit,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanAuditEvents(rows)
}

// FindByTenant retrieves audit events for a specific tenant
func (r *PgAuditRepository) FindByTenant(tenantID string, limit int) ([]*entities.AuditEvent, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT timestamp, event_type, user_id, username, tenant_id, action, resource, resource_id,
		        method, path, status_code, duration, ip_address, user_agent, error, metadata
		 FROM control_plane.audit_events
		 WHERE tenant_id = $1
		 ORDER BY timestamp DESC
		 LIMIT $2`, tenantID, limit,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanAuditEvents(rows)
}

// FindByTimeRange retrieves audit events within a time range
func (r *PgAuditRepository) FindByTimeRange(start, end time.Time) ([]*entities.AuditEvent, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT timestamp, event_type, user_id, username, tenant_id, action, resource, resource_id,
		        method, path, status_code, duration, ip_address, user_agent, error, metadata
		 FROM control_plane.audit_events
		 WHERE timestamp >= $1 AND timestamp <= $2
		 ORDER BY timestamp DESC`, start, end,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanAuditEvents(rows)
}

// FindErrors retrieves audit events that represent errors
func (r *PgAuditRepository) FindErrors(limit int) ([]*entities.AuditEvent, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT timestamp, event_type, user_id, username, tenant_id, action, resource, resource_id,
		        method, path, status_code, duration, ip_address, user_agent, error, metadata
		 FROM control_plane.audit_events
		 WHERE status_code >= 400 OR error != ''
		 ORDER BY timestamp DESC
		 LIMIT $1`, limit,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanAuditEvents(rows)
}

func scanAuditEvents(rows pgx.Rows) ([]*entities.AuditEvent, error) {
	var result []*entities.AuditEvent
	for rows.Next() {
		var ae entities.AuditEvent
		var metadata []byte

		if err := rows.Scan(
			&ae.Timestamp, &ae.EventType, &ae.UserID, &ae.Username, &ae.TenantID,
			&ae.Action, &ae.Resource, &ae.ResourceID, &ae.Method, &ae.Path,
			&ae.StatusCode, &ae.Duration, &ae.IPAddress, &ae.UserAgent, &ae.Error, &metadata,
		); err != nil {
			return nil, err
		}

		if err := json.Unmarshal(metadata, &ae.Metadata); err != nil {
			return nil, err
		}

		result = append(result, &ae)
	}
	return result, rows.Err()
}
