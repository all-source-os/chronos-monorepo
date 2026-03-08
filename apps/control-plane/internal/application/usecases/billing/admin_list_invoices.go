package billing

import (
	"context"
	"fmt"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// AdminListInvoicesUseCase lists invoices from LemonSqueezy for admin review.
type AdminListInvoicesUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
	lsClient   clients.LemonSqueezyClient
}

// NewAdminListInvoicesUseCase creates a new AdminListInvoicesUseCase.
func NewAdminListInvoicesUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	lsClient clients.LemonSqueezyClient,
) *AdminListInvoicesUseCase {
	return &AdminListInvoicesUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		lsClient:   lsClient,
	}
}

// Execute retrieves invoices from LemonSqueezy, optionally filtered by tenant and status.
func (uc *AdminListInvoicesUseCase) Execute(ctx context.Context, req dto.AdminInvoiceListRequest, adminUserID string) (*dto.AdminInvoiceListResponse, error) {
	page := req.Page
	if page < 1 {
		page = 1
	}

	// If tenant_id is provided, we could filter by customer_id in the future.
	// For now, we fetch all invoices and let the admin filter client-side.
	storeID := uc.lsClient.GetStoreID()

	invoiceResp, err := uc.lsClient.ListInvoices(ctx, storeID, page, req.Status)
	if err != nil {
		return nil, fmt.Errorf("list invoices from LemonSqueezy: %w", err)
	}

	// Map customer IDs to tenant IDs for enrichment
	tenantMap := uc.buildCustomerTenantMap()

	var items []dto.AdminInvoiceItem
	for _, inv := range invoiceResp.Invoices {
		item := dto.AdminInvoiceItem{
			ID:             inv.ID,
			CustomerID:     inv.CustomerID,
			SubscriptionID: inv.SubscriptionID,
			Status:         inv.Status,
			TotalFormatted: inv.TotalFormatted,
			Total:          inv.Total,
			Currency:       inv.Currency,
			CreatedAt:      inv.CreatedAt,
			RefundedAt:     inv.RefundedAt,
		}

		// Enrich with tenant ID if we can map the customer
		if tenantID, ok := tenantMap[fmt.Sprintf("%d", inv.CustomerID)]; ok {
			item.TenantID = tenantID
		}

		// Filter by tenant_id if specified
		if req.TenantID != "" && item.TenantID != req.TenantID {
			continue
		}

		items = append(items, item)
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("admin.billing.invoices.list", "list", "GET", "/api/v1/admin/billing/invoices") //nolint:errcheck
	auditEvent.WithUser(adminUserID, "").WithResource("invoices", "")
	auditEvent.AddMetadata("page", page)
	auditEvent.AddMetadata("status_filter", req.Status)
	auditEvent.AddMetadata("tenant_filter", req.TenantID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &dto.AdminInvoiceListResponse{
		Invoices:   items,
		Total:      invoiceResp.Total,
		Page:       invoiceResp.Page,
		TotalPages: invoiceResp.TotalPages,
	}, nil
}

// buildCustomerTenantMap creates a map of LemonSqueezy customer ID -> tenant ID.
func (uc *AdminListInvoicesUseCase) buildCustomerTenantMap() map[string]string {
	result := make(map[string]string)
	tenants, err := uc.tenantRepo.FindAll()
	if err != nil {
		return result
	}

	for _, t := range tenants {
		sub := extractSubscription(t.Metadata)
		if sub.CustomerID != "" {
			result[sub.CustomerID] = t.ID
		}
	}
	return result
}
