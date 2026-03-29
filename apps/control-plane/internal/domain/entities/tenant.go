package entities

import (
	"errors"
	"strings"
	"time"
)

// TenantQuotas holds the resource limits for a tenant.
type TenantQuotas struct {
	EventLimit     int64 `json:"event_limit"`
	QueryLimit     int64 `json:"query_limit"`
	StorageLimitMB int64 `json:"storage_limit_mb"`
}

// Validate checks that all quota values are non-negative.
func (q *TenantQuotas) Validate() error {
	if q.EventLimit < 0 {
		return errors.New("event_limit must not be negative")
	}
	if q.QueryLimit < 0 {
		return errors.New("query_limit must not be negative")
	}
	if q.StorageLimitMB < 0 {
		return errors.New("storage_limit_mb must not be negative")
	}
	return nil
}

// Tenant represents a tenant in the system
type Tenant struct {
	ID          string
	Name        string
	Description string
	Status      TenantStatus
	Quotas      TenantQuotas
	CreatedAt   time.Time
	UpdatedAt   time.Time
	Metadata    map[string]interface{}
}

// TenantStatus represents the status of a tenant
type TenantStatus string

// TenantStatus constants define the possible statuses of a tenant.
const (
	TenantStatusActive    TenantStatus = "active"
	TenantStatusSuspended TenantStatus = "suspended"
	TenantStatusArchived  TenantStatus = "archived"
	TenantStatusDeleted   TenantStatus = "deleted"
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

// TenantSlug generates a URL-safe slug from a raw name.
// Used by onboarding, agent registration, and demo flows.
func TenantSlug(raw string) string {
	s := strings.ToLower(raw)
	s = strings.ReplaceAll(s, "@", "-at-")
	s = strings.ReplaceAll(s, ".", "-")
	s = strings.ReplaceAll(s, " ", "-")
	return s
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

// Archive marks tenant as archived
func (t *Tenant) Archive() {
	t.Status = TenantStatusArchived
	t.UpdatedAt = time.Now()
}

// Activate marks tenant as active
func (t *Tenant) Activate() {
	t.Status = TenantStatusActive
	t.UpdatedAt = time.Now()
}

// UpdateQuotas sets new quota values after validation.
func (t *Tenant) UpdateQuotas(quotas TenantQuotas) error {
	if err := quotas.Validate(); err != nil {
		return err
	}
	t.Quotas = quotas
	t.UpdatedAt = time.Now()
	return nil
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
