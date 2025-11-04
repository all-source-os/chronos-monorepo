package persistence

import (
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"sync"
)

// MemoryPolicyRepository is an in-memory implementation of PolicyRepository
type MemoryPolicyRepository struct {
	policies map[string]*entities.Policy
	mu       sync.RWMutex
}

// NewMemoryPolicyRepository creates a new MemoryPolicyRepository
func NewMemoryPolicyRepository() *MemoryPolicyRepository {
	repo := &MemoryPolicyRepository{
		policies: make(map[string]*entities.Policy),
	}
	repo.addDefaultPolicies()
	return repo
}

// addDefaultPolicies adds default security policies
func (r *MemoryPolicyRepository) addDefaultPolicies() {
	// Policy 1: Prevent deletion of default tenant
	policy1, _ := entities.NewPolicy(
		"prevent-default-tenant-deletion",
		"Prevent Default Tenant Deletion",
		"Prevents deletion of the default tenant",
		"tenant",
		entities.ActionDeny,
		100,
	)
	_ = policy1.AddCondition("tenant_id", "eq", "default")
	_ = policy1.AddCondition("operation", "eq", "delete")
	_ = r.Save(policy1)

	// Policy 2: Require admin for tenant creation
	policy2, _ := entities.NewPolicy(
		"require-admin-tenant-create",
		"Require Admin for Tenant Creation",
		"Only admins can create new tenants",
		"tenant",
		entities.ActionDeny,
		90,
	)
	_ = policy2.AddCondition("operation", "eq", "create")
	_ = policy2.AddCondition("role", "ne", "Admin")
	_ = r.Save(policy2)
}

// Save persists a policy
func (r *MemoryPolicyRepository) Save(policy *entities.Policy) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	r.policies[policy.ID] = policy
	return nil
}

// FindByID retrieves a policy by ID
func (r *MemoryPolicyRepository) FindByID(id string) (*entities.Policy, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	policy, exists := r.policies[id]
	if !exists {
		return nil, domain.ErrPolicyNotFound
	}

	return policy, nil
}

// FindAll retrieves all policies
func (r *MemoryPolicyRepository) FindAll() ([]*entities.Policy, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.Policy, 0, len(r.policies))
	for _, policy := range r.policies {
		result = append(result, policy)
	}

	return result, nil
}

// FindByResource retrieves policies for a specific resource
func (r *MemoryPolicyRepository) FindByResource(resource string) ([]*entities.Policy, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.Policy, 0)
	for _, policy := range r.policies {
		if policy.Resource == resource {
			result = append(result, policy)
		}
	}

	return result, nil
}

// FindEnabled retrieves all enabled policies
func (r *MemoryPolicyRepository) FindEnabled() ([]*entities.Policy, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.Policy, 0)
	for _, policy := range r.policies {
		if policy.Enabled {
			result = append(result, policy)
		}
	}

	return result, nil
}

// Update updates an existing policy
func (r *MemoryPolicyRepository) Update(policy *entities.Policy) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.policies[policy.ID]; !exists {
		return domain.ErrPolicyNotFound
	}

	r.policies[policy.ID] = policy
	return nil
}

// Delete removes a policy
func (r *MemoryPolicyRepository) Delete(id string) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.policies[id]; !exists {
		return domain.ErrPolicyNotFound
	}

	delete(r.policies, id)
	return nil
}

// Exists checks if a policy exists
func (r *MemoryPolicyRepository) Exists(id string) (bool, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	_, exists := r.policies[id]
	return exists, nil
}
