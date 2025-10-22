package entities

import (
	"errors"
	"time"
)

// Tenant represents a tenant in the system
type Tenant struct {
	ID          string
	Name        string
	Description string
	Status      TenantStatus
	CreatedAt   time.Time
	UpdatedAt   time.Time
	Metadata    map[string]interface{}
}

// TenantStatus represents the status of a tenant
type TenantStatus string

const (
	TenantStatusActive   TenantStatus = "active"
	TenantStatusSuspended TenantStatus = "suspended"
	TenantStatusDeleted  TenantStatus = "deleted"
)

// NewTenant creates a new tenant with validation
func NewTenant(id, name, description string) (*Tenant, error) {
	if err := ValidateTenantID(id); err != nil {
		return nil, err
	}
	if err := ValidateTenantName(name); err != nil {
		return nil, err
	}

	now := time.Now()
	return &Tenant{
		ID:          id,
		Name:        name,
		Description: description,
		Status:      TenantStatusActive,
		CreatedAt:   now,
		UpdatedAt:   now,
		Metadata:    make(map[string]interface{}),
	}, nil
}

// ValidateTenantID validates a tenant ID
func ValidateTenantID(id string) error {
	if id == "" {
		return errors.New("tenant ID cannot be empty")
	}
	if len(id) > 255 {
		return errors.New("tenant ID too long")
	}
	return nil
}

// ValidateTenantName validates a tenant name
func ValidateTenantName(name string) error {
	if name == "" {
		return errors.New("tenant name cannot be empty")
	}
	if len(name) > 255 {
		return errors.New("tenant name too long")
	}
	return nil
}

// IsActive checks if tenant is active
func (t *Tenant) IsActive() bool {
	return t.Status == TenantStatusActive
}

// Suspend marks tenant as suspended
func (t *Tenant) Suspend() {
	t.Status = TenantStatusSuspended
	t.UpdatedAt = time.Now()
}

// Activate marks tenant as active
func (t *Tenant) Activate() {
	t.Status = TenantStatusActive
	t.UpdatedAt = time.Now()
}

// MarkDeleted marks tenant as deleted
func (t *Tenant) MarkDeleted() error {
	if t.ID == "default" {
		return errors.New("cannot delete default tenant")
	}
	t.Status = TenantStatusDeleted
	t.UpdatedAt = time.Now()
	return nil
}
