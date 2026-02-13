package persistence

import (
	"context"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

// PgConfigRepository is a PostgreSQL implementation of ConfigRepository
type PgConfigRepository struct {
	pool *pgxpool.Pool
}

// NewPgConfigRepository creates a new PgConfigRepository
func NewPgConfigRepository(pool *pgxpool.Pool) *PgConfigRepository {
	return &PgConfigRepository{pool: pool}
}

// Save persists a config entry (upsert)
func (r *PgConfigRepository) Save(entry *entities.ConfigEntry) error {
	_, err := r.pool.Exec(context.Background(),
		`INSERT INTO control_plane.config_entries (key, value, description, category, updated_by, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7)
		 ON CONFLICT (key) DO UPDATE SET
		   value = EXCLUDED.value,
		   description = EXCLUDED.description,
		   category = EXCLUDED.category,
		   updated_by = EXCLUDED.updated_by,
		   updated_at = EXCLUDED.updated_at`,
		entry.Key, entry.Value, entry.Description, entry.Category,
		entry.UpdatedBy, entry.CreatedAt, entry.UpdatedAt,
	)
	return err
}

// FindByKey retrieves a config entry by key
func (r *PgConfigRepository) FindByKey(key string) (*entities.ConfigEntry, error) {
	var entry entities.ConfigEntry
	err := r.pool.QueryRow(context.Background(),
		`SELECT key, value, description, category, updated_by, created_at, updated_at
		 FROM control_plane.config_entries WHERE key = $1`, key,
	).Scan(&entry.Key, &entry.Value, &entry.Description, &entry.Category,
		&entry.UpdatedBy, &entry.CreatedAt, &entry.UpdatedAt)

	if err == pgx.ErrNoRows {
		return nil, domain.ErrConfigNotFound
	}
	if err != nil {
		return nil, err
	}
	return &entry, nil
}

// FindAll retrieves all config entries
func (r *PgConfigRepository) FindAll() ([]*entities.ConfigEntry, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT key, value, description, category, updated_by, created_at, updated_at
		 FROM control_plane.config_entries ORDER BY key`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanConfigEntries(rows)
}

// FindByCategory retrieves config entries by category
func (r *PgConfigRepository) FindByCategory(category string) ([]*entities.ConfigEntry, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT key, value, description, category, updated_by, created_at, updated_at
		 FROM control_plane.config_entries WHERE category = $1 ORDER BY key`, category)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanConfigEntries(rows)
}

// Delete removes a config entry by key
func (r *PgConfigRepository) Delete(key string) error {
	result, err := r.pool.Exec(context.Background(),
		`DELETE FROM control_plane.config_entries WHERE key = $1`, key)
	if err != nil {
		return err
	}
	if result.RowsAffected() == 0 {
		return domain.ErrConfigNotFound
	}
	return nil
}

// Exists checks if a config entry exists
func (r *PgConfigRepository) Exists(key string) (bool, error) {
	var exists bool
	err := r.pool.QueryRow(context.Background(),
		`SELECT EXISTS(SELECT 1 FROM control_plane.config_entries WHERE key = $1)`, key,
	).Scan(&exists)
	return exists, err
}

func scanConfigEntries(rows pgx.Rows) ([]*entities.ConfigEntry, error) {
	var result []*entities.ConfigEntry
	for rows.Next() {
		var entry entities.ConfigEntry
		if err := rows.Scan(
			&entry.Key, &entry.Value, &entry.Description, &entry.Category,
			&entry.UpdatedBy, &entry.CreatedAt, &entry.UpdatedAt,
		); err != nil {
			return nil, err
		}
		result = append(result, &entry)
	}
	return result, rows.Err()
}
