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
	// HomeRegion is the Fly region that owns writes for this tenant
	// (Step 1 of docs/proposals/REGIONAL_INDEPENDENCE.md). All other
	// regions hold async-replicated read replicas; the Control Plane
	// routes writes here and reads to the nearest replica. Empty
	// string means "fall back to the default region" — the routing
	// layer treats it as the platform's home (iad today).
	HomeRegion string
	CreatedAt  time.Time
	UpdatedAt  time.Time
	Metadata   map[string]interface{}
}

// DefaultHomeRegion is the platform's home region. Existing tenants
// without an explicit HomeRegion are treated as homed here; new
// tenants without an explicit region get this as their default.
const DefaultHomeRegion = "iad"

// allowedHomeRegions is the closed set of regions a tenant may be
// homed in. Tied to the regions where Core followers are deployed —
// extending this list without extending the Fly topology will result
// in routing requests at URLs that don't exist. Update both at once.
var allowedHomeRegions = map[string]struct{}{
	"iad": {}, // US East (Virginia) — current platform home
	"lhr": {}, // EU West (London)
	"ord": {}, // US Central (Chicago)
	"fra": {}, // EU Central (Frankfurt)
	"syd": {}, // AU East (Sydney)
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

// NewTenant creates a new tenant with validation. The home region
// defaults to DefaultHomeRegion; use NewTenantInRegion to pin a tenant
// to a specific region at creation time.
func NewTenant(id, name, description string) (*Tenant, error) {
	return NewTenantInRegion(id, name, description, DefaultHomeRegion)
}

// NewTenantInRegion creates a tenant explicitly homed in a region.
// Returns an error if homeRegion is not in the allowlist; pass an
// empty string to use DefaultHomeRegion.
func NewTenantInRegion(id, name, description, homeRegion string) (*Tenant, error) {
	if err := ValidateTenantID(id); err != nil {
		return nil, err
	}
	if err := ValidateTenantName(name); err != nil {
		return nil, err
	}
	if homeRegion == "" {
		homeRegion = DefaultHomeRegion
	}
	if err := ValidateHomeRegion(homeRegion); err != nil {
		return nil, err
	}

	now := time.Now()
	return &Tenant{
		ID:          id,
		Name:        name,
		Description: description,
		Status:      TenantStatusActive,
		HomeRegion:  homeRegion,
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

// ValidateHomeRegion checks that a region is one we can route to.
// Empty string is rejected — callers should resolve to
// DefaultHomeRegion before validating.
func ValidateHomeRegion(region string) error {
	if region == "" {
		return errors.New("home region cannot be empty")
	}
	if _, ok := allowedHomeRegions[region]; !ok {
		return errors.New("home region " + region + " is not in the allowlist")
	}
	return nil
}

// EffectiveHomeRegion returns the tenant's home region, falling back
// to DefaultHomeRegion when the field is empty (legacy tenants
// migrated before the field existed).
func (t *Tenant) EffectiveHomeRegion() string {
	if t.HomeRegion == "" {
		return DefaultHomeRegion
	}
	return t.HomeRegion
}

// SetHomeRegion changes the tenant's home region after validation.
// Returns an error if the region is not in the allowlist. Note that
// changing a tenant's home region in a multi-region deployment
// requires draining and re-replicating their data; this method only
// updates the metadata. See REGIONAL_INDEPENDENCE.md "Open
// questions" for the migration playbook (TBD).
func (t *Tenant) SetHomeRegion(region string) error {
	if err := ValidateHomeRegion(region); err != nil {
		return err
	}
	t.HomeRegion = region
	t.UpdatedAt = time.Now()
	return nil
}

// TenantSlug generates a URL-safe slug from a raw name.
// Used by onboarding, agent registration, and demo flows.
// Output matches Core's tenant ID validation: alphanumeric, hyphens, underscores.
func TenantSlug(raw string) string {
	s := strings.ToLower(raw)
	s = strings.ReplaceAll(s, "@", "-at-")
	s = strings.ReplaceAll(s, ".", "-")
	s = strings.ReplaceAll(s, " ", "-")
	// Strip any remaining invalid characters (colons from "oauth:github:12345",
	// plus any other punctuation) to satisfy Core's tenant ID regex.
	var b strings.Builder
	for _, r := range s {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') || r == '-' || r == '_' {
			b.WriteRune(r)
		} else {
			b.WriteRune('-')
		}
	}
	// Collapse consecutive hyphens
	result := b.String()
	for strings.Contains(result, "--") {
		result = strings.ReplaceAll(result, "--", "-")
	}
	return strings.Trim(result, "-")
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
