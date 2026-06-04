package billing

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// x402 event types reconciled into the x402_used counter. Kept as local
// constants (rather than importing the x402 package) so the billing package
// does not depend on the infrastructure/x402 package — the dependency direction
// is application → domain/clients, not application → infrastructure/x402.
const (
	eventAllowanceConsumed = "x402.allowance.consumed"
	eventPaymentSettled    = "x402.payment.settled"
)

// queryPageLimit bounds a single Core events page. The reconciliation pages
// through results until a short page is returned.
const queryPageLimit = 500

// SyncX402UsageUseCase reconciles a tenant's x402_used counter from the x402
// events recorded in Core (allowance-consumed + settled payments) since the
// start of the current billing period. This is the metering half of the 011
// x402 allowance: the quota gate enforces against x402_used, and this use case
// keeps that number truthful from the durable event log in Core (per CLAUDE.md,
// usage metering that is event-shaped routes through Core, not an in-memory
// counter that dies on restart).
//
// It is designed to run on the same scheduler tick as overage reporting.
type SyncX402UsageUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
}

// NewSyncX402UsageUseCase creates a new SyncX402UsageUseCase.
func NewSyncX402UsageUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *SyncX402UsageUseCase {
	return &SyncX402UsageUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// SyncResult holds the outcome of reconciling one tenant.
type SyncResult struct {
	TenantID string
	X402Used int64
	Skipped  bool
	Error    error
}

// periodStart returns the RFC3339 timestamp for the start of the tenant's
// current x402 billing period. We use the quotas.reset_date if present;
// otherwise the first of the current month (matching the dashboard copy
// "Resets on the 1st of each month").
func periodStart(quotas entities.QuotaMetadata, now time.Time) string {
	if quotas.ResetDate != "" {
		if t, err := time.Parse(time.RFC3339, quotas.ResetDate); err == nil {
			// reset_date is the NEXT reset; the current period started a month before.
			return t.AddDate(0, -1, 0).Format(time.RFC3339)
		}
		// Date-only form (YYYY-MM-DD).
		if t, err := time.Parse("2006-01-02", quotas.ResetDate); err == nil {
			return t.AddDate(0, -1, 0).Format(time.RFC3339)
		}
	}
	monthStart := time.Date(now.Year(), now.Month(), 1, 0, 0, 0, 0, now.Location())
	return monthStart.Format(time.RFC3339)
}

// Execute reconciles x402_used for a single tenant.
func (uc *SyncX402UsageUseCase) Execute(ctx context.Context, tenantID string) (*SyncResult, error) {
	if uc.coreClient == nil {
		return &SyncResult{TenantID: tenantID, Skipped: true}, nil
	}

	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}

	quotas := extractQuotas(tenant.Metadata)
	// Only tenants with an x402 allowance (or unlimited) need a meter. -1 is
	// unlimited and never depletes, so we skip reconciliation for it.
	if quotas.X402Allowance == 0 {
		return &SyncResult{TenantID: tenantID, Skipped: true}, nil
	}

	since := periodStart(quotas, time.Now().UTC())

	consumed, err := uc.countEvents(ctx, tenantID, eventAllowanceConsumed, since)
	if err != nil {
		return nil, fmt.Errorf("count allowance events for %s: %w", tenantID, err)
	}
	settled, err := uc.countEvents(ctx, tenantID, eventPaymentSettled, since)
	if err != nil {
		return nil, fmt.Errorf("count settled events for %s: %w", tenantID, err)
	}
	total := consumed + settled

	if total == quotas.X402Used {
		return &SyncResult{TenantID: tenantID, X402Used: total, Skipped: true}, nil
	}

	quotas.X402Used = total
	if tenant.Metadata == nil {
		tenant.Metadata = make(map[string]interface{})
	}
	tenant.Metadata["quotas"] = &quotas

	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, fmt.Errorf("persist x402_used for %s: %w", tenantID, err)
	}

	if uc.auditRepo != nil {
		auditEvent, _ := entities.NewAuditEvent("billing.x402.synced", "report", "SCHEDULER", "/billing/x402") //nolint:errcheck
		auditEvent.WithResource("tenant", tenantID).WithTenant(tenantID)
		auditEvent.AddMetadata("x402_used", fmt.Sprintf("%d", total))
		_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck
	}

	return &SyncResult{TenantID: tenantID, X402Used: total}, nil
}

// countEvents counts events of the given type for the tenant since the period
// start, paging through Core's event query.
func (uc *SyncX402UsageUseCase) countEvents(ctx context.Context, tenantID, eventType, since string) (int64, error) {
	var total int64
	offset := 0
	for {
		resp, err := uc.coreClient.QueryEvents(ctx, clients.QueryEventsRequest{
			EntityID:  tenantID,
			EventType: eventType,
			TenantID:  tenantID,
			Since:     since,
			Limit:     queryPageLimit,
			Offset:    offset,
		})
		if err != nil {
			return 0, err
		}
		total += int64(len(resp.Events))
		if len(resp.Events) < queryPageLimit {
			break
		}
		offset += queryPageLimit
	}
	return total, nil
}

// ExecuteAll reconciles x402_used for all active tenants that have an allowance.
func (uc *SyncX402UsageUseCase) ExecuteAll(ctx context.Context) []SyncResult {
	tenants, err := uc.tenantRepo.FindActive()
	if err != nil {
		log.Printf("SyncX402Usage: failed to list tenants: %v", err)
		return nil
	}

	var results []SyncResult
	for _, t := range tenants {
		result, err := uc.Execute(ctx, t.ID)
		if err != nil {
			results = append(results, SyncResult{TenantID: t.ID, Error: err})
			continue
		}
		results = append(results, *result)
	}
	return results
}
