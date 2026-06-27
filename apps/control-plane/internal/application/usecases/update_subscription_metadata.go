package usecases

import (
	"sync"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// keyedMutex serializes work per string key (here: per tenant ID). Lazily
// allocates one mutex per key; entries are never reclaimed, which is fine for a
// bounded tenant set.
type keyedMutex struct {
	m sync.Map
}

func (k *keyedMutex) lock(key string) func() {
	mu, _ := k.m.LoadOrStore(key, &sync.Mutex{})
	mtx := mu.(*sync.Mutex)
	mtx.Lock()
	return mtx.Unlock
}

// UpdateSubscriptionMetadataUseCase writes billing/subscription metadata
// to Core's tenant metadata. Called by the Control Plane when subscription
// events arrive from LemonSqueezy webhooks.
//
// Tenant metadata lives in Core as a single JSON blob updated via
// read-modify-write (FindByID -> mutate -> Update replaces the whole map).
// Concurrent subscription events for the same tenant (e.g. two webhooks, or a
// webhook racing a change-plan / reconciliation) would otherwise lost-update
// each other's entry. The per-tenant lock serializes the full read-modify-write
// so each caller observes the previous write. Single-instance correct (the
// Control Plane runs one Fly machine today); horizontal scale-out would need a
// Core-side compare-and-swap / ETag instead — see t-cad0f0.
type UpdateSubscriptionMetadataUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
	locks      *keyedMutex
}

// NewUpdateSubscriptionMetadataUseCase creates a new UpdateSubscriptionMetadataUseCase.
func NewUpdateSubscriptionMetadataUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *UpdateSubscriptionMetadataUseCase {
	return &UpdateSubscriptionMetadataUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		locks:      &keyedMutex{},
	}
}

// UpsertSubscription atomically upserts a single subscription into the tenant's
// tracked set and recomputes the effective tier as the highest-ranked ACTIVE
// subscription. The fresh read of the subscriptions map happens INSIDE the
// per-tenant lock, so concurrent upserts can't lose each other's entry (the
// bug a read-outside-lock + full-map-replace would cause). Returns the
// effective tier and the primary (highest-active) subscription ID.
//
// This is the single apply path for the bubble-up logic shared by the webhook,
// change-plan, and reconciliation use cases.
func (uc *UpdateSubscriptionMetadataUseCase) UpsertSubscription(
	tenantID, subID string, ref entities.SubscriptionRef,
) (effectiveTier, primarySubID string, err error) {
	unlock := uc.locks.lock(tenantID)
	defer unlock()

	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return "", "", err
	}
	subs := extractSubscriptionsMap(tenant.Metadata)
	if subs == nil {
		subs = map[string]entities.SubscriptionRef{}
	}
	subs[subID] = ref

	// Single source of truth for effective-tier / primary resolution (domain).
	primary := entities.PrimarySubscriptionFor(subs, providerLemonSqueezy)
	if _, err := uc.applyLocked(tenant, &entities.TenantBillingMetadata{
		Subscription:  primary,
		Subscriptions: subs,
	}); err != nil {
		return "", "", err
	}
	return primary.Tier, primary.SubscriptionID, nil
}

// Execute updates the subscription, quotas, and overage metadata for a tenant.
// It merges billing fields into the existing metadata without overwriting
// non-billing fields.
func (uc *UpdateSubscriptionMetadataUseCase) Execute(tenantID string, billing *entities.TenantBillingMetadata) (*dto.TenantResponse, error) {
	// Serialize the full read-modify-write per tenant (see type doc).
	unlock := uc.locks.lock(tenantID)
	defer unlock()

	// Find existing tenant
	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}
	return uc.applyLocked(tenant, billing)
}

// applyLocked merges billing metadata into an already-fetched tenant and
// persists it. Caller MUST already hold the per-tenant lock and have fetched
// `tenant` inside that lock.
func (uc *UpdateSubscriptionMetadataUseCase) applyLocked(tenant *entities.Tenant, billing *entities.TenantBillingMetadata) (*dto.TenantResponse, error) {
	tenantID := tenant.ID

	// Initialize metadata map if nil
	if tenant.Metadata == nil {
		tenant.Metadata = make(map[string]interface{})
	}

	// Merge billing metadata into tenant metadata, preserving non-billing keys
	billingMap := billing.ToMetadataMap()
	for k, v := range billingMap {
		tenant.Metadata[k] = v
	}

	// Apply tier-based entitlements if subscription tier is set and quotas are
	// not explicitly provided. This resolves retired tiers to their successor
	// (QuotasForTier handles that) and persists the FULL 011 entitlement set —
	// events/queries quota, x402 allowance, retention, streams, MCP scope — so
	// downstream enforcement (quota gate, x402 allowance checker) has everything
	// it needs from a single tenant read.
	//
	// CRITICAL: preserve the existing USAGE counters (events_used / queries_used /
	// x402_used / reset_date). These are the metered "used this period" numbers the
	// dashboard reads; rebuilding the quotas map from tier limits alone silently
	// zeroed them on every webhook / change-plan / scheduler tick, which is why a
	// backfilled events_used would not survive. Quota LIMITS come from the tier;
	// USAGE carries forward from whatever was already stored.
	if billing.Subscription != nil && billing.Subscription.Tier != "" && billing.Quotas == nil {
		tierQuotas := entities.QuotasForTier(billing.Subscription.Tier)
		prev := extractQuotas(tenant.Metadata)
		tenant.Metadata["quotas"] = &entities.QuotaMetadata{
			EventsQuota:           tierQuotas.EventsQuota,
			QueriesQuota:          tierQuotas.QueriesQuota,
			X402Allowance:         tierQuotas.X402Allowance,
			ExtractionTokensQuota: tierQuotas.ExtractionTokensQuota,
			RetentionDays:         tierQuotas.RetentionDays,
			MaxStreams:            tierQuotas.MaxStreams,
			MCPScope:              tierQuotas.MCPScope,
			// Carry usage forward — do not reset on a tier apply.
			EventsUsed:           prev.EventsUsed,
			QueriesUsed:          prev.QueriesUsed,
			X402Used:             prev.X402Used,
			ExtractionTokensUsed: prev.ExtractionTokensUsed,
			ResetDate:            prev.ResetDate,
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
		HomeRegion:  tenant.EffectiveHomeRegion(),
		CreatedAt:   tenant.CreatedAt,
		UpdatedAt:   tenant.UpdatedAt,
		Metadata:    tenant.Metadata,
	}, nil
}
