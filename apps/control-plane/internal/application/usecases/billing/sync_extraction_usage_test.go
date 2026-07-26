package billing

import (
	"context"
	"testing"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// extractionMockCore returns a fixed page of prime.extraction.usage events on
// the first query and an empty page after, so paging terminates.
type extractionMockCore struct {
	clients.CoreClient
	events []clients.EventEntry
}

func (m *extractionMockCore) QueryEvents(_ context.Context, req clients.QueryEventsRequest) (*clients.QueryEventsResponse, error) {
	if req.Offset > 0 {
		return &clients.QueryEventsResponse{}, nil
	}
	return &clients.QueryEventsResponse{Events: m.events, Count: len(m.events)}, nil
}

func usageEvent(tokens float64) clients.EventEntry {
	return clients.EventEntry{
		EventType: eventExtractionUsage,
		Payload:   map[string]any{"total_tokens": tokens},
	}
}

func TestSyncExtractionUsage_SumsTokensPayload(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	audit := persistence.NewMemoryAuditRepository()
	seedTenant(t, repo, "t1", 0) // starts with extraction_tokens_used = 0
	core := &extractionMockCore{events: []clients.EventEntry{usageEvent(150), usageEvent(250)}}
	uc := NewSyncExtractionUsageUseCase(repo, audit, core)

	res, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if res.Skipped || res.ExtractionTokensUsed != 400 {
		t.Fatalf("expected 400 tokens summed, got %+v", res)
	}
	tn, err := repo.FindByID("t1")
	if err != nil {
		t.Fatalf("repo.FindByID: %v", err)
	}
	if got := extractQuotas(tn.Metadata).ExtractionTokensUsed; got != 400 {
		t.Fatalf("persisted extraction_tokens_used = %d, want 400", got)
	}
}

func TestSyncExtractionUsage_SkipsWhenAligned(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	audit := persistence.NewMemoryAuditRepository()
	seedTenant(t, repo, "t1", 0)
	// Pre-set the meter to the value the events will sum to → no-op.
	tn, err := repo.FindByID("t1")
	if err != nil {
		t.Fatalf("repo.FindByID: %v", err)
	}
	q := extractQuotas(tn.Metadata)
	q.ExtractionTokensUsed = 400
	tn.Metadata["quotas"] = &q
	_ = repo.Update(tn) //nolint:errcheck // test setup

	core := &extractionMockCore{events: []clients.EventEntry{usageEvent(400)}}
	uc := NewSyncExtractionUsageUseCase(repo, audit, core)
	res, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if !res.Skipped {
		t.Fatalf("expected skip when meter already matches, got %+v", res)
	}
}

func TestSyncExtractionUsage_NilCoreSkips(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	seedTenant(t, repo, "t1", 0)
	uc := NewSyncExtractionUsageUseCase(repo, persistence.NewMemoryAuditRepository(), nil)
	res, err := uc.Execute(context.Background(), "t1")
	if err != nil {
		t.Fatalf("uc.Execute: %v", err)
	}
	if !res.Skipped {
		t.Fatalf("nil core client must skip, got %+v", res)
	}
}
