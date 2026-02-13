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

// PgPolicyRepository is a PostgreSQL implementation of PolicyRepository
type PgPolicyRepository struct {
	pool *pgxpool.Pool
}

// NewPgPolicyRepository creates a new PgPolicyRepository
func NewPgPolicyRepository(pool *pgxpool.Pool) *PgPolicyRepository {
	return &PgPolicyRepository{pool: pool}
}

// Save persists a policy
func (r *PgPolicyRepository) Save(policy *entities.Policy) error {
	conditions, err := json.Marshal(policy.Conditions)
	if err != nil {
		return err
	}

	_, err = r.pool.Exec(context.Background(),
		`INSERT INTO control_plane.policies (id, name, description, resource, action, conditions, priority, enabled)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
		policy.ID, policy.Name, policy.Description, policy.Resource,
		string(policy.Action), conditions, policy.Priority, policy.Enabled,
	)
	return err
}

// FindByID retrieves a policy by ID
func (r *PgPolicyRepository) FindByID(id string) (*entities.Policy, error) {
	var p entities.Policy
	var action string
	var conditions []byte

	err := r.pool.QueryRow(context.Background(),
		`SELECT id, name, description, resource, action, conditions, priority, enabled
		 FROM control_plane.policies WHERE id = $1`, id,
	).Scan(&p.ID, &p.Name, &p.Description, &p.Resource, &action, &conditions, &p.Priority, &p.Enabled)

	if errors.Is(err, pgx.ErrNoRows) {
		return nil, domain.ErrPolicyNotFound
	}
	if err != nil {
		return nil, err
	}

	p.Action = entities.PolicyAction(action)
	if err := json.Unmarshal(conditions, &p.Conditions); err != nil {
		return nil, err
	}

	return &p, nil
}

// FindAll retrieves all policies
func (r *PgPolicyRepository) FindAll() ([]*entities.Policy, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, name, description, resource, action, conditions, priority, enabled
		 FROM control_plane.policies`,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanPolicies(rows)
}

// FindByResource retrieves policies for a specific resource
func (r *PgPolicyRepository) FindByResource(resource string) ([]*entities.Policy, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, name, description, resource, action, conditions, priority, enabled
		 FROM control_plane.policies WHERE resource = $1`, resource,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanPolicies(rows)
}

// FindEnabled retrieves all enabled policies
func (r *PgPolicyRepository) FindEnabled() ([]*entities.Policy, error) {
	rows, err := r.pool.Query(context.Background(),
		`SELECT id, name, description, resource, action, conditions, priority, enabled
		 FROM control_plane.policies WHERE enabled = TRUE`,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	return scanPolicies(rows)
}

// Update updates an existing policy
func (r *PgPolicyRepository) Update(policy *entities.Policy) error {
	conditions, err := json.Marshal(policy.Conditions)
	if err != nil {
		return err
	}

	ct, err := r.pool.Exec(context.Background(),
		`UPDATE control_plane.policies
		 SET name = $1, description = $2, resource = $3, action = $4, conditions = $5, priority = $6, enabled = $7, updated_at = NOW()
		 WHERE id = $8`,
		policy.Name, policy.Description, policy.Resource, string(policy.Action),
		conditions, policy.Priority, policy.Enabled, policy.ID,
	)
	if err != nil {
		return err
	}
	if ct.RowsAffected() == 0 {
		return domain.ErrPolicyNotFound
	}
	return nil
}

// Delete removes a policy
func (r *PgPolicyRepository) Delete(id string) error {
	ct, err := r.pool.Exec(context.Background(),
		`DELETE FROM control_plane.policies WHERE id = $1`, id,
	)
	if err != nil {
		return err
	}
	if ct.RowsAffected() == 0 {
		return domain.ErrPolicyNotFound
	}
	return nil
}

// Exists checks if a policy exists
func (r *PgPolicyRepository) Exists(id string) (bool, error) {
	var exists bool
	err := r.pool.QueryRow(context.Background(),
		`SELECT EXISTS(SELECT 1 FROM control_plane.policies WHERE id = $1)`, id,
	).Scan(&exists)
	return exists, err
}

func scanPolicies(rows pgx.Rows) ([]*entities.Policy, error) {
	var result []*entities.Policy
	for rows.Next() {
		var p entities.Policy
		var action string
		var conditions []byte

		if err := rows.Scan(&p.ID, &p.Name, &p.Description, &p.Resource, &action, &conditions, &p.Priority, &p.Enabled); err != nil {
			return nil, err
		}

		p.Action = entities.PolicyAction(action)
		if err := json.Unmarshal(conditions, &p.Conditions); err != nil {
			return nil, err
		}

		result = append(result, &p)
	}
	return result, rows.Err()
}
