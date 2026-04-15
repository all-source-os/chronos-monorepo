package billing

import (
	"context"
	"testing"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// mockEmailClient records emails sent for testing.
type mockEmailClient struct {
	sent []clients.SendEmailRequest
}

func (m *mockEmailClient) SendEmail(_ context.Context, req clients.SendEmailRequest) error {
	m.sent = append(m.sent, req)
	return nil
}

func setupWarningTestTenant(t *testing.T, tenantRepo *persistence.MemoryTenantRepository, id string, eventsUsed, eventsQuota int64, email, tier string, lastWarningPct int) {
	t.Helper()
	tenant := &entities.Tenant{
		ID:     id,
		Name:   "Test Tenant",
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"email": email,
			"quotas": &entities.QuotaMetadata{
				EventsUsed:  eventsUsed,
				EventsQuota: eventsQuota,
			},
			"subscription": &entities.SubscriptionMetadata{
				Tier: tier,
			},
		},
	}
	if lastWarningPct > 0 {
		tenant.Metadata["usage_warnings"] = &UsageWarningMetadata{
			LastWarningPct: lastWarningPct,
		}
	}
	if err := tenantRepo.Save(tenant); err != nil {
		t.Fatalf("save tenant: %v", err)
	}
}

func TestCheckUsageWarnings_80Percent(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	emailClient := &mockEmailClient{}
	uc := NewCheckUsageWarningsUseCase(tenantRepo, auditRepo, emailClient)

	setupWarningTestTenant(t, tenantRepo, "t1", 8500, 10000, "user@example.com", "free", 0)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !result.EmailSent {
		t.Error("expected email to be sent")
	}
	if result.ThresholdPct != 80 {
		t.Errorf("expected threshold 80, got %d", result.ThresholdPct)
	}
	if len(emailClient.sent) != 1 {
		t.Fatalf("expected 1 email sent, got %d", len(emailClient.sent))
	}
	if emailClient.sent[0].To != "user@example.com" {
		t.Errorf("expected email to user@example.com, got %s", emailClient.sent[0].To)
	}
	if emailClient.sent[0].Subject != "AllSource Chronos: You're approaching your event quota" {
		t.Errorf("unexpected subject: %s", emailClient.sent[0].Subject)
	}
}

func TestCheckUsageWarnings_100Percent(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	emailClient := &mockEmailClient{}
	uc := NewCheckUsageWarningsUseCase(tenantRepo, auditRepo, emailClient)

	setupWarningTestTenant(t, tenantRepo, "t1", 10000, 10000, "user@example.com", "free", 0)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !result.EmailSent {
		t.Error("expected email to be sent")
	}
	if result.ThresholdPct != 100 {
		t.Errorf("expected threshold 100, got %d", result.ThresholdPct)
	}
	if emailClient.sent[0].Subject != "AllSource Chronos: You've reached your event quota" {
		t.Errorf("unexpected subject: %s", emailClient.sent[0].Subject)
	}
}

func TestCheckUsageWarnings_BelowThreshold(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	emailClient := &mockEmailClient{}
	uc := NewCheckUsageWarningsUseCase(tenantRepo, auditRepo, emailClient)

	setupWarningTestTenant(t, tenantRepo, "t1", 5000, 10000, "user@example.com", "free", 0)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !result.Skipped {
		t.Error("expected to be skipped (below threshold)")
	}
	if len(emailClient.sent) != 0 {
		t.Errorf("expected no emails, got %d", len(emailClient.sent))
	}
}

func TestCheckUsageWarnings_AlreadySent80(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	emailClient := &mockEmailClient{}
	uc := NewCheckUsageWarningsUseCase(tenantRepo, auditRepo, emailClient)

	// 85% usage, but 80% warning already sent
	setupWarningTestTenant(t, tenantRepo, "t1", 8500, 10000, "user@example.com", "free", 80)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !result.Skipped {
		t.Error("expected to be skipped (already sent 80% warning)")
	}
	if len(emailClient.sent) != 0 {
		t.Errorf("expected no emails, got %d", len(emailClient.sent))
	}
}

func TestCheckUsageWarnings_Send100AfterSending80(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	emailClient := &mockEmailClient{}
	uc := NewCheckUsageWarningsUseCase(tenantRepo, auditRepo, emailClient)

	// 100% usage, 80% warning already sent — should still send 100%
	setupWarningTestTenant(t, tenantRepo, "t1", 10000, 10000, "user@example.com", "free", 80)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !result.EmailSent {
		t.Error("expected email to be sent (100% after 80%)")
	}
	if result.ThresholdPct != 100 {
		t.Errorf("expected threshold 100, got %d", result.ThresholdPct)
	}
}

func TestCheckUsageWarnings_NoEmail(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	emailClient := &mockEmailClient{}
	uc := NewCheckUsageWarningsUseCase(tenantRepo, auditRepo, emailClient)

	// 90% usage but no email on file
	setupWarningTestTenant(t, tenantRepo, "t1", 9000, 10000, "", "free", 0)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !result.Skipped {
		t.Error("expected to be skipped (no email)")
	}
}

func TestCheckUsageWarnings_ProUpgradeCTA(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	emailClient := &mockEmailClient{}
	uc := NewCheckUsageWarningsUseCase(tenantRepo, auditRepo, emailClient)

	setupWarningTestTenant(t, tenantRepo, "t1", 450000, 500000, "pro@example.com", "pro", 0)

	_, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(emailClient.sent) != 1 {
		t.Fatalf("expected 1 email, got %d", len(emailClient.sent))
	}
	body := emailClient.sent[0].Body
	if !containsString(body, "upgrade to Growth") {
		t.Error("expected Pro tier email to mention Growth upgrade")
	}
}

func TestCheckUsageWarnings_UnlimitedQuota(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	emailClient := &mockEmailClient{}
	uc := NewCheckUsageWarningsUseCase(tenantRepo, auditRepo, emailClient)

	// Unlimited quota (-1) should be skipped
	setupWarningTestTenant(t, tenantRepo, "t1", 999999, -1, "user@example.com", "team", 0)

	result, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !result.Skipped {
		t.Error("expected unlimited quota to be skipped")
	}
}

func TestBuildWarningEmail_80Percent(t *testing.T) {
	quotas := entities.QuotaMetadata{EventsUsed: 8500, EventsQuota: 10000}
	subject, body := buildWarningEmail(80, quotas, "free", "Acme Corp")

	if subject != "AllSource Chronos: You're approaching your event quota" {
		t.Errorf("unexpected subject: %s", subject)
	}
	if !containsString(body, "80%") {
		t.Error("expected body to mention 80%")
	}
	if !containsString(body, "8500 / 10000") {
		t.Error("expected body to include usage numbers")
	}
	if !containsString(body, "upgrade to Pro") {
		t.Error("expected free tier to mention Pro upgrade")
	}
}

func TestBuildWarningEmail_100Percent(t *testing.T) {
	quotas := entities.QuotaMetadata{EventsUsed: 10000, EventsQuota: 10000}
	subject, body := buildWarningEmail(100, quotas, "free", "")

	if subject != "AllSource Chronos: You've reached your event quota" {
		t.Errorf("unexpected subject: %s", subject)
	}
	if !containsString(body, "100%") {
		t.Error("expected body to mention 100%")
	}
	if !containsString(body, "Hi there") {
		t.Error("expected fallback greeting when name is empty")
	}
}

func containsString(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || s != "" && contains(s, substr))
}

func contains(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
