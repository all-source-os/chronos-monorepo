package usecases

import (
	"context"
	"errors"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func TestClaimTrialAgent_HappyPath(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createTenantUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	mock := &mockCoreClient{}

	now := time.Date(2026, 5, 27, 12, 0, 0, 0, time.UTC)
	mintUC := NewRegisterTrialAgentUseCase(createTenantUC, auditRepo, mock, stubKeySigner).
		WithClock(fixedClock(now))
	claimUC := NewClaimTrialAgentUseCase(tenantRepo, auditRepo, mock).
		WithClock(fixedClock(now.Add(1 * time.Hour))) // claim 1h later

	// Mint a trial first
	mintResp, err := mintUC.Execute(context.Background(), dto.RegisterTrialAgentRequest{
		ClientFingerprint: "ua-123",
	})
	if err != nil {
		t.Fatalf("mint failed: %v", err)
	}

	// Claim it
	mock.events = nil
	claimResp, err := claimUC.Execute(
		context.Background(),
		dto.ClaimTrialAgentRequest{ClaimToken: mintResp.ClaimToken},
		"real-tenant-alice",
	)
	if err != nil {
		t.Fatalf("claim failed: %v", err)
	}

	if claimResp.TrialTenantID != mintResp.TenantID {
		t.Errorf("trial_tenant_id mismatch: minted %q, claimed %q", mintResp.TenantID, claimResp.TrialTenantID)
	}
	if claimResp.ClaimedByTenant != "real-tenant-alice" {
		t.Errorf("claimed_by_tenant should be the caller, got %q", claimResp.ClaimedByTenant)
	}
	if claimResp.EventsMigrated {
		t.Error("events_migrated should be false in v1")
	}
	if claimResp.EventsMigrationNote == "" {
		t.Error("events_migration_note should be set to explain the deferral")
	}

	// Tenant metadata must record the claim
	trial, err := tenantRepo.FindByID(mintResp.TenantID)
	if err != nil {
		t.Fatalf("find trial after claim: %v", err)
	}
	if claimToken, _ := trial.Metadata["claim_token"].(string); claimToken != "" { //nolint:errcheck // type assertion's second value is a bool ok-flag, not an error
		t.Errorf("claim_token should be cleared after claim, got %q", claimToken)
	}
	if at, _ := trial.Metadata["claimed_at"].(string); at == "" { //nolint:errcheck // type assertion's second value is a bool ok-flag, not an error
		t.Error("claimed_at metadata should be set")
	}
	if by, _ := trial.Metadata["claimed_by_tenant"].(string); by != "real-tenant-alice" { //nolint:errcheck // type assertion's second value is a bool ok-flag, not an error
		t.Errorf("claimed_by_tenant metadata should be the caller, got %q", by)
	}

	// agent.trial_claimed event must be emitted
	var found bool
	for _, e := range mock.events {
		if e.EventType == "agent.trial_claimed" {
			found = true
			if e.Payload["claimed_by_tenant"] != "real-tenant-alice" {
				t.Errorf("event payload claimed_by_tenant should be 'real-tenant-alice', got %v", e.Payload["claimed_by_tenant"])
			}
			break
		}
	}
	if !found {
		t.Errorf("expected agent.trial_claimed event, got events: %+v", mock.events)
	}
}

func TestClaimTrialAgent_UnknownTokenReturnsNotFound(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	uc := NewClaimTrialAgentUseCase(tenantRepo, auditRepo, &mockCoreClient{})

	_, err := uc.Execute(
		context.Background(),
		dto.ClaimTrialAgentRequest{ClaimToken: "nonexistent-token"},
		"real-tenant-alice",
	)
	if !errors.Is(err, ErrClaimTokenNotFound) {
		t.Errorf("expected ErrClaimTokenNotFound, got %v", err)
	}
}

func TestClaimTrialAgent_ExpiredTrialReturnsExpired(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createTenantUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	mock := &mockCoreClient{}

	mintTime := time.Date(2026, 5, 1, 0, 0, 0, 0, time.UTC)
	mintUC := NewRegisterTrialAgentUseCase(createTenantUC, auditRepo, mock, stubKeySigner).
		WithClock(fixedClock(mintTime))
	mintResp, err := mintUC.Execute(context.Background(), dto.RegisterTrialAgentRequest{})
	if err != nil {
		t.Fatalf("mint failed: %v", err)
	}

	// Claim AFTER the 14-day TTL has passed
	claimTime := mintTime.Add(TrialValidDuration + 1*time.Hour)
	claimUC := NewClaimTrialAgentUseCase(tenantRepo, auditRepo, mock).WithClock(fixedClock(claimTime))

	_, err = claimUC.Execute(
		context.Background(),
		dto.ClaimTrialAgentRequest{ClaimToken: mintResp.ClaimToken},
		"real-tenant-bob",
	)
	if !errors.Is(err, ErrTrialExpired) {
		t.Errorf("expected ErrTrialExpired, got %v", err)
	}
}

func TestClaimTrialAgent_DoubleClaimReturnsNotFound(t *testing.T) {
	// Second claim with the same token must not succeed. v1 returns
	// NotFound rather than AlreadyClaimed because we don't keep a
	// history of consumed tokens (the trial's metadata.claim_token is
	// cleared on first claim, so the scan no longer matches).
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createTenantUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	mock := &mockCoreClient{}

	now := time.Date(2026, 5, 27, 12, 0, 0, 0, time.UTC)
	mintUC := NewRegisterTrialAgentUseCase(createTenantUC, auditRepo, mock, stubKeySigner).
		WithClock(fixedClock(now))
	mintResp, err := mintUC.Execute(context.Background(), dto.RegisterTrialAgentRequest{})
	if err != nil {
		t.Fatalf("mint failed: %v", err)
	}

	claimUC := NewClaimTrialAgentUseCase(tenantRepo, auditRepo, mock).WithClock(fixedClock(now.Add(time.Hour)))
	// First claim succeeds
	if _, err := claimUC.Execute(
		context.Background(),
		dto.ClaimTrialAgentRequest{ClaimToken: mintResp.ClaimToken},
		"first-claimer",
	); err != nil {
		t.Fatalf("first claim should succeed: %v", err)
	}
	// Second claim with the same token fails
	_, err = claimUC.Execute(
		context.Background(),
		dto.ClaimTrialAgentRequest{ClaimToken: mintResp.ClaimToken},
		"second-claimer",
	)
	if !errors.Is(err, ErrClaimTokenNotFound) {
		t.Errorf("expected ErrClaimTokenNotFound on double-claim, got %v", err)
	}
}

func TestClaimTrialAgent_RequiresClaimingTenantID(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	uc := NewClaimTrialAgentUseCase(tenantRepo, auditRepo, &mockCoreClient{})

	_, err := uc.Execute(
		context.Background(),
		dto.ClaimTrialAgentRequest{ClaimToken: "any"},
		"", // empty caller — defensive guard
	)
	if err == nil {
		t.Fatal("expected error when claimingTenantID is empty, got nil")
	}
}
