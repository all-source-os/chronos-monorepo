package billing

import (
	"context"
	"fmt"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// AdminRefundUseCase processes refunds via LemonSqueezy.
type AdminRefundUseCase struct {
	auditRepo repositories.AuditRepository
	lsClient  clients.LemonSqueezyClient
}

// NewAdminRefundUseCase creates a new AdminRefundUseCase.
func NewAdminRefundUseCase(
	auditRepo repositories.AuditRepository,
	lsClient clients.LemonSqueezyClient,
) *AdminRefundUseCase {
	return &AdminRefundUseCase{
		auditRepo: auditRepo,
		lsClient:  lsClient,
	}
}

// Execute processes a refund for the given order.
func (uc *AdminRefundUseCase) Execute(ctx context.Context, req dto.AdminRefundRequest, adminUserID string) (*dto.AdminRefundResponse, error) {
	if req.OrderID == "" {
		return nil, fmt.Errorf("order_id is required")
	}
	if req.Amount <= 0 {
		return nil, fmt.Errorf("amount must be positive")
	}

	err := uc.lsClient.RefundInvoice(ctx, req.OrderID, req.Amount)
	if err != nil {
		// Audit the failed refund attempt
		auditEvent, _ := entities.NewAuditEvent("admin.billing.refund.failed", "refund", "POST", "/api/v1/admin/billing/refund") //nolint:errcheck
		auditEvent.WithUser(adminUserID, "").WithResource("order", req.OrderID)
		auditEvent.AddMetadata("amount", req.Amount)
		auditEvent.AddMetadata("error", err.Error())
		_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

		return nil, fmt.Errorf("process refund via LemonSqueezy: %w", err)
	}

	// Audit the successful refund
	auditEvent, _ := entities.NewAuditEvent("admin.billing.refund.processed", "refund", "POST", "/api/v1/admin/billing/refund") //nolint:errcheck
	auditEvent.WithUser(adminUserID, "").WithResource("order", req.OrderID)
	auditEvent.AddMetadata("amount", req.Amount)
	auditEvent.AddMetadata("reason", req.Reason)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &dto.AdminRefundResponse{
		OrderID: req.OrderID,
		Amount:  req.Amount,
		Status:  "refunded",
	}, nil
}
