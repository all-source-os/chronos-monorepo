package billing

import (
	"testing"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func TestCalculateOverage_Disabled(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	tenant := &entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage": &entities.OverageMetadata{Enabled: false},
			"quotas":  &entities.QuotaMetadata{EventsQuota: 1000, EventsUsed: 2000},
		},
	}
	_ = repo.Save(tenant)

	uc := NewCalculateOverageUseCase(repo)
	result, err := uc.Execute("t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.Enabled {
		t.Error("expected overage to be disabled")
	}
	if result.EventsOverage != 0 {
		t.Errorf("expected 0 events overage when disabled, got %d", result.EventsOverage)
	}
}

func TestCalculateOverage_EventsOnly(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	tenant := &entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true, EventRate: 0.001},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 15000, QueriesUsed: 3000},
			"subscription": &entities.SubscriptionMetadata{Tier: "starter"},
		},
	}
	_ = repo.Save(tenant)

	uc := NewCalculateOverageUseCase(repo)
	result, err := uc.Execute("t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.Enabled {
		t.Error("expected overage to be enabled")
	}
	if result.EventsOverage != 5000 {
		t.Errorf("expected 5000 events overage, got %d", result.EventsOverage)
	}
	if result.QueriesOverage != 0 {
		t.Errorf("expected 0 queries overage, got %d", result.QueriesOverage)
	}
}

func TestCalculateOverage_QueriesOnly(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	tenant := &entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true, QueryRate: 0.002},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 8000, QueriesUsed: 7000},
			"subscription": &entities.SubscriptionMetadata{Tier: "starter"},
		},
	}
	_ = repo.Save(tenant)

	uc := NewCalculateOverageUseCase(repo)
	result, err := uc.Execute("t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.EventsOverage != 0 {
		t.Errorf("expected 0 events overage, got %d", result.EventsOverage)
	}
	if result.QueriesOverage != 2000 {
		t.Errorf("expected 2000 queries overage, got %d", result.QueriesOverage)
	}
}

func TestCalculateOverage_Both(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	tenant := &entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true, EventRate: 0.001, QueryRate: 0.002},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 12000, QueriesUsed: 6000},
			"subscription": &entities.SubscriptionMetadata{Tier: "starter"},
		},
	}
	_ = repo.Save(tenant)

	uc := NewCalculateOverageUseCase(repo)
	result, err := uc.Execute("t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.EventsOverage != 2000 {
		t.Errorf("expected 2000 events overage, got %d", result.EventsOverage)
	}
	if result.QueriesOverage != 1000 {
		t.Errorf("expected 1000 queries overage, got %d", result.QueriesOverage)
	}
}

func TestCalculateOverage_NoOverage(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	tenant := &entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 5000, QueriesUsed: 2000},
			"subscription": &entities.SubscriptionMetadata{Tier: "starter"},
		},
	}
	_ = repo.Save(tenant)

	uc := NewCalculateOverageUseCase(repo)
	result, err := uc.Execute("t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.EventsOverage != 0 {
		t.Errorf("expected 0 events overage, got %d", result.EventsOverage)
	}
	if result.QueriesOverage != 0 {
		t.Errorf("expected 0 queries overage, got %d", result.QueriesOverage)
	}
}

func TestCalculateOverage_UnlimitedTier(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	tenant := &entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true},
			"quotas":       &entities.QuotaMetadata{EventsQuota: -1, QueriesQuota: -1, EventsUsed: 999999},
			"subscription": &entities.SubscriptionMetadata{Tier: "enterprise"},
		},
	}
	_ = repo.Save(tenant)

	uc := NewCalculateOverageUseCase(repo)
	result, err := uc.Execute("t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.EventsOverage != 0 {
		t.Errorf("expected 0 events overage for unlimited tier, got %d", result.EventsOverage)
	}
}

func TestCalculateOverage_ExecuteAll(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()

	// Tenant with overage
	_ = repo.Save(&entities.Tenant{
		ID:     "t1",
		Name:   "tenant-with-overage",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 1000, QueriesQuota: 500, EventsUsed: 2000, QueriesUsed: 100},
			"subscription": &entities.SubscriptionMetadata{Tier: "starter"},
		},
	})

	// Tenant without overage
	_ = repo.Save(&entities.Tenant{
		ID:     "t2",
		Name:   "tenant-no-overage",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 100, QueriesUsed: 50},
			"subscription": &entities.SubscriptionMetadata{Tier: "starter"},
		},
	})

	// Tenant with overage disabled
	_ = repo.Save(&entities.Tenant{
		ID:     "t3",
		Name:   "tenant-disabled",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage": &entities.OverageMetadata{Enabled: false},
			"quotas":  &entities.QuotaMetadata{EventsQuota: 1000, EventsUsed: 5000},
		},
	})

	uc := NewCalculateOverageUseCase(repo)
	results, err := uc.ExecuteAll()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	// Only t1 should appear (has overage and enabled)
	if len(results) != 1 {
		t.Fatalf("expected 1 result with overage, got %d", len(results))
	}
	if results[0].TenantID != "t1" {
		t.Errorf("expected tenant t1, got %s", results[0].TenantID)
	}
	if results[0].EventsOverage != 1000 {
		t.Errorf("expected 1000 events overage, got %d", results[0].EventsOverage)
	}
}
