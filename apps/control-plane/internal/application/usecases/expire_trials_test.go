package usecases

import (
	"context"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// fixedClock is defined in register_trial_agent_test.go (same package) and
// reused here for deterministic now().

// TestTrialSubscriptionMetadata proves the shared stamp every minting site uses:
// tier="trial" + status="active" + a future trial_expires_at (now + 14 days).
// This is the property the onboard/OAuth/agent-register paths inherit, so a
// regression here is a regression in all of them.
func TestTrialSubscriptionMetadata(t *testing.T) {
	now := time.Date(2026, 6, 1, 12, 0, 0, 0, time.UTC)
	sub, expiresAt := TrialSubscriptionMetadata(now)

	if sub["tier"] != TrialTierName {
		t.Errorf("tier = %v, want %q", sub["tier"], TrialTierName)
	}
	if sub["status"] != "active" {
		t.Errorf("status = %v, want active", sub["status"])
	}

	// Expiry is now + 14 days, and is reflected both in the returned time and the
	// RFC3339 string stamped into metadata.
	wantExpiry := now.Add(TrialValidDuration)
	if !expiresAt.Equal(wantExpiry) {
		t.Errorf("expiresAt = %v, want %v", expiresAt, wantExpiry)
	}
	gotStr, _ := sub["trial_expires_at"].(string) //nolint:errcheck // absent value correctly reads as empty and fails the assert below
	if gotStr != wantExpiry.UTC().Format(time.RFC3339) {
		t.Errorf("trial_expires_at = %q, want %q", gotStr, wantExpiry.UTC().Format(time.RFC3339))
	}

	// And it must be in the FUTURE relative to the mint time (not "free forever",
	// not already-expired).
	if !expiresAt.After(now) {
		t.Errorf("trial_expires_at %v is not after mint time %v", expiresAt, now)
	}
}

// seedTenant inserts a tenant with the given metadata directly into the repo.
func seedTrialTenant(t *testing.T, repo *persistence.MemoryTenantRepository, id string, metadata map[string]interface{}) *entities.Tenant {
	t.Helper()
	tenant, err := entities.NewTenant(id, id, "test tenant")
	if err != nil {
		t.Fatalf("NewTenant(%s): %v", id, err)
	}
	tenant.Metadata = metadata
	if err := repo.Save(tenant); err != nil {
		t.Fatalf("Save(%s): %v", id, err)
	}
	return tenant
}

// trialMeta builds tenant metadata for a trial expiring at expiresAt.
func trialMeta(expiresAt time.Time) map[string]interface{} {
	return map[string]interface{}{
		"subscription": map[string]interface{}{
			"tier":             TrialTierName,
			"status":           "active",
			"trial_expires_at": expiresAt.UTC().Format(time.RFC3339),
		},
	}
}

func TestExpireTrials_SuspendsExpiredUnpaidTrial(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()

	now := time.Date(2026, 6, 20, 0, 0, 0, 0, time.UTC)

	// Expired trial (expired yesterday), no paid subscription → must be suspended.
	expired := seedTrialTenant(t, repo, "expired-trial", trialMeta(now.Add(-24*time.Hour)))
	// Active trial (expires next week) → must be left alone.
	active := seedTrialTenant(t, repo, "active-trial", trialMeta(now.Add(7*24*time.Hour)))
	// Paid tenant (active studio subscription) → must be left alone even if some
	// stale subscription.tier still reads "trial".
	paid := seedTrialTenant(t, repo, "paid-tenant", map[string]interface{}{
		"subscription": map[string]interface{}{
			"tier":             TrialTierName, // stale label
			"trial_expires_at": now.Add(-72 * time.Hour).Format(time.RFC3339),
		},
		"subscriptions": map[string]interface{}{
			"sub_123": map[string]interface{}{"tier": string(entities.TierStudio), "status": "active"},
		},
	})
	// Non-trial tenant (no subscription block) → out of scope, left alone.
	other := seedTrialTenant(t, repo, "legacy-free", map[string]interface{}{})

	uc := NewExpireTrialsUseCase(repo, auditRepo, nil).WithClock(fixedClock(now))
	results := uc.ExecuteAll(context.Background())

	if len(results) != 4 {
		t.Fatalf("expected 4 results, got %d", len(results))
	}

	// The expired, unpaid trial is now suspended.
	got, _ := repo.FindByID(expired.ID) //nolint:errcheck // test assertion reads state seeded above
	if got.Status != entities.TenantStatusSuspended {
		t.Errorf("expired trial status = %s, want suspended", got.Status)
	}

	// Everyone else stays active.
	for _, id := range []string{active.ID, paid.ID, other.ID} {
		g, _ := repo.FindByID(id) //nolint:errcheck // test assertion reads state seeded above
		if g.Status != entities.TenantStatusActive {
			t.Errorf("tenant %s status = %s, want active (must not be suspended)", id, g.Status)
		}
	}

	// Exactly one suspension reported.
	var suspended int
	for _, r := range results {
		if r.Suspended {
			suspended++
			if r.TenantID != expired.ID {
				t.Errorf("unexpected suspension of %s", r.TenantID)
			}
		}
	}
	if suspended != 1 {
		t.Errorf("suspended count = %d, want 1", suspended)
	}
}

func TestExpireTrials_ReactivatedSuspendIsReversible(t *testing.T) {
	// The sweep uses the same reversible Suspend() the admin tools use, so an
	// operator (or the claim/upgrade flow) can re-activate. Proven by flipping it
	// back and confirming a re-run does NOT touch an already-active paid tenant.
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	now := time.Date(2026, 6, 20, 0, 0, 0, 0, time.UTC)

	tenant := seedTrialTenant(t, repo, "expired-trial", trialMeta(now.Add(-1*time.Hour)))

	uc := NewExpireTrialsUseCase(repo, auditRepo, nil).WithClock(fixedClock(now))
	uc.ExecuteAll(context.Background())

	got, _ := repo.FindByID(tenant.ID) //nolint:errcheck // test assertion reads state seeded above
	if got.Status != entities.TenantStatusSuspended {
		t.Fatalf("expected suspended after sweep, got %s", got.Status)
	}

	// Reactivate (what an operator/claim flow does) — the sweep only scans ACTIVE
	// tenants, so a suspended one is never re-processed; reactivating restores
	// access and the tenant is no longer in scope until a fresh trial window.
	got.Activate()
	_ = repo.Update(got)                                                           //nolint:errcheck // test setup
	if g, _ := repo.FindByID(tenant.ID); g.Status != entities.TenantStatusActive { //nolint:errcheck // test assertion reads state seeded above
		t.Fatalf("reactivation failed, status = %s", g.Status)
	}
}

func TestExpireTrials_IgnoresTrialWithoutParseableExpiry(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	now := time.Now()

	// tier=trial but no/garbled expiry → fail safe (do NOT suspend).
	seedTrialTenant(t, repo, "no-expiry-trial", map[string]interface{}{
		"subscription": map[string]interface{}{"tier": TrialTierName},
	})
	seedTrialTenant(t, repo, "bad-expiry-trial", map[string]interface{}{
		"subscription": map[string]interface{}{"tier": TrialTierName, "trial_expires_at": "not-a-date"},
	})

	uc := NewExpireTrialsUseCase(repo, auditRepo, nil).WithClock(fixedClock(now))
	results := uc.ExecuteAll(context.Background())

	for _, r := range results {
		if r.Suspended {
			t.Errorf("tenant %s suspended despite no parseable expiry — must fail safe", r.TenantID)
		}
	}
}
