package billing

import (
	"context"
	"fmt"
	"log"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// defaultBackfillMaxPages bounds how many event pages a single backfill will read
// from Core before it stops and reports a capped (lower-bound) count. At
// queryPageLimit (500) per page this defaults to 2,000 pages = up to 1,000,000
// events scanned. A tenant with more events than this gets an honest "at least N"
// result (Capped=true) rather than an under-count silently presented as a total.
const defaultBackfillMaxPages = 2000

// BackfillEventsUsedUseCase reconciles a tenant's events_used counter from the
// durable event log in Core. The metering path (QS UsageReporter ->
// POST /api/v1/tenants/{id}/usage/increment) only counts events written THROUGH
// the Query Service; events ingested out-of-band (Prime sync, direct Control-Plane
// ingest, MCP) never moved the counter, so a tenant whose store is full can still
// read events_used = 0. This use case counts the tenant's real events in Core and
// writes that count into metadata.quotas.events_used — the field the dashboard
// (QS /api/tenant + /api/tenant/usage) reads.
//
// Per CLAUDE.md: events live in Core (never Postgres); the usage counter lives in
// the tenant's Core metadata blob. This reads events from Core and writes only the
// counter — it never moves or mutates event data, and never reads events from
// Postgres. Mirrors SyncX402UsageUseCase's read-count-then-write shape.
type BackfillEventsUsedUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
}

// NewBackfillEventsUsedUseCase constructs the use case.
func NewBackfillEventsUsedUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *BackfillEventsUsedUseCase {
	return &BackfillEventsUsedUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// BackfillRequest parameterizes a backfill run.
type BackfillRequest struct {
	// TenantID to backfill. Empty = all active tenants (ExecuteAll).
	TenantID string `json:"tenant_id,omitempty"`
	// MaxPages caps how many Core pages are scanned per tenant. 0 = default.
	MaxPages int `json:"max_pages,omitempty"`
	// DryRun counts and reports without writing the counter.
	DryRun bool `json:"dry_run,omitempty"`
}

// BackfillResult holds the outcome for one tenant.
type BackfillResult struct {
	TenantID    string `json:"tenant_id"`
	EventsFound int64  `json:"events_found"`
	Previous    int64  `json:"previous_events_used"`
	Written     bool   `json:"written"`
	Capped      bool   `json:"capped"` // true if MaxPages was hit (EventsFound is a lower bound)
	Skipped     bool   `json:"skipped,omitempty"`
	Error       string `json:"error,omitempty"`
}

// Execute backfills events_used for a single tenant.
func (uc *BackfillEventsUsedUseCase) Execute(ctx context.Context, req BackfillRequest) (*BackfillResult, error) {
	if uc.coreClient == nil {
		return &BackfillResult{TenantID: req.TenantID, Skipped: true}, nil
	}

	tenant, err := uc.tenantRepo.FindByID(req.TenantID)
	if err != nil {
		return nil, err
	}

	maxPages := req.MaxPages
	if maxPages <= 0 {
		maxPages = defaultBackfillMaxPages
	}

	count, capped, err := uc.countTenantEvents(ctx, req.TenantID, maxPages)
	if err != nil {
		return nil, fmt.Errorf("count events for %s: %w", req.TenantID, err)
	}

	quotas := extractQuotas(tenant.Metadata)
	result := &BackfillResult{
		TenantID:    req.TenantID,
		EventsFound: count,
		Previous:    quotas.EventsUsed,
		Capped:      capped,
	}

	if req.DryRun {
		return result, nil
	}

	// Idempotent: nothing to write if the counter already matches.
	if count == quotas.EventsUsed {
		return result, nil
	}

	quotas.EventsUsed = count
	if tenant.Metadata == nil {
		tenant.Metadata = make(map[string]interface{})
	}
	tenant.Metadata["quotas"] = &quotas

	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, fmt.Errorf("persist events_used for %s: %w", req.TenantID, err)
	}
	result.Written = true

	if uc.auditRepo != nil {
		auditEvent, _ := entities.NewAuditEvent("billing.events_used.backfilled", "update", "POST", "/billing/backfill-usage") //nolint:errcheck
		auditEvent.WithResource("tenant", req.TenantID).WithTenant(req.TenantID)
		auditEvent.AddMetadata("events_used", fmt.Sprintf("%d", count))
		auditEvent.AddMetadata("previous", fmt.Sprintf("%d", result.Previous))
		auditEvent.AddMetadata("capped", fmt.Sprintf("%t", capped))
		_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck
	}

	return result, nil
}

// countTenantEvents pages through ALL of the tenant's events in Core (no event-type
// filter, no since — a lifetime count) up to maxPages. Returns the count and
// whether the page cap was reached (count is then a lower bound).
func (uc *BackfillEventsUsedUseCase) countTenantEvents(ctx context.Context, tenantID string, maxPages int) (total int64, capped bool, err error) {
	offset := 0
	for page := 0; page < maxPages; page++ {
		resp, err := uc.coreClient.QueryEvents(ctx, clients.QueryEventsRequest{
			// Tenant-scoped, all entities/types. TenantID is required post auth-skip
			// cutover so Core scopes the query to this tenant's partition.
			TenantID: tenantID,
			Limit:    queryPageLimit,
			Offset:   offset,
		})
		if err != nil {
			return total, false, err
		}
		total += int64(len(resp.Events))
		if len(resp.Events) < queryPageLimit {
			return total, false, nil
		}
		offset += queryPageLimit
	}
	// Hit the page cap with a full last page — there may be more.
	return total, true, nil
}

// ExecuteAll backfills events_used for every active tenant.
func (uc *BackfillEventsUsedUseCase) ExecuteAll(ctx context.Context, req BackfillRequest) []BackfillResult {
	tenants, err := uc.tenantRepo.FindActive()
	if err != nil {
		log.Printf("BackfillEventsUsed: failed to list tenants: %v", err)
		return nil
	}

	var results []BackfillResult
	for _, t := range tenants {
		r, err := uc.Execute(ctx, BackfillRequest{TenantID: t.ID, MaxPages: req.MaxPages, DryRun: req.DryRun})
		if err != nil {
			results = append(results, BackfillResult{TenantID: t.ID, Error: err.Error()})
			continue
		}
		results = append(results, *r)
	}
	return results
}
