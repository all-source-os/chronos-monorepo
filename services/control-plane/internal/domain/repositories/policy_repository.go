package repositories

import "github.com/allsource/control-plane/internal/domain/entities"

// PolicyRepository defines the interface for policy persistence
type PolicyRepository interface {
	// Save persists a policy
	Save(policy *entities.Policy) error

	// FindByID retrieves a policy by ID
	FindByID(id string) (*entities.Policy, error)

	// FindAll retrieves all policies
	FindAll() ([]*entities.Policy, error)

	// FindByResource retrieves policies for a specific resource
	FindByResource(resource string) ([]*entities.Policy, error)

	// FindEnabled retrieves all enabled policies
	FindEnabled() ([]*entities.Policy, error)

	// Update updates an existing policy
	Update(policy *entities.Policy) error

	// Delete removes a policy
	Delete(id string) error

	// Exists checks if a policy exists
	Exists(id string) (bool, error)
}
