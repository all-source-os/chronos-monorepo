package usecases

import (
	"testing"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func TestUpdateSubscriptionMetadataUseCase_Execute(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	updateSubUC := NewUpdateSubscriptionMetadataUseCase(tenantRepo, auditRepo)

	// Create a tenant first
	_, err := createUC.Execute(dto.CreateTenantRequest{
		ID:   "tenant-sub-1",
		Name: "Sub Tenant",
	})
	if err != nil {
		t.Fatalf("setup: create tenant failed: %v", err)
	}

	t.Run("set subscription with explicit quotas", func(t *testing.T) {
		billing := &entities.TenantBillingMetadata{
			Subscription: &entities.SubscriptionMetadata{
				CustomerID:     "cust_123",
				SubscriptionID: "sub_456",
				Status:         "active",
				Tier:           "pro",
			},
			Quotas: &entities.QuotaMetadata{
				EventsQuota:  1_000_000,
				QueriesQuota: 100_000,
			},
			Overage: &entities.OverageMetadata{
				Enabled:   true,
				EventRate: 0.001,
				QueryRate: 0.0005,
			},
		}

		resp, err := updateSubUC.Execute("tenant-sub-1", billing)
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		if resp.Metadata["subscription"] == nil {
			t.Error("expected subscription in metadata")
		}
		if resp.Metadata["quotas"] == nil {
			t.Error("expected quotas in metadata")
		}
		if resp.Metadata["overage"] == nil {
			t.Error("expected overage in metadata")
		}
	})

	t.Run("set subscription with tier-based auto quotas", func(t *testing.T) {
		billing := &entities.TenantBillingMetadata{
			Subscription: &entities.SubscriptionMetadata{
				CustomerID:     "cust_789",
				SubscriptionID: "sub_012",
				Status:         "active",
				Tier:           "studio",
			},
			// No explicit Quotas — should auto-apply from the tier (011 §2: Studio = 5M/500K).
		}

		resp, err := updateSubUC.Execute("tenant-sub-1", billing)
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		quotas, ok := resp.Metadata["quotas"].(*entities.QuotaMetadata)
		if !ok {
			t.Fatalf("quotas should be *QuotaMetadata, got %T", resp.Metadata["quotas"])
		}
		if quotas.EventsQuota != 5_000_000 {
			t.Errorf("EventsQuota = %d, want 5000000", quotas.EventsQuota)
		}
		if quotas.QueriesQuota != 500_000 {
			t.Errorf("QueriesQuota = %d, want 500000", quotas.QueriesQuota)
		}
	})

	t.Run("preserves non-billing metadata", func(t *testing.T) {
		// First set some non-billing metadata
		tenant, err := tenantRepo.FindByID("tenant-sub-1")
		if err != nil {
			t.Fatalf("find tenant: %v", err)
		}
		tenant.Metadata["custom_key"] = "custom_value"
		if err := tenantRepo.Update(tenant); err != nil {
			t.Fatalf("update tenant: %v", err)
		}

		// Now update subscription
		billing := &entities.TenantBillingMetadata{
			Subscription: &entities.SubscriptionMetadata{
				Tier: "free",
			},
		}

		resp, err := updateSubUC.Execute("tenant-sub-1", billing)
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		if resp.Metadata["custom_key"] != "custom_value" {
			t.Errorf("custom_key should be preserved, got %v", resp.Metadata["custom_key"])
		}
		if resp.Metadata["subscription"] == nil {
			t.Error("subscription should be set")
		}
	})

	t.Run("tenant not found", func(t *testing.T) {
		billing := &entities.TenantBillingMetadata{
			Subscription: &entities.SubscriptionMetadata{Tier: "free"},
		}

		_, err := updateSubUC.Execute("nonexistent", billing)
		if err == nil {
			t.Error("expected error for nonexistent tenant")
		}
	})

	t.Run("preserves usage counters across a tier apply", func(t *testing.T) {
		// Regression: a tier apply used to rebuild the quotas map from tier limits
		// alone, silently zeroing events_used / queries_used / x402_used on every
		// webhook / change-plan / scheduler tick — which erased any backfilled
		// usage. The metered "used" numbers must carry forward.
		repo := persistence.NewMemoryTenantRepository()
		audit := persistence.NewMemoryAuditRepository()
		ctUC := NewCreateTenantUseCase(repo, audit)
		subUC := NewUpdateSubscriptionMetadataUseCase(repo, audit)

		if _, err := ctUC.Execute(dto.CreateTenantRequest{ID: "tenant-usage", Name: "Usage Tenant"}); err != nil {
			t.Fatalf("setup: create tenant: %v", err)
		}

		// Seed real usage on the tenant's quotas.
		seed, err := repo.FindByID("tenant-usage")
		if err != nil {
			t.Fatalf("find tenant: %v", err)
		}
		seed.Metadata["quotas"] = &entities.QuotaMetadata{
			EventsQuota:  5_000_000,
			QueriesQuota: 500_000,
			EventsUsed:   870_000,
			QueriesUsed:  1_234,
			X402Used:     7,
			ResetDate:    "2026-07-01",
		}
		if err := repo.Update(seed); err != nil {
			t.Fatalf("seed usage: %v", err)
		}

		// Apply a tier (no explicit quotas → tier auto-quotas path, the clobber site).
		resp, err := subUC.Execute("tenant-usage", &entities.TenantBillingMetadata{
			Subscription: &entities.SubscriptionMetadata{Tier: "studio", Status: "active"},
		})
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		quotas, ok := resp.Metadata["quotas"].(*entities.QuotaMetadata)
		if !ok {
			t.Fatalf("quotas should be *QuotaMetadata, got %T", resp.Metadata["quotas"])
		}
		// Limits refreshed from the tier...
		if quotas.EventsQuota != 5_000_000 {
			t.Errorf("EventsQuota = %d, want 5000000", quotas.EventsQuota)
		}
		// ...usage carried forward (the regression).
		if quotas.EventsUsed != 870_000 {
			t.Errorf("EventsUsed = %d, want 870000 (must survive tier apply)", quotas.EventsUsed)
		}
		if quotas.QueriesUsed != 1_234 {
			t.Errorf("QueriesUsed = %d, want 1234 (must survive tier apply)", quotas.QueriesUsed)
		}
		if quotas.X402Used != 7 {
			t.Errorf("X402Used = %d, want 7 (must survive tier apply)", quotas.X402Used)
		}
		if quotas.ResetDate != "2026-07-01" {
			t.Errorf("ResetDate = %q, want 2026-07-01 (must survive tier apply)", quotas.ResetDate)
		}
	})
}
