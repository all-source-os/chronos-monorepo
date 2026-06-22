package billing

import (
	"context"
	"testing"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// backfillMockCore returns a fixed total of events across paged QueryEvents calls.
// It embeds clients.CoreClient so only QueryEvents needs implementing.
type backfillMockCore struct {
	clients.CoreClient
	totalEvents int
	calls       int
}

func (m *backfillMockCore) QueryEvents(_ context.Context, req clients.QueryEventsRequest) (*clients.QueryEventsResponse, error) {
	m.calls++
	remaining := m.totalEvents - req.Offset
	if remaining < 0 {
		remaining = 0
	}
	page := remaining
	if page > req.Limit {
		page = req.Limit
	}
	return &clients.QueryEventsResponse{Events: make([]clients.EventEntry, page)}, nil
}

func newBackfillFixture(t *testing.T, totalEvents int) (*BackfillEventsUsedUseCase, *persistence.MemoryTenantRepository) {
	t.Helper()
	repo := persistence.NewMemoryTenantRepository()
	audit := persistence.NewMemoryAuditRepository()
	tenant, err := entities.NewTenant("t-backfill", "Backfill Tenant", "")
	if err != nil {
		t.Fatalf("setup: new tenant: %v", err)
	}
	if err := repo.Save(tenant); err != nil {
		t.Fatalf("setup: save tenant: %v", err)
	}
	core := &backfillMockCore{totalEvents: totalEvents}
	return NewBackfillEventsUsedUseCase(repo, audit, core), repo
}

func TestBackfillEventsUsed_WritesRealCount(t *testing.T) {
	uc, repo := newBackfillFixture(t, 870_000)

	res, err := uc.Execute(context.Background(), BackfillRequest{TenantID: "t-backfill"})
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}
	if res.EventsFound != 870_000 {
		t.Errorf("EventsFound = %d, want 870000", res.EventsFound)
	}
	if res.Capped {
		t.Errorf("Capped = true, want false (under the page cap)")
	}
	if !res.Written {
		t.Errorf("Written = false, want true")
	}

	// The counter must be persisted to metadata.quotas.events_used.
	tenant, err := repo.FindByID("t-backfill")
	if err != nil {
		t.Fatalf("find tenant: %v", err)
	}
	if q := extractQuotas(tenant.Metadata); q.EventsUsed != 870_000 {
		t.Errorf("persisted events_used = %d, want 870000", q.EventsUsed)
	}
}

func TestBackfillEventsUsed_DryRunDoesNotWrite(t *testing.T) {
	uc, repo := newBackfillFixture(t, 500)

	res, err := uc.Execute(context.Background(), BackfillRequest{TenantID: "t-backfill", DryRun: true})
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}
	if res.EventsFound != 500 {
		t.Errorf("EventsFound = %d, want 500", res.EventsFound)
	}
	if res.Written {
		t.Errorf("Written = true on dry run, want false")
	}

	tenant, _ := repo.FindByID("t-backfill") //nolint:errcheck
	if q := extractQuotas(tenant.Metadata); q.EventsUsed != 0 {
		t.Errorf("dry run persisted events_used = %d, want 0", q.EventsUsed)
	}
}

func TestBackfillEventsUsed_CapReportsLowerBound(t *testing.T) {
	uc, _ := newBackfillFixture(t, 10_000)

	// maxPages=2 at queryPageLimit(500) scans only 1000 events of the 10000.
	res, err := uc.Execute(context.Background(), BackfillRequest{TenantID: "t-backfill", MaxPages: 2})
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}
	if !res.Capped {
		t.Errorf("Capped = false, want true (hit the page cap)")
	}
	if res.EventsFound != 1_000 {
		t.Errorf("EventsFound = %d, want 1000 (lower bound at the cap)", res.EventsFound)
	}
}
