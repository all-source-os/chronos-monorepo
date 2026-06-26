package usecases

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// GetTenantUsageUseCase retrieves usage breakdown for a tenant by aggregating
// metrics from Core via the Query Service.
type GetTenantUsageUseCase struct {
	tenantRepo repositories.TenantRepository
	coreClient clients.CoreClient
}

// NewGetTenantUsageUseCase creates a new GetTenantUsageUseCase.
func NewGetTenantUsageUseCase(
	tenantRepo repositories.TenantRepository,
	coreClient clients.CoreClient,
) *GetTenantUsageUseCase {
	return &GetTenantUsageUseCase{
		tenantRepo: tenantRepo,
		coreClient: coreClient,
	}
}

// Execute retrieves usage data for the given tenant.
// It validates the tenant exists, then fetches stats from Core.
func (uc *GetTenantUsageUseCase) Execute(ctx context.Context, id string) (*dto.TenantUsageResponse, error) {
	// Verify tenant exists (and read its quota limits for the usage bars).
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	// Fetch stats from Core
	var eventCount, storageBytes, streamCount int64
	if uc.coreClient != nil {
		stats, err := uc.coreClient.GetTenantStats(ctx, id)
		if err == nil {
			eventCount = stats.EventCount
			storageBytes = stats.StorageUsed
			streamCount = stats.StreamCount
		}
		// If Core is unavailable, return zeros rather than failing
	}

	// Quota limits live in metadata.quotas under events_quota / queries_quota
	// (NOT event_limit/query_limit — that mismatch left the admin usage bars with
	// a zero denominator). Surface them so the bars render real percentages.
	eventLimit, queryLimit, storageLimitMB := extractQuotaLimits(tenant)

	// Build per-day usage breakdown for the last 7 days.
	// Real per-day metrics would come from Core's metrics aggregation;
	// for now, we return the current totals on today and zeros for prior days.
	dailyUsage := buildDailyUsage(eventCount, storageBytes)

	return &dto.TenantUsageResponse{
		TenantID:       id,
		EventCount:     eventCount,
		QueryCount:     0, // Query count tracking is a future metric
		StorageBytes:   storageBytes,
		StreamCount:    streamCount,
		EventLimit:     eventLimit,
		QueryLimit:     queryLimit,
		StorageLimitMB: storageLimitMB,
		DailyUsage:     dailyUsage,
	}, nil
}

// extractQuotaLimits reads the tenant's quota ceilings from metadata.quotas.
// The canonical keys Core persists are `events_quota` / `queries_quota`; a few
// older writers also used `event_limit` / `query_limit`, so we accept either.
// A missing storage ceiling stays 0 (storage metering is a future metric).
func extractQuotaLimits(t *entities.Tenant) (eventLimit, queryLimit, storageLimitMB int64) {
	if t == nil || t.Metadata == nil {
		return 0, 0, 0
	}
	q, ok := t.Metadata["quotas"].(map[string]interface{})
	if !ok {
		return 0, 0, 0
	}
	pick := func(keys ...string) int64 {
		for _, k := range keys {
			if v, ok := q[k]; ok {
				if n := toInt64Val(v); n != 0 {
					return n
				}
			}
		}
		return 0
	}
	eventLimit = pick("events_quota", "event_limit")
	queryLimit = pick("queries_quota", "query_limit")
	storageLimitMB = pick("storage_limit_mb", "storage_quota_mb")
	return eventLimit, queryLimit, storageLimitMB
}

// buildDailyUsage constructs a 7-day usage breakdown.
// Today gets the current totals; prior days are zeros until Core exposes
// per-day metrics via its metrics aggregation endpoint.
func buildDailyUsage(eventCount, storageBytes int64) []dto.DailyUsage {
	now := time.Now().UTC()
	days := make([]dto.DailyUsage, 7)

	for i := 6; i >= 0; i-- {
		day := now.AddDate(0, 0, -i)
		du := dto.DailyUsage{
			Date: day.Format("2006-01-02"),
		}
		if i == 0 {
			// Current day gets the running totals
			du.EventsIngested = eventCount
			du.StorageBytes = storageBytes
		}
		days[6-i] = du
	}

	return days
}
