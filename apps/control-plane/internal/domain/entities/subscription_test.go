package entities

import (
	"testing"
)

func TestQuotasForTier(t *testing.T) {
	tests := []struct {
		tier        string
		wantEvents  int64
		wantQueries int64
	}{
		{"free", 100_000, 10_000},
		// Canonical 011 tiers (PRICING_EXPOSURE_PLAN.md §2).
		{"indie", 500_000, 50_000},
		{"studio", 5_000_000, 500_000},
		{"scale", 50_000_000, 5_000_000},
		{"enterprise", -1, -1},
		// Retired tiers resolve to their 011 successor via MapRetiredTier, but a
		// retired PAID tier keeps its pre-011 events/queries quota (no-downgrade
		// floor) so existing customers aren't silently halved.
		{"pro", 1_000_000, 100_000},       // RETIRED → indie successor, floored to pro's old 1M
		{"growth", 10_000_000, 1_000_000}, // RETIRED → studio successor, floored to growth's old 10M
		{"team", 10_000_000, 1_000_000},   // RETIRED → studio successor, floored (legacy alias)
		{"starter", 500_000, 50_000},      // RETIRED → indie (no floor; meets old entry quota)
		{"unknown", 100_000, 10_000},      // defaults to free
		{"", 100_000, 10_000},             // defaults to free
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

func TestQuotasForTier_ExtractionTokens(t *testing.T) {
	// Each plan includes an extraction-token allowance (hosted Hound doc
	// extraction). Scales with the tier; free = none, enterprise = unlimited.
	tests := []struct {
		tier string
		want int64
	}{
		{"free", 0},
		{"indie", 1_000_000},
		{"studio", 10_000_000},
		{"scale", 100_000_000},
		{"enterprise", -1},
	}
	for _, tt := range tests {
		t.Run(tt.tier, func(t *testing.T) {
			if got := QuotasForTier(tt.tier).ExtractionTokensQuota; got != tt.want {
				t.Errorf("ExtractionTokensQuota = %d, want %d", got, tt.want)
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
		{"free", TierQuotaMap[TierFree], false},
		{"indie", TierQuotaMap[TierIndie], false},
		{"studio", TierQuotaMap[TierStudio], false},
		{"scale", TierQuotaMap[TierScale], false},
		{"enterprise", TierQuotaMap[TierEnterprise], true},
		{"custom_unlimited", TierQuotas{EventsQuota: -1, QueriesQuota: -1}, true},
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

func TestHighestActiveTier(t *testing.T) {
	ref := func(tier, status string) SubscriptionRef {
		return SubscriptionRef{Tier: tier, Status: status}
	}

	t.Run("duplicates bubble up to highest active", func(t *testing.T) {
		subs := map[string]SubscriptionRef{
			"a": ref("studio", "active"),
			"b": ref("studio", "active"),
			"c": ref("indie", "active"),
		}
		tier, id := HighestActiveTier(subs)
		if tier != "studio" {
			t.Errorf("tier = %q, want studio", tier)
		}
		if id != "a" && id != "b" {
			t.Errorf("winning id = %q, want a or b (a studio)", id)
		}
	})

	t.Run("cancel top falls back to next active", func(t *testing.T) {
		subs := map[string]SubscriptionRef{
			"a": ref("studio", "cancelled"), //nolint:misspell // mirrors LemonSqueezy status spelling
			"c": ref("indie", "active"),
		}
		if tier, _ := HighestActiveTier(subs); tier != "indie" {
			t.Errorf("tier = %q, want indie", tier)
		}
	})

	t.Run("all inactive -> free", func(t *testing.T) {
		subs := map[string]SubscriptionRef{
			"a": ref("studio", "expired"),
			"c": ref("indie", "cancelled"), //nolint:misspell // mirrors LemonSqueezy status spelling
		}
		if tier, id := HighestActiveTier(subs); tier != "free" || id != "" {
			t.Errorf("got (%q,%q), want (free,'')", tier, id)
		}
	})

	t.Run("past_due still active (grace)", func(t *testing.T) {
		subs := map[string]SubscriptionRef{"a": ref("studio", "past_due")}
		if tier, _ := HighestActiveTier(subs); tier != "studio" {
			t.Errorf("tier = %q, want studio (past_due is active)", tier)
		}
	})

	t.Run("rank order enterprise>scale>studio>indie", func(t *testing.T) {
		subs := map[string]SubscriptionRef{
			"i": ref("indie", "active"),
			"s": ref("scale", "active"),
			"e": ref("enterprise", "active"),
			"t": ref("studio", "active"),
		}
		if tier, _ := HighestActiveTier(subs); tier != "enterprise" {
			t.Errorf("tier = %q, want enterprise", tier)
		}
	})
}

func TestPrimarySubscriptionFor(t *testing.T) {
	t.Run("bubbles up to highest active and carries its fields", func(t *testing.T) {
		subs := map[string]SubscriptionRef{
			"sub-a": {Tier: "indie", Status: "active", CustomerID: "c1", BillingPeriod: "month"},
			"sub-b": {Tier: "studio", Status: "active", CustomerID: "c2", BillingPeriod: "year"},
		}
		p := PrimarySubscriptionFor(subs, "lemonsqueezy")
		if p.Tier != "studio" || p.SubscriptionID != "sub-b" {
			t.Fatalf("got tier=%q id=%q, want studio/sub-b", p.Tier, p.SubscriptionID)
		}
		if p.Status != "active" || p.CustomerID != "c2" || p.BillingPeriod != "year" {
			t.Errorf("primary fields = %+v", p)
		}
		if p.PaymentProvider != "lemonsqueezy" {
			t.Errorf("provider = %q, want default lemonsqueezy", p.PaymentProvider)
		}
	})

	t.Run("no active subscriptions yields canceled free primary", func(t *testing.T) {
		subs := map[string]SubscriptionRef{
			"sub-a": {Tier: "studio", Status: "canceled"},
		}
		p := PrimarySubscriptionFor(subs, "lemonsqueezy")
		if p.Tier != "free" || p.Status != "canceled" || p.SubscriptionID != "" {
			t.Fatalf("got %+v, want free/canceled/empty", p)
		}
	})

	t.Run("winning ref provider overrides default", func(t *testing.T) {
		subs := map[string]SubscriptionRef{
			"sub-a": {Tier: "scale", Status: "active", PaymentProvider: "comp"},
		}
		if p := PrimarySubscriptionFor(subs, "lemonsqueezy"); p.PaymentProvider != "comp" {
			t.Errorf("provider = %q, want comp", p.PaymentProvider)
		}
	})
}
