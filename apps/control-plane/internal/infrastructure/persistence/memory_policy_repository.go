package persistence

import (
	"sync"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
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
	policy1, _ := entities.NewPolicy( //nolint:errcheck // default policy initialization
		"prevent-default-tenant-deletion",
		"Prevent Default Tenant Deletion",
		"Prevents deletion of the default tenant",
		"tenant",
		entities.ActionDeny,
		100,
	)
	_ = policy1.AddCondition("tenant_id", "eq", "default") //nolint:errcheck // default policy initialization
	_ = policy1.AddCondition("operation", "eq", "delete")  //nolint:errcheck // default policy initialization
	_ = r.Save(policy1)                                    //nolint:errcheck // default policy initialization

	// Policy 2: Require admin for tenant creation
	policy2, _ := entities.NewPolicy( //nolint:errcheck // default policy initialization
		"require-admin-tenant-create",
		"Require Admin for Tenant Creation",
		"Only admins can create new tenants",
		"tenant",
		entities.ActionDeny,
		90,
	)
	_ = policy2.AddCondition("operation", "eq", "create") //nolint:errcheck // default policy initialization
	_ = policy2.AddCondition("role", "ne", "admin")       //nolint:errcheck // default policy initialization
	_ = r.Save(policy2)                                   //nolint:errcheck // default policy initialization

	// Policy 3: Warn on large operations
	policy3, _ := entities.NewPolicy( //nolint:errcheck // default policy initialization
		"warn-large-operations",
		"Warn on Large Operations",
		"Warn when operations affect more than 10000 records",
		"operation",
		entities.ActionWarn,
		80,
	)
	_ = policy3.AddCondition("record_count", "gt", "10000") //nolint:errcheck // default policy initialization
	_ = r.Save(policy3)                                     //nolint:errcheck // default policy initialization

	// Policy 4: Prevent self-deletion
	policy4, _ := entities.NewPolicy( //nolint:errcheck // default policy initialization
		"prevent-self-deletion",
		"Prevent Self Deletion",
		"Users cannot delete their own account",
		"user",
		entities.ActionDeny,
		95,
	)
	_ = policy4.AddCondition("target_id", "eq", "actor_id") //nolint:errcheck // default policy initialization
	_ = policy4.AddCondition("operation", "eq", "delete")   //nolint:errcheck // default policy initialization
	_ = r.Save(policy4)                                     //nolint:errcheck // default policy initialization

	// Policy 5: Rate-limit expensive operations
	policy5, _ := entities.NewPolicy( //nolint:errcheck // default policy initialization
		"rate-limit-expensive-ops",
		"Rate Limit Expensive Operations",
		"Warn on snapshot, backup, and restore operations",
		"operation",
		entities.ActionWarn,
		70,
	)
	_ = policy5.AddCondition("type", "in", "snapshot,backup,restore") //nolint:errcheck // default policy initialization
	_ = r.Save(policy5)                                               //nolint:errcheck // default policy initialization
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
