package billing

import (
	"context"
	"fmt"
	"log"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// ReportUsageUseCase reports overage to LemonSqueezy via metered billing API.
// It tracks reported overage in tenant metadata to prevent double-reporting.
type ReportUsageUseCase struct {
	tenantRepo  repositories.TenantRepository
	auditRepo   repositories.AuditRepository
	lsClient    clients.LemonSqueezyClient
	calculateUC *CalculateOverageUseCase
}

// NewReportUsageUseCase creates a new ReportUsageUseCase.
func NewReportUsageUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	lsClient clients.LemonSqueezyClient,
	calculateUC *CalculateOverageUseCase,
) *ReportUsageUseCase {
	return &ReportUsageUseCase{
		tenantRepo:  tenantRepo,
		auditRepo:   auditRepo,
		lsClient:    lsClient,
		calculateUC: calculateUC,
	}
}

// ReportResult holds the result of a single tenant's overage report.
type ReportResult struct {
	TenantID       string
	EventsReported int64
	QueriesReported int64
	Skipped        bool
	Error          error
}

// Execute reports overage for a single tenant, computing the delta since the last report.
func (uc *ReportUsageUseCase) Execute(ctx context.Context, tenantID string) (*ReportResult, error) {
	result, err := uc.calculateUC.Execute(tenantID)
	if err != nil {
		return nil, fmt.Errorf("calculate overage for %s: %w", tenantID, err)
	}

	if !result.Enabled || (result.EventsOverage == 0 && result.QueriesOverage == 0) {
		return &ReportResult{TenantID: tenantID, Skipped: true}, nil
	}

	// Load tenant to get subscription item ID and last reported values
	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}

	sub := extractSubscription(tenant.Metadata)
	if sub.SubscriptionItemID == "" {
		return &ReportResult{TenantID: tenantID, Skipped: true}, nil
	}

	overage := extractOverage(tenant.Metadata)

	// Compute delta since last report to prevent double-reporting
	eventsDelta := result.EventsOverage - overage.LastReportedEvents
	queriesDelta := result.QueriesOverage - overage.LastReportedQueries

	if eventsDelta <= 0 && queriesDelta <= 0 {
		return &ReportResult{TenantID: tenantID, Skipped: true}, nil
	}

	// Report events overage to LemonSqueezy
	if eventsDelta > 0 {
		err := uc.lsClient.ReportUsage(ctx, clients.ReportUsageRequest{
			SubscriptionItemID: sub.SubscriptionItemID,
			Quantity:           int(eventsDelta),
			Action:             "increment",
		})
		if err != nil {
			return nil, fmt.Errorf("report events overage for %s: %w", tenantID, err)
		}
	}

	// Report queries overage (using same subscription item — LemonSqueezy aggregates)
	if queriesDelta > 0 {
		err := uc.lsClient.ReportUsage(ctx, clients.ReportUsageRequest{
			SubscriptionItemID: sub.SubscriptionItemID,
			Quantity:           int(queriesDelta),
			Action:             "increment",
		})
		if err != nil {
			return nil, fmt.Errorf("report queries overage for %s: %w", tenantID, err)
		}
	}

	// Update last reported values in tenant metadata
	overage.LastReportedEvents = result.EventsOverage
	overage.LastReportedQueries = result.QueriesOverage
	overage.EventsOverage = result.EventsOverage
	overage.QueriesOverage = result.QueriesOverage

	if tenant.Metadata == nil {
		tenant.Metadata = make(map[string]interface{})
	}
	tenant.Metadata["overage"] = &overage

	if err := uc.tenantRepo.Update(tenant); err != nil {
		log.Printf("ReportUsage: failed to update last reported for %s: %v", tenantID, err)
	}

	// Audit
	auditEvent, _ := entities.NewAuditEvent("billing.overage.reported", "report", "SCHEDULER", "/billing/overage") //nolint:errcheck
	auditEvent.WithResource("tenant", tenantID).WithTenant(tenantID)
	auditEvent.AddMetadata("events_delta", fmt.Sprintf("%d", eventsDelta))
	auditEvent.AddMetadata("queries_delta", fmt.Sprintf("%d", queriesDelta))
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &ReportResult{
		TenantID:        tenantID,
		EventsReported:  eventsDelta,
		QueriesReported: queriesDelta,
	}, nil
}

// ExecuteAll reports overage for all active tenants with overage.
func (uc *ReportUsageUseCase) ExecuteAll(ctx context.Context) []ReportResult {
	overages, err := uc.calculateUC.ExecuteAll()
	if err != nil {
		log.Printf("ReportUsage: failed to calculate overages: %v", err)
		return nil
	}

	var results []ReportResult
	for _, o := range overages {
		result, err := uc.Execute(ctx, o.TenantID)
		if err != nil {
			results = append(results, ReportResult{TenantID: o.TenantID, Error: err})
			continue
		}
		results = append(results, *result)
	}
	return results
}

// --- Metadata extraction helper for subscription ---

func extractSubscription(metadata map[string]interface{}) entities.SubscriptionMetadata {
	if metadata == nil {
		return entities.SubscriptionMetadata{}
	}
	raw, ok := metadata["subscription"]
	if !ok {
		return entities.SubscriptionMetadata{}
	}
	switch v := raw.(type) {
	case *entities.SubscriptionMetadata:
		return *v
	case entities.SubscriptionMetadata:
		return v
	case map[string]interface{}:
		return subscriptionFromMap(v)
	default:
		return entities.SubscriptionMetadata{}
	}
}

func subscriptionFromMap(m map[string]interface{}) entities.SubscriptionMetadata {
	s := entities.SubscriptionMetadata{}
	if v, ok := m["subscription_id"].(string); ok {
		s.SubscriptionID = v
	}
	if v, ok := m["subscription_item_id"].(string); ok {
		s.SubscriptionItemID = v
	}
	if v, ok := m["customer_id"].(string); ok {
		s.CustomerID = v
	}
	if v, ok := m["status"].(string); ok {
		s.Status = v
	}
	if v, ok := m["tier"].(string); ok {
		s.Tier = v
	}
	return s
}
