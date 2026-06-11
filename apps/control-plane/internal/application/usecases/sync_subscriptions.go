package usecases

import (
	"context"
	"log"
	"strconv"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// SubSyncResult records the outcome of reconciling one tenant's subscriptions.
type SubSyncResult struct {
	TenantID string
	Tier     string
	Changed  bool
	Error    error
}

// SyncSubscriptionsUseCase reconciles each tenant's tracked subscriptions
// against LemonSqueezy. Webhooks are the primary sync path, but a missed or
// rejected webhook (e.g. a transient outage or a signature blip) would leave the
// stored tier stale. This use case re-reads each tracked subscription's current
// status + variant from LS and recomputes the effective tier (highest active),
// so subscription state self-heals without manual webhook replays.
//
// It only touches subscriptions ALREADY tracked on the tenant (the
// subscription_created webhook still seeds new ones); it catches drift in
// status (active→cancelled/expired) and plan (variant changed) on known subs.
type SyncSubscriptionsUseCase struct {
	tenantRepo  repositories.TenantRepository
	lsClient    clients.LemonSqueezyClient
	updateSubUC *UpdateSubscriptionMetadataUseCase
	variantTier VariantTierMap // variant id → tier (reverse of LEMON_SQUEEZY_VARIANT_MAP)
}

// NewSyncSubscriptionsUseCase constructs the reconciliation use case.
func NewSyncSubscriptionsUseCase(
	tenantRepo repositories.TenantRepository,
	lsClient clients.LemonSqueezyClient,
	updateSubUC *UpdateSubscriptionMetadataUseCase,
	variantTier VariantTierMap,
) *SyncSubscriptionsUseCase {
	return &SyncSubscriptionsUseCase{
		tenantRepo:  tenantRepo,
		lsClient:    lsClient,
		updateSubUC: updateSubUC,
		variantTier: variantTier,
	}
}

// Execute reconciles one tenant. Returns Changed=false when nothing drifted.
func (uc *SyncSubscriptionsUseCase) Execute(ctx context.Context, tenantID string) (*SubSyncResult, error) {
	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}
	subs := extractSubscriptionsMap(tenant.Metadata)
	if len(subs) == 0 {
		return &SubSyncResult{TenantID: tenantID}, nil
	}

	changed := false
	effective := ""
	for id, ref := range subs {
		lsSub, err := uc.lsClient.GetSubscription(ctx, id)
		if err != nil || lsSub == nil {
			// Leave the tracked record as-is on a transient read failure.
			continue
		}
		newStatus := lsSub.Status
		newTier := ref.Tier
		if uc.variantTier != nil {
			if t, ok := uc.variantTier[strconv.Itoa(lsSub.VariantID)]; ok && t != "" {
				newTier = t
			}
		}
		if newStatus == ref.Status && newTier == ref.Tier {
			continue
		}
		ref.Status = newStatus
		ref.Tier = newTier
		// Atomic per-tenant upsert + bubble-up; re-reads the map under the lock
		// so a concurrent webhook can't lose this correction (race-safe).
		eff, _, err := uc.updateSubUC.UpsertSubscription(tenantID, id, ref)
		if err != nil {
			return &SubSyncResult{TenantID: tenantID, Changed: true, Error: err}, err
		}
		effective = eff
		changed = true
	}
	if !changed {
		return &SubSyncResult{TenantID: tenantID}, nil
	}
	return &SubSyncResult{TenantID: tenantID, Tier: effective, Changed: true}, nil
}

// ExecuteAll reconciles every active tenant. nil LS client → no-op.
func (uc *SyncSubscriptionsUseCase) ExecuteAll(ctx context.Context) []SubSyncResult {
	if uc.lsClient == nil {
		return nil
	}
	tenants, err := uc.tenantRepo.FindActive()
	if err != nil {
		log.Printf("SyncSubscriptions: list tenants failed: %v", err)
		return nil
	}
	results := make([]SubSyncResult, 0)
	for _, t := range tenants {
		r, err := uc.Execute(ctx, t.ID)
		if err != nil {
			results = append(results, SubSyncResult{TenantID: t.ID, Error: err})
			continue
		}
		if r.Changed {
			results = append(results, *r)
		}
	}
	return results
}
