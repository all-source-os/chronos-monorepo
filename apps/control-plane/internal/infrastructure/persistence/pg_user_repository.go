package persistence

import (
	"context"
	"errors"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

// PgUserRepository is a PostgreSQL implementation of UserRepository
type PgUserRepository struct {
	pool *pgxpool.Pool
}

// NewPgUserRepository creates a new PgUserRepository
func NewPgUserRepository(pool *pgxpool.Pool) *PgUserRepository {
	return &PgUserRepository{pool: pool}
}

// Save persists a user
func (r *PgUserRepository) Save(user *entities.User) error {
	_, err := r.pool.Exec(context.Background(),
		`INSERT INTO control_plane.users (id, username, tenant_id, role, is_api_key, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7)`,
		user.ID, user.Username, user.TenantID, string(user.Role), user.IsAPIKey, user.CreatedAt, user.UpdatedAt,
	)
	return err
}

// FindByID retrieves a user by ID
func (r *PgUserRepository) FindByID(id string) (*entities.User, error) {
	var u entities.User
	var role string

	err := r.pool.QueryRow(context.Background(),
		`SELECT id, username, tenant_id, role, is_api_key, created_at, updated_at
		 FROM control_plane.users WHERE id = $1`, id,
	).Scan(&u.ID, &u.Username, &u.TenantID, &role, &u.IsAPIKey, &u.CreatedAt, &u.UpdatedAt)

	if errors.Is(err, pgx.ErrNoRows) {
		return nil, domain.ErrUserNotFound
	}
	if err != nil {
		return nil, err
	}

	u.Role = entities.Role(role)
	return &u, nil
}

// FindByUsername retrieves a user by username
func (r *PgUserRepository) FindByUsername(username string) (*entities.User, error) {
	var u entities.User
	var role string

	err := r.pool.QueryRow(context.Background(),
		`SELECT id, username, tenant_id, role, is_api_key, created_at, updated_at
		 FROM control_plane.users WHERE username = $1`, username,
	).Scan(&u.ID, &u.Username, &u.TenantID, &role, &u.IsAPIKey, &u.CreatedAt, &u.UpdatedAt)

	if errors.Is(err, pgx.ErrNoRows) {
		return nil, domain.ErrUserNotFound
	}
	if err != nil {
		return nil, err
	}

	u.Role = entities.Role(role)
	return &u, nil
}

// FindByTenant retrieves all users for a tenant
func (r *PgUserRepository) FindByTenant(tenantID string) ([]*entities.User, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, username, tenant_id, role, is_api_key, created_at, updated_at
		 FROM control_plane.users WHERE tenant_id = $1`, tenantID,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var result []*entities.User
	for rows.Next() {
		var u entities.User
		var role string

		if err := rows.Scan(&u.ID, &u.Username, &u.TenantID, &role, &u.IsAPIKey, &u.CreatedAt, &u.UpdatedAt); err != nil {
			return nil, err
		}

		u.Role = entities.Role(role)
		result = append(result, &u)
	}
	return result, rows.Err()
}

// Update updates an existing user
func (r *PgUserRepository) Update(user *entities.User) error {
	ct, err := r.pool.Exec(context.Background(),
		`UPDATE control_plane.users
		 SET username = $1, tenant_id = $2, role = $3, is_api_key = $4, updated_at = $5
		 WHERE id = $6`,
		user.Username, user.TenantID, string(user.Role), user.IsAPIKey, user.UpdatedAt, user.ID,
	)
	if err != nil {
		return err
	}
	if ct.RowsAffected() == 0 {
		return domain.ErrUserNotFound
	}
	return nil
}

// Delete removes a user
func (r *PgUserRepository) Delete(id string) error {
	ct, err := r.pool.Exec(context.Background(),
		`DELETE FROM control_plane.users WHERE id = $1`, id,
	)
	if err != nil {
		return err
	}
	if ct.RowsAffected() == 0 {
		return domain.ErrUserNotFound
	}
	return nil
}

// Exists checks if a user exists
func (r *PgUserRepository) Exists(id string) (bool, error) {
	var exists bool
	err := r.pool.QueryRow(context.Background(),
		`SELECT EXISTS(SELECT 1 FROM control_plane.users WHERE id = $1)`, id,
	).Scan(&exists)
	return exists, err
}
