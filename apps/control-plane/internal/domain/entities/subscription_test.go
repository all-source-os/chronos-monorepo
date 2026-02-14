package entities

import (
	"testing"
)

func TestQuotasForTier(t *testing.T) {
	tests := []struct {
		tier         string
		wantEvents   int64
		wantQueries  int64
	}{
		{"free", 10_000, 5_000},
		{"starter", 100_000, 50_000},
		{"pro", 1_000_000, 100_000},
		{"enterprise", -1, -1},
		{"unknown", 10_000, 5_000},   // defaults to free
		{"", 10_000, 5_000},           // defaults to free
	}

	for _, tt := range tests {
		t.Run(tt.tier, func(t *testing.T) {
			q := QuotasForTier(tt.tier)
			if q.EventsQuota != tt.wantEvents {
				t.Errorf("EventsQuota = %d, want %d", q.EventsQuota, tt.wantEvents)
			}
			if q.QueriesQuota != tt.wantQueries {
				t.Errorf("QueriesQuota = %d, want %d", q.QueriesQuota, tt.wantQueries)
			}
		})
	}
}

func TestTierQuotas_IsUnlimited(t *testing.T) {
	tests := []struct {
		name string
		q    TierQuotas
		want bool
	}{
		{"enterprise", TierQuotaMap[TierEnterprise], true},
		{"free", TierQuotaMap[TierFree], false},
		{"pro", TierQuotaMap[TierPro], false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := tt.q.IsUnlimited(); got != tt.want {
				t.Errorf("IsUnlimited() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestTenantBillingMetadata_ToMetadataMap(t *testing.T) {
	t.Run("all fields set", func(t *testing.T) {
		b := &TenantBillingMetadata{
			Subscription: &SubscriptionMetadata{
				CustomerID:     "cust_123",
				SubscriptionID: "sub_456",
				Status:         "active",
				Tier:           "pro",
			},
			Quotas: &QuotaMetadata{
				EventsQuota:  1_000_000,
				QueriesQuota: 100_000,
			},
			Overage: &OverageMetadata{
				Enabled:   true,
				EventRate: 0.001,
			},
		}

		m := b.ToMetadataMap()
		if m["subscription"] == nil {
			t.Error("expected subscription key")
		}
		if m["quotas"] == nil {
			t.Error("expected quotas key")
		}
		if m["overage"] == nil {
			t.Error("expected overage key")
		}
	})

	t.Run("nil fields omitted", func(t *testing.T) {
		b := &TenantBillingMetadata{
			Subscription: &SubscriptionMetadata{Tier: "free"},
		}

		m := b.ToMetadataMap()
		if m["subscription"] == nil {
			t.Error("expected subscription key")
		}
		if _, ok := m["quotas"]; ok {
			t.Error("quotas should not be present when nil")
		}
		if _, ok := m["overage"]; ok {
			t.Error("overage should not be present when nil")
		}
	})
}
