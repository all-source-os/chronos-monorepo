package repositories

import "github.com/allsource/control-plane/internal/domain/entities"

// UserRepository defines the interface for user persistence
type UserRepository interface {
	// Save persists a user
	Save(user *entities.User) error

	// FindByID retrieves a user by ID
	FindByID(id string) (*entities.User, error)

	// FindByUsername retrieves a user by username
	FindByUsername(username string) (*entities.User, error)

	// FindByTenant retrieves all users for a tenant
	FindByTenant(tenantID string) ([]*entities.User, error)

	// Update updates an existing user
	Update(user *entities.User) error

	// Delete removes a user
	Delete(id string) error

	// Exists checks if a user exists
	Exists(id string) (bool, error)
}
