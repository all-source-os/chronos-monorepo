package billing

import (
	"context"
	"fmt"
	"testing"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// mockLSClient implements clients.LemonSqueezyClient for testing.
type mockLSClient struct {
	reportedUsages []clients.ReportUsageRequest
	reportErr      error
}

func (m *mockLSClient) CreateCheckout(_ context.Context, _ clients.CreateCheckoutRequest) (*clients.CheckoutResponse, error) {
	return nil, fmt.Errorf("not implemented")
}

func (m *mockLSClient) GetCustomerPortalURL(_ context.Context, _ string) (string, error) {
	return "", fmt.Errorf("not implemented")
}

func (m *mockLSClient) ReportUsage(_ context.Context, req clients.ReportUsageRequest) error {
	if m.reportErr != nil {
		return m.reportErr
	}
	m.reportedUsages = append(m.reportedUsages, req)
	return nil
}

func (m *mockLSClient) GetSubscription(_ context.Context, _ string) (*clients.SubscriptionResponse, error) {
	return nil, fmt.Errorf("not implemented")
}

func (m *mockLSClient) LookupVariantID(_, _ string) (string, error) {
	return "", fmt.Errorf("not implemented")
}

func (m *mockLSClient) GetVariant(_ context.Context, _ string) (*clients.VariantResponse, error) {
	return nil, fmt.Errorf("not implemented")
}

func (m *mockLSClient) UpdateSubscription(_ context.Context, _ string, _ int) (*clients.SubscriptionResponse, error) {
	return nil, fmt.Errorf("not implemented")
}

func (m *mockLSClient) VariantMap() clients.VariantMap {
	return nil
}

func (m *mockLSClient) ListSubscriptions(_ context.Context, _ string, _ int) (*clients.SubscriptionListResponse, error) {
	return nil, fmt.Errorf("not implemented")
}

func (m *mockLSClient) ListInvoices(_ context.Context, _ string, _ int, _ string) (*clients.InvoiceListResponse, error) {
	return nil, fmt.Errorf("not implemented")
}

func (m *mockLSClient) RefundInvoice(_ context.Context, _ string, _ int) error {
	return fmt.Errorf("not implemented")
}

func (m *mockLSClient) GetStoreID() string {
	return "test-store"
}

func TestReportUsage_NoOverage(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	lsClient := &mockLSClient{}

	if err := repo.Save(&entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 5000, QueriesUsed: 2000},
			"subscription": &entities.SubscriptionMetadata{Tier: "pro", SubscriptionItemID: "si_1"},
		},
	}); err != nil {
		t.Fatalf("save tenant: %v", err)
	}

	calcUC := NewCalculateOverageUseCase(repo)
	uc := NewReportUsageUseCase(repo, auditRepo, lsClient, calcUC)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.Skipped {
		t.Error("expected skipped when no overage")
	}
	if len(lsClient.reportedUsages) != 0 {
		t.Error("expected no usage reported")
	}
}

func TestReportUsage_EventsOverage(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	lsClient := &mockLSClient{}

	if err := repo.Save(&entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true, EventRate: 0.001},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 15000, QueriesUsed: 3000},
			"subscription": &entities.SubscriptionMetadata{Tier: "pro", SubscriptionItemID: "si_1"},
		},
	}); err != nil {
		t.Fatalf("save tenant: %v", err)
	}

	calcUC := NewCalculateOverageUseCase(repo)
	uc := NewReportUsageUseCase(repo, auditRepo, lsClient, calcUC)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.Skipped {
		t.Error("expected not skipped")
	}
	if result.EventsReported != 5000 {
		t.Errorf("expected 5000 events reported, got %d", result.EventsReported)
	}
	if len(lsClient.reportedUsages) != 1 {
		t.Fatalf("expected 1 usage report, got %d", len(lsClient.reportedUsages))
	}
	if lsClient.reportedUsages[0].Quantity != 5000 {
		t.Errorf("expected quantity 5000, got %d", lsClient.reportedUsages[0].Quantity)
	}

	// Verify last_reported was updated in metadata
	tenant, err := repo.FindByID("t1")
	if err != nil {
		t.Fatalf("find tenant: %v", err)
	}
	overage := extractOverage(tenant.Metadata)
	if overage.LastReportedEvents != 5000 {
		t.Errorf("expected last_reported_events=5000, got %d", overage.LastReportedEvents)
	}
}

func TestReportUsage_DoubleReportPrevention(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	lsClient := &mockLSClient{}

	if err := repo.Save(&entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage": &entities.OverageMetadata{
				Enabled:             true,
				EventRate:           0.001,
				LastReportedEvents:  5000, // already reported 5000
				LastReportedQueries: 0,
			},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 15000, QueriesUsed: 3000},
			"subscription": &entities.SubscriptionMetadata{Tier: "pro", SubscriptionItemID: "si_1"},
		},
	}); err != nil {
		t.Fatalf("save tenant: %v", err)
	}

	calcUC := NewCalculateOverageUseCase(repo)
	uc := NewReportUsageUseCase(repo, auditRepo, lsClient, calcUC)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.Skipped {
		t.Error("expected skipped — overage already reported")
	}
	if len(lsClient.reportedUsages) != 0 {
		t.Error("expected no new usage reported (already reported)")
	}
}

func TestReportUsage_IncrementalReport(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	lsClient := &mockLSClient{}

	// Previously reported 3000 events overage, now at 5000
	if err := repo.Save(&entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage": &entities.OverageMetadata{
				Enabled:            true,
				EventRate:          0.001,
				LastReportedEvents: 3000,
			},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 15000, QueriesUsed: 3000},
			"subscription": &entities.SubscriptionMetadata{Tier: "pro", SubscriptionItemID: "si_1"},
		},
	}); err != nil {
		t.Fatalf("save tenant: %v", err)
	}

	calcUC := NewCalculateOverageUseCase(repo)
	uc := NewReportUsageUseCase(repo, auditRepo, lsClient, calcUC)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.Skipped {
		t.Error("expected not skipped")
	}
	// Delta = 5000 (current) - 3000 (last reported) = 2000
	if result.EventsReported != 2000 {
		t.Errorf("expected 2000 events delta, got %d", result.EventsReported)
	}
	if lsClient.reportedUsages[0].Quantity != 2000 {
		t.Errorf("expected quantity 2000, got %d", lsClient.reportedUsages[0].Quantity)
	}
}

func TestReportUsage_NoSubscriptionItemID(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	lsClient := &mockLSClient{}

	if err := repo.Save(&entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 15000, QueriesUsed: 3000},
			"subscription": &entities.SubscriptionMetadata{Tier: "pro"}, // no SubscriptionItemID
		},
	}); err != nil {
		t.Fatalf("save tenant: %v", err)
	}

	calcUC := NewCalculateOverageUseCase(repo)
	uc := NewReportUsageUseCase(repo, auditRepo, lsClient, calcUC)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.Skipped {
		t.Error("expected skipped — no subscription item ID")
	}
}

func TestReportUsage_BothOverage(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	lsClient := &mockLSClient{}

	if err := repo.Save(&entities.Tenant{
		ID:     "t1",
		Name:   "test-tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"overage":      &entities.OverageMetadata{Enabled: true, EventRate: 0.001, QueryRate: 0.002},
			"quotas":       &entities.QuotaMetadata{EventsQuota: 10000, QueriesQuota: 5000, EventsUsed: 12000, QueriesUsed: 7000},
			"subscription": &entities.SubscriptionMetadata{Tier: "pro", SubscriptionItemID: "si_1"},
		},
	}); err != nil {
		t.Fatalf("save tenant: %v", err)
	}

	calcUC := NewCalculateOverageUseCase(repo)
	uc := NewReportUsageUseCase(repo, auditRepo, lsClient, calcUC)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.EventsReported != 2000 {
		t.Errorf("expected 2000 events reported, got %d", result.EventsReported)
	}
	if result.QueriesReported != 2000 {
		t.Errorf("expected 2000 queries reported, got %d", result.QueriesReported)
	}
	// 2 usage reports: one for events, one for queries
	if len(lsClient.reportedUsages) != 2 {
		t.Fatalf("expected 2 usage reports, got %d", len(lsClient.reportedUsages))
	}
}
