package persistence

import (
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"sync"
)

// MemoryUserRepository is an in-memory implementation of UserRepository
type MemoryUserRepository struct {
	users      map[string]*entities.User
	byUsername map[string]*entities.User
	mu         sync.RWMutex
}

// NewMemoryUserRepository creates a new MemoryUserRepository
func NewMemoryUserRepository() *MemoryUserRepository {
	return &MemoryUserRepository{
		users:      make(map[string]*entities.User),
		byUsername: make(map[string]*entities.User),
	}
}

// Save persists a user
func (r *MemoryUserRepository) Save(user *entities.User) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	r.users[user.ID] = user
	r.byUsername[user.Username] = user
	return nil
}

// FindByID retrieves a user by ID
func (r *MemoryUserRepository) FindByID(id string) (*entities.User, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	user, exists := r.users[id]
	if !exists {
		return nil, domain.ErrUserNotFound
	}

	return user, nil
}

// FindByUsername retrieves a user by username
func (r *MemoryUserRepository) FindByUsername(username string) (*entities.User, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	user, exists := r.byUsername[username]
	if !exists {
		return nil, domain.ErrUserNotFound
	}

	return user, nil
}

// FindByTenant retrieves all users for a tenant
func (r *MemoryUserRepository) FindByTenant(tenantID string) ([]*entities.User, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	result := make([]*entities.User, 0)
	for _, user := range r.users {
		if user.TenantID == tenantID {
			result = append(result, user)
		}
	}

	return result, nil
}

// Update updates an existing user
func (r *MemoryUserRepository) Update(user *entities.User) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if _, exists := r.users[user.ID]; !exists {
		return domain.ErrUserNotFound
	}

	r.users[user.ID] = user
	r.byUsername[user.Username] = user
	return nil
}

// Delete removes a user
func (r *MemoryUserRepository) Delete(id string) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	user, exists := r.users[id]
	if !exists {
		return domain.ErrUserNotFound
	}

	delete(r.users, id)
	delete(r.byUsername, user.Username)
	return nil
}

// Exists checks if a user exists
func (r *MemoryUserRepository) Exists(id string) (bool, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	_, exists := r.users[id]
	return exists, nil
}
