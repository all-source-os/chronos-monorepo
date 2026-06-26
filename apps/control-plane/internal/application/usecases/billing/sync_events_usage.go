package billing

import (
	"context"
	"fmt"
	"log"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// SyncEventsUsageUseCase reconciles a tenant's quotas.events_used against the
// REAL count of the tenant's events in Core. events_used is a forward counter
// incremented by the Query Service usage reporter; it drifts both ways —
// out-of-band ingestion (Prime sync, direct ingest, MCP) under-counts, and a
// capped/one-shot backfill over-counts (one tenant sat at exactly 1,000,000,
// the backfill's page cap). Both the usage bar AND usage enforcement read this
// value, so drift over-bills or wrongly throttles.
//
// This is the RECURRING, self-healing companion to BackfillEventsUsedUseCase
// (the on-demand admin tool): it writes the same all-time real count, but cheaply
// via Core's total_count (one limit:1 query, no paging) and with no page cap, so
// it also corrects a previously-capped backfill value. Mirrors
// SyncX402UsageUseCase and runs on the scheduler.
type SyncEventsUsageUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
}

// NewSyncEventsUsageUseCase creates a new SyncEventsUsageUseCase.
func NewSyncEventsUsageUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *SyncEventsUsageUseCase {
	return &SyncEventsUsageUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// EventsUsageResult holds the outcome of reconciling one tenant.
type EventsUsageResult struct {
	TenantID   string
	EventsUsed int64
	Skipped    bool
	Error      error
}

// Execute reconciles events_used for a single tenant to the tenant's real
// all-time event count (matching BackfillEventsUsedUseCase's semantics). Core
// returns the count as total_count from a single tenant-scoped query (limit:1),
// so there is no paging over the whole stream.
func (uc *SyncEventsUsageUseCase) Execute(ctx context.Context, tenantID string) (*EventsUsageResult, error) {
	if uc.coreClient == nil {
		return &EventsUsageResult{TenantID: tenantID, Skipped: true}, nil
	}

	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}

	quotas := extractQuotas(tenant.Metadata)

	resp, err := uc.coreClient.QueryEvents(ctx, clients.QueryEventsRequest{
		TenantID: tenantID,
		Limit:    1,
	})
	if err != nil {
		return nil, fmt.Errorf("count events for %s: %w", tenantID, err)
	}
	real := int64(resp.TotalCount)

	if real == quotas.EventsUsed {
		return &EventsUsageResult{TenantID: tenantID, EventsUsed: real, Skipped: true}, nil
	}

	quotas.EventsUsed = real
	if tenant.Metadata == nil {
		tenant.Metadata = make(map[string]interface{})
	}
	tenant.Metadata["quotas"] = &quotas

	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, fmt.Errorf("persist events_used for %s: %w", tenantID, err)
	}

	if uc.auditRepo != nil {
		auditEvent, _ := entities.NewAuditEvent("billing.events.synced", "report", "SCHEDULER", "/billing/events") //nolint:errcheck
		auditEvent.WithResource("tenant", tenantID).WithTenant(tenantID)
		auditEvent.AddMetadata("events_used", fmt.Sprintf("%d", real))
		_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck
	}

	return &EventsUsageResult{TenantID: tenantID, EventsUsed: real}, nil
}

// ExecuteAll reconciles events_used for all active tenants.
func (uc *SyncEventsUsageUseCase) ExecuteAll(ctx context.Context) []EventsUsageResult {
	tenants, err := uc.tenantRepo.FindActive()
	if err != nil {
		log.Printf("SyncEventsUsage: failed to list tenants: %v", err)
		return nil
	}

	results := make([]EventsUsageResult, 0, len(tenants))
	for _, t := range tenants {
		result, err := uc.Execute(ctx, t.ID)
		if err != nil {
			results = append(results, EventsUsageResult{TenantID: t.ID, Error: err})
			continue
		}
		results = append(results, *result)
	}
	return results
}
