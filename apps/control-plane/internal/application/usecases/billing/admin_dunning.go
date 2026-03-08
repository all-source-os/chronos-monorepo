package billing

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// AdminDunningUseCase lists tenants with failed payments and retry status.
type AdminDunningUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewAdminDunningUseCase creates a new AdminDunningUseCase.
func NewAdminDunningUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *AdminDunningUseCase {
	return &AdminDunningUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute retrieves tenants with failed or past-due payment status.
func (uc *AdminDunningUseCase) Execute(adminUserID string) (*dto.AdminDunningResponse, error) {
	tenants, err := uc.tenantRepo.FindAll()
	if err != nil {
		return nil, err
	}

	var items []dto.AdminDunningItem
	for _, t := range tenants {
		sub := extractSubscriptionEntity(t.Metadata)

		// Include tenants with problematic subscription statuses
		switch sub.Status {
		case statusPastDue, "unpaid", "expired":
			// Determine retry status based on subscription state
			retryStatus := classifyRetryStatus(sub)

			items = append(items, dto.AdminDunningItem{
				TenantID:       t.ID,
				TenantName:     t.Name,
				SubscriptionID: sub.SubscriptionID,
				Status:         sub.Status,
				Tier:           sub.Tier,
				CustomerID:     sub.CustomerID,
				RetryStatus:    retryStatus,
			})
		}
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("admin.billing.dunning.list", "list", "GET", "/api/v1/admin/billing/dunning") //nolint:errcheck
	auditEvent.WithUser(adminUserID, "").WithResource("dunning", "")
	auditEvent.AddMetadata("count", len(items))
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &dto.AdminDunningResponse{
		Items:      items,
		TotalCount: len(items),
	}, nil
}

// classifyRetryStatus determines the retry status based on subscription metadata.
func classifyRetryStatus(sub entities.SubscriptionMetadata) string {
	switch sub.Status {
	case statusPastDue:
		// LemonSqueezy automatically retries failed payments
		return "pending_retry"
	case "unpaid":
		// Payment retries exhausted
		return "exhausted"
	case "expired":
		// Subscription expired, needs manual intervention
		return "manual_review"
	default:
		return "unknown"
	}
}
