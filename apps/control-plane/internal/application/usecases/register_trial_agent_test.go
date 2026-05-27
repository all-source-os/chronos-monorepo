package usecases

import (
	"context"
	"strings"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// fixedClock returns a deterministic time so expires_at assertions are
// reproducible. 2026-05-26T00:00:00Z + the trial duration = clean RFC3339.
func fixedClock(t time.Time) func() time.Time {
	return func() time.Time { return t }
}

func TestRegisterTrialAgent(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createTenantUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	mock := &mockCoreClient{}

	now := time.Date(2026, 5, 26, 0, 0, 0, 0, time.UTC)
	uc := NewRegisterTrialAgentUseCase(createTenantUC, auditRepo, mock, stubKeySigner).
		WithClock(fixedClock(now)).
		WithClaimBaseURL("https://test.example/connect?claim=")

	t.Run("mints a trial tenant with the proposal's wire shape", func(t *testing.T) {
		mock.events = nil

		resp, err := uc.Execute(context.Background(), dto.RegisterTrialAgentRequest{
			ClientFingerprint: "ua-test-123",
		})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}

		// Tenant ID is "trial-<8-hex>" so the source is obvious in logs.
		if !strings.HasPrefix(resp.TenantID, "trial-") {
			t.Errorf("tenant_id should start with 'trial-', got %q", resp.TenantID)
		}
		if len(resp.TenantID) != len("trial-")+16 {
			t.Errorf("expected 16-hex tenant_id suffix, got %q (len=%d)", resp.TenantID, len(resp.TenantID))
		}
		if resp.APIKey == "" {
			t.Error("api_key must be returned")
		}
		if resp.Tier != "trial" {
			t.Errorf("tier should be 'trial', got %q", resp.Tier)
		}
		if resp.Quotas.EventsQuota != TrialEventsQuota {
			t.Errorf("events_quota should be %d, got %d", TrialEventsQuota, resp.Quotas.EventsQuota)
		}
		if resp.Quotas.QueriesQuota != TrialQueriesQuota {
			t.Errorf("queries_quota should be %d, got %d", TrialQueriesQuota, resp.Quotas.QueriesQuota)
		}
		expectedExpiry := now.Add(TrialValidDuration).UTC().Format(time.RFC3339)
		if resp.ExpiresAt != expectedExpiry {
			t.Errorf("expires_at should be %q, got %q", expectedExpiry, resp.ExpiresAt)
		}
		if resp.ClaimToken == "" {
			t.Error("claim_token must be returned for migration")
		}
		expectedClaimURL := "https://test.example/connect?claim=" + resp.ClaimToken
		if resp.ClaimURL != expectedClaimURL {
			t.Errorf("claim_url should be %q, got %q", expectedClaimURL, resp.ClaimURL)
		}
	})

	t.Run("each call mints a fresh tenant — no idempotency", func(t *testing.T) {
		resp1, err := uc.Execute(context.Background(), dto.RegisterTrialAgentRequest{})
		if err != nil {
			t.Fatalf("first Execute failed: %v", err)
		}
		resp2, err := uc.Execute(context.Background(), dto.RegisterTrialAgentRequest{})
		if err != nil {
			t.Fatalf("second Execute failed: %v", err)
		}
		if resp1.TenantID == resp2.TenantID {
			t.Errorf("two calls should produce distinct tenants, both got %q", resp1.TenantID)
		}
		if resp1.ClaimToken == resp2.ClaimToken {
			t.Error("two calls should produce distinct claim tokens")
		}
	})

	t.Run("writes agent.trial_registered event for the audit trail", func(t *testing.T) {
		mock.events = nil
		_, err := uc.Execute(context.Background(), dto.RegisterTrialAgentRequest{})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}
		var found bool
		for _, e := range mock.events {
			if e.EventType == "agent.trial_registered" {
				found = true
				if e.Payload["tier"] != "trial" {
					t.Errorf("event payload tier should be 'trial', got %v", e.Payload["tier"])
				}
				if e.Payload["events_quota"] != TrialEventsQuota {
					t.Errorf("event payload events_quota should be %d, got %v", TrialEventsQuota, e.Payload["events_quota"])
				}
				break
			}
		}
		if !found {
			t.Errorf("expected agent.trial_registered event, got events: %+v", mock.events)
		}
	})

	t.Run("client_fingerprint flows into tenant metadata", func(t *testing.T) {
		// Smoke test: we can't easily inspect tenant metadata via the public
		// response, but a key signing failure surfaces wiring issues.
		// More importantly, verify the request shape accepts the field.
		_, err := uc.Execute(context.Background(), dto.RegisterTrialAgentRequest{
			ClientFingerprint: "abc-123-def",
		})
		if err != nil {
			t.Fatalf("Execute with client_fingerprint failed: %v", err)
		}
	})
}

func TestRegisterTrialAgent_KeySignerFailure(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createTenantUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	mock := &mockCoreClient{}

	uc := NewRegisterTrialAgentUseCase(createTenantUC, auditRepo, mock, failingKeySigner)

	_, err := uc.Execute(context.Background(), dto.RegisterTrialAgentRequest{})
	if err == nil {
		t.Fatal("expected error when key signer fails, got nil")
	}
	if !strings.Contains(err.Error(), "sign API key") {
		t.Errorf("error should mention key signing, got: %v", err)
	}
}

func TestRegisterTrialAgent_DefaultClaimURL(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createTenantUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	mock := &mockCoreClient{}

	// No WithClaimBaseURL → falls back to the production URL.
	uc := NewRegisterTrialAgentUseCase(createTenantUC, auditRepo, mock, stubKeySigner)

	resp, err := uc.Execute(context.Background(), dto.RegisterTrialAgentRequest{})
	if err != nil {
		t.Fatalf("Execute failed: %v", err)
	}
	if !strings.HasPrefix(resp.ClaimURL, "https://www.all-source.xyz/connect?claim=") {
		t.Errorf("default claim_url should point at the production /connect deep-link, got %q", resp.ClaimURL)
	}
}
