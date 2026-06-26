package billing

import (
	"context"
	"testing"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// eventsMockCore returns a fixed total_count for QueryEvents; everything else
// would panic (nil interface) — keeps the test honest about what's called.
type eventsMockCore struct {
	clients.CoreClient
	total int
}

func (m *eventsMockCore) QueryEvents(_ context.Context, _ clients.QueryEventsRequest) (*clients.QueryEventsResponse, error) {
	return &clients.QueryEventsResponse{TotalCount: m.total}, nil
}

func seedTenant(t *testing.T, repo *persistence.MemoryTenantRepository, id string, used int64) {
	t.Helper()
	if err := repo.Save(&entities.Tenant{
		ID:     id,
		Name:   id,
		Status: entities.TenantStatusActive,
		Metadata: map[string]interface{}{
			"quotas": &entities.QuotaMetadata{EventsQuota: 5_000_000, EventsUsed: used},
		},
	}); err != nil {
		t.Fatalf("seed %s: %v", id, err)
	}
}

func TestSyncEventsUsage_CorrectsDrift(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	audit := persistence.NewMemoryAuditRepository()
	// Drifted: meter says 1,000,000 but the real store count is 143,002.
	seedTenant(t, repo, "t1", 1_000_000)
	uc := NewSyncEventsUsageUseCase(repo, audit, &eventsMockCore{total: 143_002})

	res, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if res.Skipped || res.EventsUsed != 143_002 {
		t.Fatalf("expected events_used corrected to 143002, got %+v", res)
	}
	tn, _ := repo.FindByID("t1")
	if got := extractQuotas(tn.Metadata).EventsUsed; got != 143_002 {
		t.Fatalf("persisted events_used = %d, want 143002", got)
	}
}

func TestSyncEventsUsage_SkipsWhenAligned(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	audit := persistence.NewMemoryAuditRepository()
	seedTenant(t, repo, "t1", 50_000)
	uc := NewSyncEventsUsageUseCase(repo, audit, &eventsMockCore{total: 50_000})

	res, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if !res.Skipped {
		t.Fatalf("expected skip when meter already matches, got %+v", res)
	}
}

func TestSyncEventsUsage_NilCoreSkips(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	seedTenant(t, repo, "t1", 1_000_000)
	uc := NewSyncEventsUsageUseCase(repo, persistence.NewMemoryAuditRepository(), nil)
	res, _ := uc.Execute(context.Background(), "t1")
	if !res.Skipped {
		t.Fatalf("nil core client must skip, got %+v", res)
	}
}
