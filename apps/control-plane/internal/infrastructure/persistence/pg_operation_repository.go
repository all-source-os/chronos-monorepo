package persistence

import (
	"context"
	"encoding/json"
	"errors"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

// PgOperationRepository is a PostgreSQL implementation of OperationRepository
type PgOperationRepository struct {
	pool *pgxpool.Pool
}

// NewPgOperationRepository creates a new PgOperationRepository
func NewPgOperationRepository(pool *pgxpool.Pool) *PgOperationRepository {
	return &PgOperationRepository{pool: pool}
}

// Save persists an operation
func (r *PgOperationRepository) Save(op *entities.Operation) error {
	params, err := json.Marshal(op.Parameters)
	if err != nil {
		return err
	}
	result, err := json.Marshal(op.Result)
	if err != nil {
		return err
	}

	_, err = r.pool.Exec(context.Background(),
		`INSERT INTO control_plane.operations (id, type, status, tenant_id, parameters, result, initiated_by, started_at, completed_at, error, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)`,
		op.ID, string(op.Type), string(op.Status), op.TenantID, params, result, op.InitiatedBy, op.StartedAt, op.CompletedAt, op.Error, op.CreatedAt,
	)
	return err
}

// FindByID retrieves an operation by ID
func (r *PgOperationRepository) FindByID(id string) (*entities.Operation, error) {
	var op entities.Operation
	var opType, status string
	var params, result []byte

	err := r.pool.QueryRow(context.Background(),
		`SELECT id, type, status, tenant_id, parameters, result, initiated_by, started_at, completed_at, error, created_at
		 FROM control_plane.operations WHERE id = $1`, id,
	).Scan(&op.ID, &opType, &status, &op.TenantID, &params, &result, &op.InitiatedBy, &op.StartedAt, &op.CompletedAt, &op.Error, &op.CreatedAt)

	if errors.Is(err, pgx.ErrNoRows) {
		return nil, domain.ErrOperationNotFound
	}
	if err != nil {
		return nil, err
	}

	op.Type = entities.OperationType(opType)
	op.Status = entities.OperationStatus(status)
	if err := json.Unmarshal(params, &op.Parameters); err != nil {
		return nil, err
	}
	if err := json.Unmarshal(result, &op.Result); err != nil {
		return nil, err
	}

	return &op, nil
}

// FindByType retrieves operations by type
func (r *PgOperationRepository) FindByType(opType entities.OperationType) ([]*entities.Operation, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, type, status, tenant_id, parameters, result, initiated_by, started_at, completed_at, error, created_at
		 FROM control_plane.operations WHERE type = $1 ORDER BY created_at DESC`,
		string(opType),
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanOperations(rows)
}

// FindByStatus retrieves operations by status
func (r *PgOperationRepository) FindByStatus(status entities.OperationStatus) ([]*entities.Operation, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, type, status, tenant_id, parameters, result, initiated_by, started_at, completed_at, error, created_at
		 FROM control_plane.operations WHERE status = $1 ORDER BY created_at DESC`,
		string(status),
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanOperations(rows)
}

// FindByTenant retrieves operations by tenant ID
func (r *PgOperationRepository) FindByTenant(tenantID string) ([]*entities.Operation, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, type, status, tenant_id, parameters, result, initiated_by, started_at, completed_at, error, created_at
		 FROM control_plane.operations WHERE tenant_id = $1 ORDER BY created_at DESC`,
		tenantID,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanOperations(rows)
}

// Update updates an existing operation
func (r *PgOperationRepository) Update(op *entities.Operation) error {
	params, err := json.Marshal(op.Parameters)
	if err != nil {
		return err
	}
	result, err := json.Marshal(op.Result)
	if err != nil {
		return err
	}

	ct, err := r.pool.Exec(context.Background(),
		`UPDATE control_plane.operations
		 SET type = $1, status = $2, tenant_id = $3, parameters = $4, result = $5, initiated_by = $6, started_at = $7, completed_at = $8, error = $9
		 WHERE id = $10`,
		string(op.Type), string(op.Status), op.TenantID, params, result, op.InitiatedBy, op.StartedAt, op.CompletedAt, op.Error, op.ID,
	)
	if err != nil {
		return err
	}
	if ct.RowsAffected() == 0 {
		return domain.ErrOperationNotFound
	}
	return nil
}

// FindRecent retrieves the most recent operations
func (r *PgOperationRepository) FindRecent(limit int) ([]*entities.Operation, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, type, status, tenant_id, parameters, result, initiated_by, started_at, completed_at, error, created_at
		 FROM control_plane.operations ORDER BY created_at DESC LIMIT $1`,
		limit,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanOperations(rows)
}

func scanOperations(rows pgx.Rows) ([]*entities.Operation, error) {
	var ops []*entities.Operation
	for rows.Next() {
		var op entities.Operation
		var opType, status string
		var params, result []byte

		if err := rows.Scan(&op.ID, &opType, &status, &op.TenantID, &params, &result, &op.InitiatedBy, &op.StartedAt, &op.CompletedAt, &op.Error, &op.CreatedAt); err != nil {
			return nil, err
		}

		op.Type = entities.OperationType(opType)
		op.Status = entities.OperationStatus(status)
		if err := json.Unmarshal(params, &op.Parameters); err != nil {
			return nil, err
		}
		if err := json.Unmarshal(result, &op.Result); err != nil {
			return nil, err
		}

		ops = append(ops, &op)
	}
	return ops, rows.Err()
}
