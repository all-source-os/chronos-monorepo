package persistence

import (
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"sync"
)

// MemoryTenantRepository is an in-memory implementation of TenantRepository
type MemoryTenantRepository struct {
	tenants map[string]*entities.Tenant
	mu      sync.RWMutex
}

// NewMemoryTenantRepository creates a new MemoryTenantRepository
func NewMemoryTenantRepository() *MemoryTenantRepository {
	return &MemoryTenantRepository{
		tenants: make(map[string]*entities.Tenant),
	}
}

// Save persists a tenant
func (r *MemoryTenantRepository) Save(tenant *entities.Tenant) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	r.tenants[tenant.ID] = tenant
	return nil
}

// FindByID retrieves a tenant by ID
func (r *MemoryTenantRepository) FindByID(id string) (*entities.Tenant, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	tenant, exists := r.tenants[id]
	if !exists {
		return nil, domain.ErrTenantNotFound
	}

	return tenant, nil
}

// FindAll retrieves all tenants
func (r *MemoryTenantRepository) FindAll() ([]*entities.Tenant, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.Tenant, 0, len(r.tenants))
	for _, tenant := range r.tenants {
		result = append(result, tenant)
	}

	return result, nil
}

// FindActive retrieves all active tenants
func (r *MemoryTenantRepository) FindActive() ([]*entities.Tenant, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.Tenant, 0)
	for _, tenant := range r.tenants {
		if tenant.IsActive() {
			result = append(result, tenant)
		}
	}

	return result, nil
}

// Update updates an existing tenant
func (r *MemoryTenantRepository) Update(tenant *entities.Tenant) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.tenants[tenant.ID]; !exists {
		return domain.ErrTenantNotFound
	}

	r.tenants[tenant.ID] = tenant
	return nil
}

// Delete removes a tenant
func (r *MemoryTenantRepository) Delete(id string) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.tenants[id]; !exists {
		return domain.ErrTenantNotFound
	}

	delete(r.tenants, id)
	return nil
}

// Exists checks if a tenant exists
func (r *MemoryTenantRepository) Exists(id string) (bool, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	_, exists := r.tenants[id]
	return exists, nil
}
