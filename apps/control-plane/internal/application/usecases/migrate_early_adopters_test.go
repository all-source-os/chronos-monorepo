package usecases

import (
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func migrateSetup(t *testing.T) (*MigrateEarlyAdoptersUseCase, *persistence.MemoryTenantRepository) {
	t.Helper()
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	updateSubUC := NewUpdateSubscriptionMetadataUseCase(repo, auditRepo)
	return NewMigrateEarlyAdoptersUseCase(repo, updateSubUC, auditRepo), repo
}

func saveTenantTier(t *testing.T, repo *persistence.MemoryTenantRepository, id, tier string) {
	t.Helper()
	md := map[string]interface{}{}
	if tier != "" {
		md["subscription"] = &entities.SubscriptionMetadata{Tier: tier, Status: "active"}
	}
	if err := repo.Save(&entities.Tenant{
		ID: id, Name: id, Status: entities.TenantStatusActive,
		CreatedAt: time.Now(), UpdatedAt: time.Now(), Metadata: md,
	}); err != nil {
		t.Fatalf("save %s: %v", id, err)
	}
}

func TestMigrateEarlyAdopters_AppliesVoucherAndOwner(t *testing.T) {
	uc, repo := migrateSetup(t)
	saveTenantTier(t, repo, "owner", "")        // free → owner comp
	saveTenantTier(t, repo, "free-1", "")       // free → voucher
	saveTenantTier(t, repo, "paid-1", "studio") // already paid → skip

	now := time.Date(2026, 6, 12, 0, 0, 0, 0, time.UTC)
	rep := uc.Execute(MigrateEarlyAdoptersRequest{VoucherTier: "studio", VoucherDays: 365, OwnerTenantID: "owner"}, now)

	if rep.Migrated != 2 || rep.Skipped != 1 || rep.Failed != 0 {
		t.Fatalf("counts = %+v, want migrated=2 skipped=1 failed=0", rep)
	}
	owner, _ := repo.FindByID("owner")                                                                      //nolint:errcheck // test assertion reads state seeded above
	if got := owner.Metadata["subscription"].(*entities.SubscriptionMetadata); got.Tier != tierEnterprise { //nolint:forcetypeassert,errcheck // a wrong type must fail this test loudly
		t.Errorf("owner tier = %q, want enterprise", got.Tier)
	}
	free, _ := repo.FindByID("free-1")                                   //nolint:errcheck // test assertion reads state seeded above
	fs := free.Metadata["subscription"].(*entities.SubscriptionMetadata) //nolint:forcetypeassert,errcheck // a wrong type must fail this test loudly
	if fs.Tier != "studio" || fs.GrandfatherUntil == nil {
		t.Errorf("free-1 should be studio with a voucher, got %+v", fs)
	}
}

func TestMigrateEarlyAdopters_DryRunWritesNothing(t *testing.T) {
	uc, repo := migrateSetup(t)
	saveTenantTier(t, repo, "free-1", "")

	rep := uc.Execute(MigrateEarlyAdoptersRequest{DryRun: true}, time.Now())
	if !rep.DryRun || rep.Migrated != 1 {
		t.Fatalf("dry-run report = %+v", rep)
	}
	// Tenant must still be free — nothing persisted.
	tn, _ := repo.FindByID("free-1") //nolint:errcheck // test assertion reads state seeded above
	if sub, ok := tn.Metadata["subscription"]; ok && sub != nil {
		if s, ok := sub.(*entities.SubscriptionMetadata); ok && s.Tier != "" {
			t.Errorf("dry-run wrote a tier: %q", s.Tier)
		}
	}
}

func TestMigrateEarlyAdopters_Idempotent(t *testing.T) {
	uc, repo := migrateSetup(t)
	saveTenantTier(t, repo, "free-1", "")

	first := uc.Execute(MigrateEarlyAdoptersRequest{}, time.Now())
	if first.Migrated != 1 {
		t.Fatalf("first run migrated=%d, want 1", first.Migrated)
	}
	// Second run: now on a paid tier → skipped, not re-migrated.
	second := uc.Execute(MigrateEarlyAdoptersRequest{}, time.Now())
	if second.Migrated != 0 || second.Skipped != 1 {
		t.Errorf("second run = %+v, want migrated=0 skipped=1", second)
	}
}

func TestMigrateEarlyAdopters_DefaultsApplied(t *testing.T) {
	uc, repo := migrateSetup(t)
	saveTenantTier(t, repo, "free-1", "")

	rep := uc.Execute(MigrateEarlyAdoptersRequest{}, time.Now()) // no tier/days
	if rep.Migrated != 1 {
		t.Fatalf("migrated=%d", rep.Migrated)
	}
	tn, _ := repo.FindByID("free-1")                                                                         //nolint:errcheck // test assertion reads state seeded above
	if got := tn.Metadata["subscription"].(*entities.SubscriptionMetadata).Tier; got != defaultVoucherTier { //nolint:forcetypeassert,errcheck // a wrong type must fail this test loudly
		t.Errorf("default tier = %q, want %q", got, defaultVoucherTier)
	}
}
