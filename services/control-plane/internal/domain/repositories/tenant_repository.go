package repositories

import "github.com/allsource/control-plane/internal/domain/entities"

// TenantRepository defines the interface for tenant persistence
type TenantRepository interface {
	// Save persists a tenant
	Save(tenant *entities.Tenant) error

	// FindByID retrieves a tenant by ID
	FindByID(id string) (*entities.Tenant, error)

	// FindAll retrieves all tenants
	FindAll() ([]*entities.Tenant, error)

	// FindActive retrieves all active tenants
	FindActive() ([]*entities.Tenant, error)

	// Update updates an existing tenant
	Update(tenant *entities.Tenant) error

	// Delete removes a tenant
	Delete(id string) error

	// Exists checks if a tenant exists
	Exists(id string) (bool, error)
}
