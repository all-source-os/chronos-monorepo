package persistence

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// CoreTenantRepository implements TenantRepository by delegating to Core via REST.
// Core is the single source of truth for tenant data — no dual-write.
type CoreTenantRepository struct {
	client clients.CoreClient
}

// NewCoreTenantRepository creates a new CoreTenantRepository.
func NewCoreTenantRepository(client clients.CoreClient) *CoreTenantRepository {
	return &CoreTenantRepository{client: client}
}

// Save persists a new tenant to Core. The tenant's HomeRegion is
// piggybacked onto Metadata under the key "home_region" so we don't
// need a Core schema change to round-trip it. The entity layer's
// EffectiveHomeRegion() handles tenants written before this field
// existed.
func (r *CoreTenantRepository) Save(tenant *entities.Tenant) error {
	ctx := context.Background()
	metadata := tenant.Metadata
	if metadata == nil {
		metadata = make(map[string]interface{})
	}
	if region := tenant.EffectiveHomeRegion(); region != "" {
		metadata["home_region"] = region
	}
	_, err := r.client.CreateTenant(ctx, clients.CreateTenantRequest{
		ID:       tenant.ID,
		Name:     tenant.Name,
		Metadata: metadata,
	})
	return err
}

// FindByID retrieves a tenant by ID from Core.
func (r *CoreTenantRepository) FindByID(id string) (*entities.Tenant, error) {
	ctx := context.Background()
	resp, err := r.client.GetTenant(ctx, id)
	if err != nil {
		return nil, domain.ErrTenantNotFound
	}
	return coreTenantToEntity(resp), nil
}

// FindAll retrieves all tenants from Core.
func (r *CoreTenantRepository) FindAll() ([]*entities.Tenant, error) {
	ctx := context.Background()
	resp, err := r.client.ListTenants(ctx)
	if err != nil {
		return nil, err
	}

	result := make([]*entities.Tenant, 0, len(resp.Tenants))
	for i := range resp.Tenants {
		result = append(result, coreTenantToEntity(&resp.Tenants[i]))
	}
	return result, nil
}

// FindActive retrieves all active tenants from Core.
func (r *CoreTenantRepository) FindActive() ([]*entities.Tenant, error) {
	all, err := r.FindAll()
	if err != nil {
		return nil, err
	}

	result := make([]*entities.Tenant, 0)
	for _, t := range all {
		if t.IsActive() {
			result = append(result, t)
		}
	}
	return result, nil
}

// Update updates a tenant in Core. Handles status changes and metadata updates.
func (r *CoreTenantRepository) Update(tenant *entities.Tenant) error {
	ctx := context.Background()

	// Record the lifecycle status in metadata so suspended-vs-archived survives the
	// round-trip: Core only has an `active` bool and cannot tell the two apart, so
	// without this hint a deactivated tenant would always read back as "suspended".
	// The Deactivate/Activate calls below flip Core's `active` flag; this just
	// preserves WHICH not-active state it is.
	if tenant.Metadata == nil {
		tenant.Metadata = map[string]any{}
	}
	switch tenant.Status {
	case entities.TenantStatusSuspended, entities.TenantStatusArchived:
		tenant.Metadata["lifecycle_status"] = string(tenant.Status)
	case entities.TenantStatusActive:
		delete(tenant.Metadata, "lifecycle_status")
	}

	// Update metadata if present
	if len(tenant.Metadata) > 0 {
		if _, err := r.client.UpdateTenantMetadata(ctx, tenant.ID, tenant.Metadata); err != nil {
			return err
		}
	}

	switch tenant.Status {
	case entities.TenantStatusSuspended:
		_, err := r.client.DeactivateTenant(ctx, tenant.ID)
		return err
	case entities.TenantStatusActive:
		_, err := r.client.ActivateTenant(ctx, tenant.ID)
		return err
	case entities.TenantStatusDeleted:
		return r.client.DeleteTenant(ctx, tenant.ID)
	case entities.TenantStatusArchived:
		_, err := r.client.DeactivateTenant(ctx, tenant.ID)
		return err
	default:
		return nil
	}
}

// Delete removes a tenant from Core.
func (r *CoreTenantRepository) Delete(id string) error {
	ctx := context.Background()
	return r.client.DeleteTenant(ctx, id)
}

// Exists checks if a tenant exists in Core.
func (r *CoreTenantRepository) Exists(id string) (bool, error) {
	ctx := context.Background()
	_, err := r.client.GetTenant(ctx, id)
	if err != nil {
		return false, nil //nolint:nilerr // not-found is not an error for Exists
	}
	return true, nil
}

// coreTenantToEntity converts a Core API tenant response to a domain entity.
func coreTenantToEntity(resp *clients.TenantResponse) *entities.Tenant {
	// Derive lifecycle status. Prefer an explicit Core `status` string if one is
	// ever sent; otherwise fall back to the real signal Core DOES emit — the
	// `active` bool — combined with the metadata.lifecycle_status hint the repo
	// writes on suspend/archive. Before this, the code read only `resp.Status`
	// (which Core never sends), so EVERY tenant defaulted to active and the admin
	// could never show a suspended/archived tenant.
	status := entities.TenantStatusActive
	switch resp.Status {
	case "suspended", "inactive":
		status = entities.TenantStatusSuspended
	case "deleted":
		status = entities.TenantStatusDeleted
	case "archived":
		status = entities.TenantStatusArchived
	case "active":
		status = entities.TenantStatusActive
	default:
		// No explicit status from Core — derive from the active flag.
		if !resp.Active {
			status = entities.TenantStatusSuspended
			if resp.Metadata != nil {
				if ls, ok := resp.Metadata["lifecycle_status"].(string); ok && ls == "archived" {
					status = entities.TenantStatusArchived
				}
			}
		}
	}

	// Pull HomeRegion out of metadata. Legacy tenants written before
	// the field existed have no key; EffectiveHomeRegion() falls back
	// to DefaultHomeRegion at read time. We accept any string here
	// rather than re-validating — Save validated on the way in, and
	// we don't want a stale allowlist entry to lock a region out
	// during read.
	homeRegion := ""
	if resp.Metadata != nil {
		if v, ok := resp.Metadata["home_region"].(string); ok {
			homeRegion = v
		}
	}

	// Use the real timestamps Core persists at creation. Older Core builds (and
	// any tenant written before Core returned these fields) send the zero value;
	// fall back to now() so the column is never blank, but never overwrite a real
	// timestamp — every tenant previously showed today's date because this used
	// time.Now() unconditionally.
	createdAt := resp.CreatedAt
	if createdAt.IsZero() {
		createdAt = time.Now()
	}
	updatedAt := resp.UpdatedAt
	if updatedAt.IsZero() {
		updatedAt = createdAt
	}

	return &entities.Tenant{
		ID:         resp.ID,
		Name:       resp.Name,
		Status:     status,
		HomeRegion: homeRegion,
		CreatedAt:  createdAt,
		UpdatedAt:  updatedAt,
		Metadata:   resp.Metadata,
	}
}
