package usecases

import (
	"context"
	"testing"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// mockLSClient overrides only GetSubscription; everything else panics if hit.
type mockLSClient struct {
	clients.LemonSqueezyClient
	subs map[string]*clients.SubscriptionResponse
	err  error
}

func (m *mockLSClient) GetSubscription(_ context.Context, id string) (*clients.SubscriptionResponse, error) {
	if m.err != nil {
		return nil, m.err
	}
	return m.subs[id], nil
}

func seedTenant(t *testing.T, repo *persistence.MemoryTenantRepository, id string, subs map[string]entities.SubscriptionRef) {
	t.Helper()
	tenant := &entities.Tenant{
		ID:       id,
		Name:     id,
		Status:   entities.TenantStatusActive,
		Metadata: map[string]interface{}{"subscriptions": subs},
	}
	if err := repo.Save(tenant); err != nil {
		t.Fatalf("seed tenant: %v", err)
	}
}

func TestSyncSubscriptions_PlanDrift(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	updateSubUC := NewUpdateSubscriptionMetadataUseCase(repo, auditRepo)

	// Tenant tracked as indie; LS now reports the sub on the studio variant (9001).
	seedTenant(t, repo, "tenant-1", map[string]entities.SubscriptionRef{
		"sub-100": {Tier: "indie", Status: "active", BillingPeriod: "month", PaymentProvider: providerLemonSqueezy},
	})
	ls := &mockLSClient{subs: map[string]*clients.SubscriptionResponse{
		"sub-100": {ID: "sub-100", Status: "active", VariantID: 9001},
	}}
	variantTier := VariantTierMap{"9001": "studio"}

	uc := NewSyncSubscriptionsUseCase(repo, ls, updateSubUC, variantTier)
	res, err := uc.Execute(context.Background(), "tenant-1")
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if !res.Changed || res.Tier != "studio" {
		t.Fatalf("expected studio change, got %+v", res)
	}

	// Persisted subscriptions map must now carry studio.
	tenant, _ := repo.FindByID("tenant-1")
	got := extractSubscriptionsMap(tenant.Metadata)
	if got["sub-100"].Tier != "studio" {
		t.Errorf("expected stored tier studio, got %q", got["sub-100"].Tier)
	}
}

func TestSyncSubscriptions_NoDrift(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	updateSubUC := NewUpdateSubscriptionMetadataUseCase(repo, auditRepo)

	seedTenant(t, repo, "tenant-2", map[string]entities.SubscriptionRef{
		"sub-200": {Tier: "studio", Status: "active", PaymentProvider: providerLemonSqueezy},
	})
	ls := &mockLSClient{subs: map[string]*clients.SubscriptionResponse{
		"sub-200": {ID: "sub-200", Status: "active", VariantID: 9001},
	}}
	uc := NewSyncSubscriptionsUseCase(repo, ls, updateSubUC, VariantTierMap{"9001": "studio"})

	res, err := uc.Execute(context.Background(), "tenant-2")
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if res.Changed {
		t.Fatalf("expected no change, got %+v", res)
	}
}

func TestSyncSubscriptions_StatusCanceled(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	updateSubUC := NewUpdateSubscriptionMetadataUseCase(repo, auditRepo)

	// Only tracked sub goes inactive → effective tier drops; no active sub remains.
	seedTenant(t, repo, "tenant-3", map[string]entities.SubscriptionRef{
		"sub-300": {Tier: "studio", Status: "active", PaymentProvider: providerLemonSqueezy},
	})
	ls := &mockLSClient{subs: map[string]*clients.SubscriptionResponse{
		"sub-300": {ID: "sub-300", Status: "cancelled", VariantID: 9001},
	}}
	uc := NewSyncSubscriptionsUseCase(repo, ls, updateSubUC, VariantTierMap{"9001": "studio"})

	res, err := uc.Execute(context.Background(), "tenant-3")
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if !res.Changed {
		t.Fatalf("expected change on cancel, got %+v", res)
	}
	tenant, _ := repo.FindByID("tenant-3")
	if got := extractSubscriptionsMap(tenant.Metadata)["sub-300"].Status; got != "cancelled" {
		t.Errorf("expected stored status cancelled, got %q", got)
	}
}

func TestSyncSubscriptions_TransientReadKeepsState(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	updateSubUC := NewUpdateSubscriptionMetadataUseCase(repo, auditRepo)

	seedTenant(t, repo, "tenant-4", map[string]entities.SubscriptionRef{
		"sub-400": {Tier: "scale", Status: "active", PaymentProvider: providerLemonSqueezy},
	})
	// GetSubscription returns nil sub (e.g. transient miss) → leave state intact.
	ls := &mockLSClient{subs: map[string]*clients.SubscriptionResponse{}}
	uc := NewSyncSubscriptionsUseCase(repo, ls, updateSubUC, VariantTierMap{"9001": "studio"})

	res, err := uc.Execute(context.Background(), "tenant-4")
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if res.Changed {
		t.Fatalf("expected no change on transient miss, got %+v", res)
	}
}
