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

// PgTenantRepository is a PostgreSQL implementation of TenantRepository
type PgTenantRepository struct {
	pool *pgxpool.Pool
}

// NewPgTenantRepository creates a new PgTenantRepository
func NewPgTenantRepository(pool *pgxpool.Pool) *PgTenantRepository {
	return &PgTenantRepository{pool: pool}
}

// Save persists a tenant
func (r *PgTenantRepository) Save(tenant *entities.Tenant) error {
	metadata, err := json.Marshal(tenant.Metadata)
	if err != nil {
		return err
	}

	_, err = r.pool.Exec(context.Background(),
		`INSERT INTO control_plane.tenants (id, name, description, status, metadata, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7)`,
		tenant.ID, tenant.Name, tenant.Description, string(tenant.Status), metadata, tenant.CreatedAt, tenant.UpdatedAt,
	)
	return err
}

// FindByID retrieves a tenant by ID
func (r *PgTenantRepository) FindByID(id string) (*entities.Tenant, error) {
	var t entities.Tenant
	var status string
	var metadata []byte

	err := r.pool.QueryRow(context.Background(),
		`SELECT id, name, description, status, metadata, created_at, updated_at
		 FROM control_plane.tenants WHERE id = $1`, id,
	).Scan(&t.ID, &t.Name, &t.Description, &status, &metadata, &t.CreatedAt, &t.UpdatedAt)

	if errors.Is(err, pgx.ErrNoRows) {
		return nil, domain.ErrTenantNotFound
	}
	if err != nil {
		return nil, err
	}

	t.Status = entities.TenantStatus(status)
	if err := json.Unmarshal(metadata, &t.Metadata); err != nil {
		return nil, err
	}

	return &t, nil
}

// FindAll retrieves all tenants
func (r *PgTenantRepository) FindAll() ([]*entities.Tenant, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, name, description, status, metadata, created_at, updated_at
		 FROM control_plane.tenants`,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanTenants(rows)
}

// FindActive retrieves all active tenants
func (r *PgTenantRepository) FindActive() ([]*entities.Tenant, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, name, description, status, metadata, created_at, updated_at
		 FROM control_plane.tenants WHERE status = $1`,
		string(entities.TenantStatusActive),
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanTenants(rows)
}

// Update updates an existing tenant
func (r *PgTenantRepository) Update(tenant *entities.Tenant) error {
	metadata, err := json.Marshal(tenant.Metadata)
	if err != nil {
		return err
	}

	ct, err := r.pool.Exec(context.Background(),
		`UPDATE control_plane.tenants
		 SET name = $1, description = $2, status = $3, metadata = $4, updated_at = $5
		 WHERE id = $6`,
		tenant.Name, tenant.Description, string(tenant.Status), metadata, tenant.UpdatedAt, tenant.ID,
	)
	if err != nil {
		return err
	}
	if ct.RowsAffected() == 0 {
		return domain.ErrTenantNotFound
	}
	return nil
}

// Delete removes a tenant
func (r *PgTenantRepository) Delete(id string) error {
	ct, err := r.pool.Exec(context.Background(),
		`DELETE FROM control_plane.tenants WHERE id = $1`, id,
	)
	if err != nil {
		return err
	}
	if ct.RowsAffected() == 0 {
		return domain.ErrTenantNotFound
	}
	return nil
}

// Exists checks if a tenant exists
func (r *PgTenantRepository) Exists(id string) (bool, error) {
	var exists bool
	err := r.pool.QueryRow(context.Background(),
		`SELECT EXISTS(SELECT 1 FROM control_plane.tenants WHERE id = $1)`, id,
	).Scan(&exists)
	return exists, err
}

func scanTenants(rows pgx.Rows) ([]*entities.Tenant, error) {
	var result []*entities.Tenant
	for rows.Next() {
		var t entities.Tenant
		var status string
		var metadata []byte

		if err := rows.Scan(&t.ID, &t.Name, &t.Description, &status, &metadata, &t.CreatedAt, &t.UpdatedAt); err != nil {
			return nil, err
		}

		t.Status = entities.TenantStatus(status)
		if err := json.Unmarshal(metadata, &t.Metadata); err != nil {
			return nil, err
		}

		result = append(result, &t)
	}
	return result, rows.Err()
}
