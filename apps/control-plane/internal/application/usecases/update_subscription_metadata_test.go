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
				Tier:           "starter",
			},
			// No explicit Quotas — should auto-apply from tier
		}

		resp, err := updateSubUC.Execute("tenant-sub-1", billing)
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		quotas, ok := resp.Metadata["quotas"].(*entities.QuotaMetadata)
		if !ok {
			t.Fatalf("quotas should be *QuotaMetadata, got %T", resp.Metadata["quotas"])
		}
		if quotas.EventsQuota != 100_000 {
			t.Errorf("EventsQuota = %d, want 100000", quotas.EventsQuota)
		}
		if quotas.QueriesQuota != 50_000 {
			t.Errorf("QueriesQuota = %d, want 50000", quotas.QueriesQuota)
		}
	})

	t.Run("preserves non-billing metadata", func(t *testing.T) {
		// First set some non-billing metadata
		tenant, _ := tenantRepo.FindByID("tenant-sub-1")
		tenant.Metadata["custom_key"] = "custom_value"
		_ = tenantRepo.Update(tenant)

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
}
