package usecases

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// catalogMockLS implements clients.LemonSqueezyClient for catalog tests.
// Only LookupVariantID + GetVariant carry behaviour; the rest are stubs.
type catalogMockLS struct {
	variants map[string]*clients.VariantResponse // variantID → variant
	getCalls int
}

func (m *catalogMockLS) LookupVariantID(tier, period string) (string, error) {
	if period == "" {
		period = "monthly"
	}
	if period == "yearly" {
		period = "annual"
	}
	id := tier + ":" + period
	if _, ok := m.variants[id]; ok {
		return id, nil
	}
	return "", fmt.Errorf("no variant for %s:%s", tier, period)
}

func (m *catalogMockLS) GetVariant(_ context.Context, variantID string) (*clients.VariantResponse, error) {
	m.getCalls++
	v, ok := m.variants[variantID]
	if !ok {
		return nil, fmt.Errorf("variant %s not found", variantID)
	}
	return v, nil
}

func (m *catalogMockLS) VariantMap() clients.VariantMap { return nil }
func (m *catalogMockLS) GetStoreID() string             { return "store" }
func (m *catalogMockLS) CreateCheckout(_ context.Context, _ clients.CreateCheckoutRequest) (*clients.CheckoutResponse, error) {
	return nil, fmt.Errorf("not implemented")
}
func (m *catalogMockLS) GetCustomerPortalURL(_ context.Context, _ string) (string, error) {
	return "", fmt.Errorf("not implemented")
}
func (m *catalogMockLS) ReportUsage(_ context.Context, _ clients.ReportUsageRequest) error {
	return fmt.Errorf("not implemented")
}
func (m *catalogMockLS) GetSubscription(_ context.Context, _ string) (*clients.SubscriptionResponse, error) {
	return nil, fmt.Errorf("not implemented")
}
func (m *catalogMockLS) ListSubscriptions(_ context.Context, _ string, _ int) (*clients.SubscriptionListResponse, error) {
	return nil, fmt.Errorf("not implemented")
}
func (m *catalogMockLS) ListInvoices(_ context.Context, _ string, _ int, _ string) (*clients.InvoiceListResponse, error) {
	return nil, fmt.Errorf("not implemented")
}
func (m *catalogMockLS) RefundInvoice(_ context.Context, _ string, _ int) error {
	return fmt.Errorf("not implemented")
}

func TestFormatCents(t *testing.T) {
	cases := map[int]string{
		1900:  "$19",
		1899:  "$18.99",
		18199: "$181.99",
		29899: "$298.99",
		0:     "$0",
	}
	for cents, want := range cases {
		if got := formatCents(cents); got != want {
			t.Errorf("formatCents(%d) = %q, want %q", cents, got, want)
		}
	}
}

func TestGetCatalog_ReadsLemonSqueezyPrices(t *testing.T) {
	ls := &catalogMockLS{variants: map[string]*clients.VariantResponse{
		"indie:monthly": {Price: 1899, Interval: "month"},
		"indie:annual":  {Price: 18199, Interval: "year"},
		"studio:monthly": {Price: 7899, Interval: "month"},
		// studio:annual intentionally missing → tier still returned with only monthly
	}}
	uc := NewGetCatalogUseCase(ls)

	cat, err := uc.Execute(context.Background(), time.Unix(1_700_000_000, 0))
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}

	byTier := map[string]CatalogTier{}
	for _, t := range cat.Tiers {
		byTier[t.Tier] = t
	}

	indie, ok := byTier["indie"]
	if !ok {
		t.Fatal("indie missing from catalog")
	}
	if indie.Monthly == nil || indie.Monthly.Formatted != "$18.99" {
		t.Errorf("indie monthly = %+v, want $18.99", indie.Monthly)
	}
	if indie.Annual == nil || indie.Annual.Formatted != "$181.99" {
		t.Errorf("indie annual = %+v, want $181.99", indie.Annual)
	}
	// $181.99/12 = $15.166 → rounds to $15.17 (not truncated $15.16).
	if indie.Annual.PerMonth != "$15.17" {
		t.Errorf("indie annual per-month = %q, want $15.17", indie.Annual.PerMonth)
	}

	studio, ok := byTier["studio"]
	if !ok || studio.Monthly == nil || studio.Monthly.Formatted != "$78.99" {
		t.Errorf("studio monthly = %+v, want $78.99", studio.Monthly)
	}
	if studio.Annual != nil {
		t.Errorf("studio annual should be nil (variant missing), got %+v", studio.Annual)
	}
}

func TestGetCatalog_NilClient_EmptyCatalog(t *testing.T) {
	uc := NewGetCatalogUseCase(nil)
	cat, err := uc.Execute(context.Background(), time.Unix(1_700_000_000, 0))
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if len(cat.Tiers) != 0 {
		t.Errorf("nil LS should yield empty catalog, got %d tiers", len(cat.Tiers))
	}
}

func TestGetCatalog_CachesWithinTTL(t *testing.T) {
	ls := &catalogMockLS{variants: map[string]*clients.VariantResponse{
		"indie:monthly": {Price: 1899},
	}}
	uc := NewGetCatalogUseCase(ls)
	base := time.Unix(1_700_000_000, 0)

	_, _ = uc.Execute(context.Background(), base)
	callsAfterFirst := ls.getCalls
	if callsAfterFirst == 0 {
		t.Fatal("expected LS GetVariant calls on first Execute")
	}
	// Within TTL → served from cache, no new LS calls.
	_, _ = uc.Execute(context.Background(), base.Add(30*time.Minute))
	if ls.getCalls != callsAfterFirst {
		t.Errorf("expected cache hit (no new LS calls); got %d -> %d", callsAfterFirst, ls.getCalls)
	}
	// Past TTL → refetch.
	_, _ = uc.Execute(context.Background(), base.Add(2*time.Hour))
	if ls.getCalls == callsAfterFirst {
		t.Error("expected LS refetch after TTL expiry")
	}
}
