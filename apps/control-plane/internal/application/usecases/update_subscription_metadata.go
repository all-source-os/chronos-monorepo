package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// UpdateSubscriptionMetadataUseCase writes billing/subscription metadata
// to Core's tenant metadata. Called by the Control Plane when subscription
// events arrive from LemonSqueezy webhooks.
type UpdateSubscriptionMetadataUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewUpdateSubscriptionMetadataUseCase creates a new UpdateSubscriptionMetadataUseCase.
func NewUpdateSubscriptionMetadataUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *UpdateSubscriptionMetadataUseCase {
	return &UpdateSubscriptionMetadataUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute updates the subscription, quotas, and overage metadata for a tenant.
// It merges billing fields into the existing metadata without overwriting
// non-billing fields.
func (uc *UpdateSubscriptionMetadataUseCase) Execute(tenantID string, billing *entities.TenantBillingMetadata) (*dto.TenantResponse, error) {
	// Find existing tenant
	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}

	// Initialize metadata map if nil
	if tenant.Metadata == nil {
		tenant.Metadata = make(map[string]interface{})
	}

	// Merge billing metadata into tenant metadata, preserving non-billing keys
	billingMap := billing.ToMetadataMap()
	for k, v := range billingMap {
		tenant.Metadata[k] = v
	}

	// Apply tier-based quotas if subscription tier is set and quotas are not explicitly provided
	if billing.Subscription != nil && billing.Subscription.Tier != "" && billing.Quotas == nil {
		tierQuotas := entities.QuotasForTier(billing.Subscription.Tier)
		tenant.Metadata["quotas"] = &entities.QuotaMetadata{
			EventsQuota:  tierQuotas.EventsQuota,
			QueriesQuota: tierQuotas.QueriesQuota,
		}
	}

	// Persist
	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, err
	}

	// Log audit event
	auditEvent, _ := entities.NewAuditEvent("tenant.subscription.updated", "update", "PUT", "/tenants/"+tenantID+"/subscription") //nolint:errcheck
	auditEvent.WithResource("tenant", tenant.ID).WithTenant(tenant.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &dto.TenantResponse{
		ID:          tenant.ID,
		Name:        tenant.Name,
		Description: tenant.Description,
		Status:      string(tenant.Status),
		CreatedAt:   tenant.CreatedAt,
		UpdatedAt:   tenant.UpdatedAt,
		Metadata:    tenant.Metadata,
	}, nil
}
