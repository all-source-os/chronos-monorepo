package http //nolint:revive // package name intentionally matches directory

import (
	"fmt"
	"net/http"
	"strconv"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases/billing"
)

// defaultRevenueRange is the default time range for revenue queries.
const defaultRevenueRange = "30d"

// --- HAL response wrappers ---

type adminInvoiceListHALResponse struct {
	HALResource
	*dto.AdminInvoiceListResponse
}

type adminRevenueHALResponse struct {
	HALResource
	*dto.AdminRevenueResponse
}

type adminRefundHALResponse struct {
	HALResource
	*dto.AdminRefundResponse
}

type adminDunningHALResponse struct {
	HALResource
	*dto.AdminDunningResponse
}

// --- HAL link helpers ---

func adminBillingInvoiceLinks(page int) map[string]Link {
	return map[string]Link{
		"self":    SelfLink(fmt.Sprintf("/api/v1/admin/billing/invoices?page=%d", page)),
		"revenue": NewLink("/api/v1/admin/billing/revenue", WithTitle("Revenue Metrics")),
		"dunning": NewLink("/api/v1/admin/billing/dunning", WithTitle("Dunning Status")),
	}
}

func adminBillingRevenueLinks(rangeParam string) map[string]Link {
	return map[string]Link{
		"self":     SelfLink(fmt.Sprintf("/api/v1/admin/billing/revenue?range=%s", rangeParam)),
		"invoices": NewLink("/api/v1/admin/billing/invoices", WithTitle("Invoices")),
		"dunning":  NewLink("/api/v1/admin/billing/dunning", WithTitle("Dunning Status")),
	}
}

func adminBillingRefundLinks() map[string]Link {
	return map[string]Link{
		"self":     SelfLink("/api/v1/admin/billing/refund"),
		"invoices": NewLink("/api/v1/admin/billing/invoices", WithTitle("Invoices")),
	}
}

func adminBillingDunningLinks() map[string]Link {
	return map[string]Link{
		"self":     SelfLink("/api/v1/admin/billing/dunning"),
		"invoices": NewLink("/api/v1/admin/billing/invoices", WithTitle("Invoices")),
		"revenue":  NewLink("/api/v1/admin/billing/revenue", WithTitle("Revenue Metrics")),
	}
}

// --- Handler ---

// AdminBillingHandler handles admin billing HTTP requests.
type AdminBillingHandler struct {
	listInvoicesUC *billing.AdminListInvoicesUseCase
	revenueUC      *billing.AdminRevenueUseCase
	refundUC       *billing.AdminRefundUseCase
	dunningUC      *billing.AdminDunningUseCase
}

// NewAdminBillingHandler creates a new AdminBillingHandler.
func NewAdminBillingHandler(
	listInvoicesUC *billing.AdminListInvoicesUseCase,
	revenueUC *billing.AdminRevenueUseCase,
	refundUC *billing.AdminRefundUseCase,
	dunningUC *billing.AdminDunningUseCase,
) *AdminBillingHandler {
	return &AdminBillingHandler{
		listInvoicesUC: listInvoicesUC,
		revenueUC:      revenueUC,
		refundUC:       refundUC,
		dunningUC:      dunningUC,
	}
}

// ListInvoices handles GET /api/v1/admin/billing/invoices
func (h *AdminBillingHandler) ListInvoices(c *gin.Context) {
	page, _ := strconv.Atoi(c.DefaultQuery("page", "1")) //nolint:errcheck // default is valid int
	tenantID := c.Query("tenant_id")
	status := c.Query("status")

	if page < 1 {
		page = 1
	}

	adminUserID := h.extractAdminUserID(c)

	req := dto.AdminInvoiceListRequest{
		TenantID: tenantID,
		Status:   status,
		Page:     page,
	}

	if h.listInvoicesUC == nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "billing provider not configured"})
		return
	}

	result, err := h.listInvoicesUC.Execute(c.Request.Context(), req, adminUserID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, adminInvoiceListHALResponse{
		HALResource:              HALResource{Links: adminBillingInvoiceLinks(page)},
		AdminInvoiceListResponse: result,
	})
}

// GetRevenue handles GET /api/v1/admin/billing/revenue
func (h *AdminBillingHandler) GetRevenue(c *gin.Context) {
	rangeParam := c.DefaultQuery("range", defaultRevenueRange)

	// Validate range parameter
	switch rangeParam {
	case defaultRevenueRange, "90d", "1y":
		// valid
	default:
		c.JSON(http.StatusBadRequest, gin.H{"error": "range must be one of: 30d, 90d, 1y"})
		return
	}

	adminUserID := h.extractAdminUserID(c)

	req := dto.AdminRevenueRequest{
		Range: rangeParam,
	}

	result, err := h.revenueUC.Execute(c.Request.Context(), req, adminUserID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, adminRevenueHALResponse{
		HALResource:          HALResource{Links: adminBillingRevenueLinks(rangeParam)},
		AdminRevenueResponse: result,
	})
}

// ProcessRefund handles POST /api/v1/admin/billing/refund
func (h *AdminBillingHandler) ProcessRefund(c *gin.Context) {
	var req dto.AdminRefundRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	if h.refundUC == nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "billing provider not configured"})
		return
	}

	adminUserID := h.extractAdminUserID(c)

	result, err := h.refundUC.Execute(c.Request.Context(), req, adminUserID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, adminRefundHALResponse{
		HALResource:         HALResource{Links: adminBillingRefundLinks()},
		AdminRefundResponse: result,
	})
}

// GetDunning handles GET /api/v1/admin/billing/dunning
func (h *AdminBillingHandler) GetDunning(c *gin.Context) {
	adminUserID := h.extractAdminUserID(c)

	result, err := h.dunningUC.Execute(adminUserID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, adminDunningHALResponse{
		HALResource:          HALResource{Links: adminBillingDunningLinks()},
		AdminDunningResponse: result,
	})
}

// extractAdminUserID retrieves the admin user ID from the gin context.
func (h *AdminBillingHandler) extractAdminUserID(c *gin.Context) string {
	adminAuth, err := GetAdminAuthContext(c)
	if err != nil {
		return ""
	}
	return adminAuth.UserID
}
