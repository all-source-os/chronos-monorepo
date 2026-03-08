package billing

import (
	"math"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
)

func TestCalculateRevenueMetrics_NoTenants(t *testing.T) {
	result := CalculateRevenueMetrics(nil, 1)

	if result.MRR != 0 {
		t.Errorf("expected MRR=0, got %v", result.MRR)
	}
	if result.ARR != 0 {
		t.Errorf("expected ARR=0, got %v", result.ARR)
	}
	if result.GrowthRate != 0 {
		t.Errorf("expected GrowthRate=0, got %v", result.GrowthRate)
	}
	if result.ChurnRate != 0 {
		t.Errorf("expected ChurnRate=0, got %v", result.ChurnRate)
	}
}

func TestCalculateRevenueMetrics_SingleProTenant(t *testing.T) {
	now := time.Now()
	tenants := []*entities.Tenant{
		{
			ID:        "t-1",
			Name:      "Pro Corp",
			Status:    entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -3, 0),
			UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{
					Tier:   "pro",
					Status: "active",
				},
			},
		},
	}

	result := CalculateRevenueMetrics(tenants, 1)

	// Pro tier is $29/mo
	if result.MRR != 29.0 {
		t.Errorf("expected MRR=29.0, got %v", result.MRR)
	}
	if result.ARR != 348.0 {
		t.Errorf("expected ARR=348.0, got %v", result.ARR)
	}
}

func TestCalculateRevenueMetrics_MultipleTiers(t *testing.T) {
	now := time.Now()
	tenants := []*entities.Tenant{
		{
			ID: "t-1", Name: "Pro 1", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -6, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "pro", Status: "active"},
			},
		},
		{
			ID: "t-2", Name: "Pro 2", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -6, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "pro", Status: "active"},
			},
		},
		{
			ID: "t-3", Name: "Team 1", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -6, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "team", Status: "active"},
			},
		},
		{
			ID: "t-4", Name: "Free 1", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -6, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "free", Status: "active"},
			},
		},
	}

	result := CalculateRevenueMetrics(tenants, 1)

	// 2x Pro ($29) + 1x Team ($99) + 1x Free ($0) = $157/mo
	expectedMRR := 157.0
	if result.MRR != expectedMRR {
		t.Errorf("expected MRR=%.2f, got %.2f", expectedMRR, result.MRR)
	}
	if result.ARR != expectedMRR*12 {
		t.Errorf("expected ARR=%.2f, got %.2f", expectedMRR*12, result.ARR)
	}

	// Verify tier stats
	if result.TierStats["pro"].Count != 2 {
		t.Errorf("expected pro count=2, got %d", result.TierStats["pro"].Count)
	}
	if result.TierStats["team"].Count != 1 {
		t.Errorf("expected team count=1, got %d", result.TierStats["team"].Count)
	}
	if result.TierStats["free"].Count != 1 {
		t.Errorf("expected free count=1, got %d", result.TierStats["free"].Count)
	}
	if result.TierStats["pro"].Revenue != 58.0 {
		t.Errorf("expected pro revenue=58.0, got %.2f", result.TierStats["pro"].Revenue)
	}
	if result.TierStats["team"].Revenue != 99.0 {
		t.Errorf("expected team revenue=99.0, got %.2f", result.TierStats["team"].Revenue)
	}
}

func TestCalculateRevenueMetrics_ChurnRate(t *testing.T) {
	now := time.Now()
	// One tenant churned 15 days ago
	churnedAt := now.AddDate(0, 0, -15)

	tenants := []*entities.Tenant{
		{
			ID: "t-1", Name: "Active", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -6, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "pro", Status: "active"},
			},
		},
		{
			ID: "t-2", Name: "Churned", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -6, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{
					Tier:               "pro",
					Status:             "canceled",
					SubscriptionEndsAt: &churnedAt,
				},
			},
		},
	}

	result := CalculateRevenueMetrics(tenants, 1)

	// Only 1 active sub (the canceled one no longer counts toward MRR)
	if result.MRR != 29.0 {
		t.Errorf("expected MRR=29.0 (only active sub), got %.2f", result.MRR)
	}

	// Churn rate: 1 churned / 2 at period start = 50%
	if result.ChurnRate != 50.0 {
		t.Errorf("expected ChurnRate=50.0, got %.2f", result.ChurnRate)
	}
}

func TestCalculateRevenueMetrics_GrowthRate(t *testing.T) {
	now := time.Now()

	tenants := []*entities.Tenant{
		// Existed before the period
		{
			ID: "t-1", Name: "Old", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -6, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "pro", Status: "active"},
			},
		},
		// Created during the period (new customer)
		{
			ID: "t-2", Name: "New", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, 0, -10), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "team", Status: "active"},
			},
		},
	}

	result := CalculateRevenueMetrics(tenants, 1)

	// 2 active now, 1 at period start -> growth = (2-1)/1 * 100 = 100%
	if result.GrowthRate != 100.0 {
		t.Errorf("expected GrowthRate=100.0, got %.2f", result.GrowthRate)
	}
}

func TestCalculateRevenueMetrics_PastDueCountsAsMRR(t *testing.T) {
	now := time.Now()
	tenants := []*entities.Tenant{
		{
			ID: "t-1", Name: "Past Due", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -3, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "pro", Status: "past_due"},
			},
		},
	}

	result := CalculateRevenueMetrics(tenants, 1)

	// Past due still counts as MRR (they haven't canceled)
	if result.MRR != 29.0 {
		t.Errorf("expected MRR=29.0 for past_due, got %.2f", result.MRR)
	}
}

func TestCalculateRevenueMetrics_EmptyTierDefaultsToFree(t *testing.T) {
	now := time.Now()
	tenants := []*entities.Tenant{
		{
			ID: "t-1", Name: "No Tier", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -3, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{},
		},
	}

	result := CalculateRevenueMetrics(tenants, 1)

	// Default to free tier = $0
	if result.MRR != 0 {
		t.Errorf("expected MRR=0 for default free tier, got %.2f", result.MRR)
	}
	if result.TierStats["free"].Count != 1 {
		t.Errorf("expected free count=1, got %d", result.TierStats["free"].Count)
	}
}

func TestCalculateRevenueMetrics_MonthlyBreakdown(t *testing.T) {
	now := time.Now()
	tenants := []*entities.Tenant{
		{
			ID: "t-1", Name: "Long Term", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(-1, 0, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "pro", Status: "active"},
			},
		},
	}

	result := CalculateRevenueMetrics(tenants, 3)

	if len(result.Breakdown) != 3 {
		t.Fatalf("expected 3 monthly periods, got %d", len(result.Breakdown))
	}

	// Each month should have revenue from the pro tenant
	for _, b := range result.Breakdown {
		if b.Revenue != 29.0 {
			t.Errorf("expected revenue=29.0 for period %s, got %.2f", b.Period, b.Revenue)
		}
		if b.Subscribers != 1 {
			t.Errorf("expected subscribers=1 for period %s, got %d", b.Period, b.Subscribers)
		}
	}
}

func TestRangeToMonths(t *testing.T) {
	tests := []struct {
		input    string
		expected int
	}{
		{"30d", 1},
		{"90d", 3},
		{"1y", 12},
		{"invalid", 1},
		{"", 1},
	}

	for _, tt := range tests {
		result := rangeToMonths(tt.input)
		if result != tt.expected {
			t.Errorf("rangeToMonths(%q) = %d, want %d", tt.input, result, tt.expected)
		}
	}
}

func TestCalculateRevenueMetrics_ARREquality(t *testing.T) {
	now := time.Now()
	tenants := []*entities.Tenant{
		{
			ID: "t-1", Name: "Test", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -3, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "team", Status: "active"},
			},
		},
	}

	result := CalculateRevenueMetrics(tenants, 1)

	// ARR should always be MRR * 12
	expectedARR := result.MRR * 12
	if math.Abs(result.ARR-expectedARR) > 0.01 {
		t.Errorf("ARR (%.2f) should equal MRR (%.2f) * 12 = %.2f", result.ARR, result.MRR, expectedARR)
	}
}

func TestClassifyRetryStatus(t *testing.T) {
	tests := []struct {
		status   string
		expected string
	}{
		{"past_due", "pending_retry"},
		{"unpaid", "exhausted"},
		{"expired", "manual_review"},
		{"active", "unknown"},
	}

	for _, tt := range tests {
		sub := entities.SubscriptionMetadata{Status: tt.status}
		result := classifyRetryStatus(sub)
		if result != tt.expected {
			t.Errorf("classifyRetryStatus(%q) = %q, want %q", tt.status, result, tt.expected)
		}
	}
}
