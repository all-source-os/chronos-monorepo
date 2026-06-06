package usecases

import (
	"context"
	"testing"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// mockCoreClient records IngestEvent calls for verification.
type mockCoreClient struct {
	clients.CoreClient
	events []clients.IngestEventRequest
}

func (m *mockCoreClient) IngestEvent(_ context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error) {
	m.events = append(m.events, req)
	return &clients.IngestEventResponse{EventID: "evt-test-001"}, nil
}

func TestResolveTier(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	updateSubUC := NewUpdateSubscriptionMetadataUseCase(tenantRepo, auditRepo)
	suspendUC := NewSuspendTenantUseCase(tenantRepo, auditRepo)

	t.Run("uses variant map when configured", func(t *testing.T) {
		variantMap := VariantTierMap{
			"12345":      "growth",
			"67890":      "enterprise",
			"growth":     "growth",
			"enterprise": "enterprise",
		}
		uc := NewProcessLemonSqueezyWebhookUseCase(tenantRepo, auditRepo, updateSubUC, suspendUC, variantMap)

		// Match by variant ID
		if tier := uc.resolveTier("SomeRandomName", 12345); tier != "growth" {
			t.Errorf("expected growth, got %s", tier)
		}

		// Match by variant name in map
		if tier := uc.resolveTier("enterprise", 0); tier != "enterprise" {
			t.Errorf("expected enterprise, got %s", tier)
		}
	})

	t.Run("falls back to hardcoded mapping without variant map", func(t *testing.T) {
		uc := NewProcessLemonSqueezyWebhookUseCase(tenantRepo, auditRepo, updateSubUC, suspendUC, nil)

		// New 011 paid tiers resolve to themselves (critical: must NOT fall
		// through to the free default, which would record a paying customer as free).
		if tier := uc.resolveTier("Indie", 0); tier != "indie" {
			t.Errorf("expected indie, got %s", tier)
		}
		if tier := uc.resolveTier("Studio", 0); tier != "studio" {
			t.Errorf("expected studio, got %s", tier)
		}
		if tier := uc.resolveTier("Scale", 0); tier != "scale" {
			t.Errorf("expected scale, got %s", tier)
		}
		// Retired variant names still resolve for in-flight subscriptions.
		if tier := uc.resolveTier("Growth", 0); tier != "growth" {
			t.Errorf("expected growth, got %s", tier)
		}
		if tier := uc.resolveTier("Pro", 0); tier != "growth" {
			t.Errorf("expected growth, got %s", tier)
		}
		if tier := uc.resolveTier("Team", 0); tier != "team" {
			t.Errorf("expected team, got %s", tier)
		}
		if tier := uc.resolveTier("Enterprise", 0); tier != "enterprise" {
			t.Errorf("expected enterprise, got %s", tier)
		}
		if tier := uc.resolveTier("Unknown", 0); tier != "free" {
			t.Errorf("expected free, got %s", tier)
		}
	})

	t.Run("variant map takes precedence over hardcoded", func(t *testing.T) {
		variantMap := VariantTierMap{
			"Pro": "custom-pro-tier",
		}
		uc := NewProcessLemonSqueezyWebhookUseCase(tenantRepo, auditRepo, updateSubUC, suspendUC, variantMap)

		if tier := uc.resolveTier("Pro", 0); tier != "custom-pro-tier" {
			t.Errorf("expected custom-pro-tier, got %s", tier)
		}
	})
}

func TestExtractTenantIDFromWebhook(t *testing.T) {
	t.Run("extracts from map[string]interface{}", func(t *testing.T) {
		event := LemonSqueezyWebhookEvent{
			Meta: map[string]interface{}{
				"custom_data": map[string]interface{}{
					"tenant_id": "tenant-abc-123",
				},
			},
		}
		if id := extractTenantIDFromWebhook(event); id != "tenant-abc-123" {
			t.Errorf("expected tenant-abc-123, got %s", id)
		}
	})

	t.Run("extracts from map[string]string", func(t *testing.T) {
		event := LemonSqueezyWebhookEvent{
			Meta: map[string]interface{}{
				"custom_data": map[string]string{
					"tenant_id": "tenant-xyz-789",
				},
			},
		}
		if id := extractTenantIDFromWebhook(event); id != "tenant-xyz-789" {
			t.Errorf("expected tenant-xyz-789, got %s", id)
		}
	})

	t.Run("returns empty for missing custom_data", func(t *testing.T) {
		event := LemonSqueezyWebhookEvent{
			Meta: map[string]interface{}{},
		}
		if id := extractTenantIDFromWebhook(event); id != "" {
			t.Errorf("expected empty, got %s", id)
		}
	})

	t.Run("returns empty for nil meta", func(t *testing.T) {
		event := LemonSqueezyWebhookEvent{}
		if id := extractTenantIDFromWebhook(event); id != "" {
			t.Errorf("expected empty, got %s", id)
		}
	})
}

func TestWebhookPropagatesBillingPeriodAndTier(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	updateSubUC := NewUpdateSubscriptionMetadataUseCase(tenantRepo, auditRepo)
	suspendUC := NewSuspendTenantUseCase(tenantRepo, auditRepo)
	uc := NewProcessLemonSqueezyWebhookUseCase(tenantRepo, auditRepo, updateSubUC, suspendUC, nil)

	if _, err := createUC.Execute(dto.CreateTenantRequest{ID: "t-bp", Name: "T"}); err != nil {
		t.Fatalf("setup: %v", err)
	}

	// A new Studio subscription with an annual billing period; both must land in
	// stored subscription metadata (tier no longer falls through to free, and
	// billing_period is carried from checkout custom_data).
	event := LemonSqueezyWebhookEvent{
		EventName: "subscription_created",
		Data: LemonSqueezyEventData{
			ID:   "sub_bp",
			Type: "subscriptions",
			Attributes: LemonSqueezySubscriptionAttrs{
				CustomerID:  7,
				VariantName: "Studio",
				Status:      "active",
			},
		},
		Meta: map[string]interface{}{
			"custom_data": map[string]interface{}{
				"tenant_id":      "t-bp",
				"tier":           "studio",
				"billing_period": "annual",
			},
		},
	}
	if err := uc.Execute(context.Background(), event); err != nil {
		t.Fatalf("Execute: %v", err)
	}

	tenant, err := tenantRepo.FindByID("t-bp")
	if err != nil {
		t.Fatalf("find tenant: %v", err)
	}
	sub, ok := tenant.Metadata["subscription"].(*entities.SubscriptionMetadata)
	if !ok {
		t.Fatalf("subscription metadata type: got %T", tenant.Metadata["subscription"])
	}
	if sub.Tier != "studio" {
		t.Errorf("tier: want studio, got %q", sub.Tier)
	}
	if sub.BillingPeriod != "annual" {
		t.Errorf("billing_period: want annual, got %q", sub.BillingPeriod)
	}
}

func TestWebhookWritesBillingEventToCore(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	updateSubUC := NewUpdateSubscriptionMetadataUseCase(tenantRepo, auditRepo)
	suspendUC := NewSuspendTenantUseCase(tenantRepo, auditRepo)
	mock := &mockCoreClient{}

	uc := NewProcessLemonSqueezyWebhookUseCase(tenantRepo, auditRepo, updateSubUC, suspendUC, nil, mock)

	// Create tenant first
	_, err := createUC.Execute(dto.CreateTenantRequest{ID: "tenant-billing-1", Name: "Billing Tenant"})
	if err != nil {
		t.Fatalf("setup: %v", err)
	}

	t.Run("subscription_created writes billing event to Core", func(t *testing.T) {
		mock.events = nil // reset

		event := LemonSqueezyWebhookEvent{
			EventName: "subscription_created",
			Data: LemonSqueezyEventData{
				ID:   "sub_001",
				Type: "subscriptions",
				Attributes: LemonSqueezySubscriptionAttrs{
					CustomerID:  42,
					VariantID:   12345,
					VariantName: "Growth",
					ProductName: "AllSource Subscription",
					Status:      "active",
				},
			},
			Meta: map[string]interface{}{
				"custom_data": map[string]interface{}{
					"tenant_id": "tenant-billing-1",
				},
			},
		}

		err := uc.Execute(context.Background(), event)
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}

		if len(mock.events) != 1 {
			t.Fatalf("expected 1 Core event, got %d", len(mock.events))
		}

		evt := mock.events[0]
		if evt.EventType != "billing.subscription_created" {
			t.Errorf("expected billing.subscription_created, got %s", evt.EventType)
		}
		if evt.EntityID != "tenant-billing-1" {
			t.Errorf("expected entity_id tenant-billing-1, got %s", evt.EntityID)
		}
		if evt.Payload["tier"] != "growth" {
			t.Errorf("expected tier growth, got %v", evt.Payload["tier"])
		}
		if evt.Payload["payment_provider"] != "lemonsqueezy" {
			t.Errorf("expected payment_provider lemonsqueezy, got %v", evt.Payload["payment_provider"])
		}
	})

	t.Run("subscription_cancelled writes billing event to Core", func(t *testing.T) { //nolint:misspell // LemonSqueezy API event name
		mock.events = nil

		event := LemonSqueezyWebhookEvent{
			EventName: "subscription_cancelled", //nolint:misspell // LemonSqueezy API event name
			Data: LemonSqueezyEventData{
				ID:   "sub_002",
				Type: "subscriptions",
				Attributes: LemonSqueezySubscriptionAttrs{
					CustomerID:  42,
					VariantID:   12345,
					VariantName: "Growth",
					Status:      "cancelled", //nolint:misspell // LemonSqueezy API status
				},
			},
			Meta: map[string]interface{}{
				"custom_data": map[string]interface{}{
					"tenant_id": "tenant-billing-1",
				},
			},
		}

		err := uc.Execute(context.Background(), event)
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}

		if len(mock.events) != 1 {
			t.Fatalf("expected 1 Core event, got %d", len(mock.events))
		}
		if mock.events[0].EventType != "billing.subscription_cancelled" { //nolint:misspell // LemonSqueezy API event name
			t.Errorf("expected billing.subscription_cancelled, got %s", mock.events[0].EventType) //nolint:misspell // LemonSqueezy API event name
		}
	})

	t.Run("no Core event written when coreClient is nil", func(t *testing.T) {
		ucNilCore := NewProcessLemonSqueezyWebhookUseCase(tenantRepo, auditRepo, updateSubUC, suspendUC, nil)

		event := LemonSqueezyWebhookEvent{
			EventName: "subscription_created",
			Data: LemonSqueezyEventData{
				Attributes: LemonSqueezySubscriptionAttrs{Status: "active"},
			},
			Meta: map[string]interface{}{
				"custom_data": map[string]interface{}{"tenant_id": "tenant-billing-1"},
			},
		}

		// Should not panic — just skip Core write
		err := ucNilCore.Execute(context.Background(), event)
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}
	})
}
