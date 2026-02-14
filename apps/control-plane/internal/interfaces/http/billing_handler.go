package http //nolint:revive // package name intentionally matches directory

import (
	"errors"
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// --- HAL response wrappers ---

type checkoutHALResponse struct {
	HALResource
	*usecases.CheckoutResult
}

type portalHALResponse struct {
	HALResource
	*usecases.PortalResult
}

type overageHALResponse struct {
	HALResource
	*usecases.OverageSummary
}

type projectedChargesHALResponse struct {
	HALResource
	*usecases.ProjectedCharges
}

// --- HAL link helpers ---

func billingLinks(tenantID string) map[string]Link {
	return map[string]Link{
		"self":              SelfLink("/api/v1/billing/checkout"),
		"portal":           NewLink("/api/v1/billing/portal", WithTitle("Customer Portal")),
		"overage":          NewLink(fmt.Sprintf("/api/v1/billing/overage?tenant_id=%s", tenantID), WithTitle("Overage Summary")),
		"projected-charges": NewLink(fmt.Sprintf("/api/v1/billing/projected-charges?tenant_id=%s", tenantID), WithTitle("Projected Charges")),
		"tenant":           NewLink(fmt.Sprintf("/api/v1/tenants/%s", tenantID), WithTitle("Tenant")),
	}
}

func portalLinks(tenantID string) map[string]Link {
	return map[string]Link{
		"self":    SelfLink("/api/v1/billing/portal"),
		"overage": NewLink(fmt.Sprintf("/api/v1/billing/overage?tenant_id=%s", tenantID), WithTitle("Overage Summary")),
		"tenant":  NewLink(fmt.Sprintf("/api/v1/tenants/%s", tenantID), WithTitle("Tenant")),
	}
}

func overageLinks(tenantID string) map[string]Link {
	return map[string]Link{
		"self":              SelfLink(fmt.Sprintf("/api/v1/billing/overage?tenant_id=%s", tenantID)),
		"enable":           NewLink("/api/v1/billing/overage/enable", WithTitle("Enable Overage")),
		"disable":          NewLink("/api/v1/billing/overage/disable", WithTitle("Disable Overage")),
		"projected-charges": NewLink(fmt.Sprintf("/api/v1/billing/projected-charges?tenant_id=%s", tenantID), WithTitle("Projected Charges")),
		"tenant":           NewLink(fmt.Sprintf("/api/v1/tenants/%s", tenantID), WithTitle("Tenant")),
	}
}

func projectedChargesLinks(tenantID string) map[string]Link {
	return map[string]Link{
		"self":    SelfLink(fmt.Sprintf("/api/v1/billing/projected-charges?tenant_id=%s", tenantID)),
		"overage": NewLink(fmt.Sprintf("/api/v1/billing/overage?tenant_id=%s", tenantID), WithTitle("Overage Summary")),
		"tenant":  NewLink(fmt.Sprintf("/api/v1/tenants/%s", tenantID), WithTitle("Tenant")),
	}
}

// --- Handler ---

// BillingHandler handles billing-related HTTP requests.
type BillingHandler struct {
	createCheckoutUC      *usecases.CreateCheckoutUseCase
	getPortalUC           *usecases.GetPortalUseCase
	getOverageSummaryUC   *usecases.GetOverageSummaryUseCase
	setOverageEnabledUC   *usecases.SetOverageEnabledUseCase
	getProjectedChargesUC *usecases.GetProjectedChargesUseCase
}

// NewBillingHandler creates a new BillingHandler.
func NewBillingHandler(
	createCheckoutUC *usecases.CreateCheckoutUseCase,
	getPortalUC *usecases.GetPortalUseCase,
	getOverageSummaryUC *usecases.GetOverageSummaryUseCase,
	setOverageEnabledUC *usecases.SetOverageEnabledUseCase,
	getProjectedChargesUC *usecases.GetProjectedChargesUseCase,
) *BillingHandler {
	return &BillingHandler{
		createCheckoutUC:      createCheckoutUC,
		getPortalUC:           getPortalUC,
		getOverageSummaryUC:   getOverageSummaryUC,
		setOverageEnabledUC:   setOverageEnabledUC,
		getProjectedChargesUC: getProjectedChargesUC,
	}
}

// CreateCheckout handles POST /api/v1/billing/checkout
func (h *BillingHandler) CreateCheckout(c *gin.Context) {
	var req usecases.CheckoutRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	result, err := h.createCheckoutUC.Execute(c.Request.Context(), req)
	if err != nil {
		h.handleBillingError(c, err)
		return
	}

	c.JSON(http.StatusOK, checkoutHALResponse{
		HALResource:    HALResource{Links: billingLinks(req.TenantID)},
		CheckoutResult: result,
	})
}

// GetPortal handles GET /api/v1/billing/portal
func (h *BillingHandler) GetPortal(c *gin.Context) {
	tenantID := c.Query("tenant_id")
	if tenantID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id query parameter is required"})
		return
	}

	result, err := h.getPortalUC.Execute(c.Request.Context(), tenantID)
	if err != nil {
		h.handleBillingError(c, err)
		return
	}

	c.JSON(http.StatusOK, portalHALResponse{
		HALResource:  HALResource{Links: portalLinks(tenantID)},
		PortalResult: result,
	})
}

// GetOverage handles GET /api/v1/billing/overage
func (h *BillingHandler) GetOverage(c *gin.Context) {
	tenantID := c.Query("tenant_id")
	if tenantID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id query parameter is required"})
		return
	}

	result, err := h.getOverageSummaryUC.Execute(tenantID)
	if err != nil {
		h.handleBillingError(c, err)
		return
	}

	c.JSON(http.StatusOK, overageHALResponse{
		HALResource:    HALResource{Links: overageLinks(tenantID)},
		OverageSummary: result,
	})
}

// EnableOverage handles POST /api/v1/billing/overage/enable
func (h *BillingHandler) EnableOverage(c *gin.Context) {
	tenantID := c.Query("tenant_id")
	if tenantID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id query parameter is required"})
		return
	}

	result, err := h.setOverageEnabledUC.Execute(tenantID, true)
	if err != nil {
		h.handleBillingError(c, err)
		return
	}

	c.JSON(http.StatusOK, overageHALResponse{
		HALResource:    HALResource{Links: overageLinks(tenantID)},
		OverageSummary: result,
	})
}

// DisableOverage handles POST /api/v1/billing/overage/disable
func (h *BillingHandler) DisableOverage(c *gin.Context) {
	tenantID := c.Query("tenant_id")
	if tenantID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id query parameter is required"})
		return
	}

	result, err := h.setOverageEnabledUC.Execute(tenantID, false)
	if err != nil {
		h.handleBillingError(c, err)
		return
	}

	c.JSON(http.StatusOK, overageHALResponse{
		HALResource:    HALResource{Links: overageLinks(tenantID)},
		OverageSummary: result,
	})
}

// GetProjectedCharges handles GET /api/v1/billing/projected-charges
func (h *BillingHandler) GetProjectedCharges(c *gin.Context) {
	tenantID := c.Query("tenant_id")
	if tenantID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id query parameter is required"})
		return
	}

	result, err := h.getProjectedChargesUC.Execute(tenantID)
	if err != nil {
		h.handleBillingError(c, err)
		return
	}

	c.JSON(http.StatusOK, projectedChargesHALResponse{
		HALResource:      HALResource{Links: projectedChargesLinks(tenantID)},
		ProjectedCharges: result,
	})
}

// handleBillingError maps domain errors to HTTP status codes.
func (h *BillingHandler) handleBillingError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrTenantNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}
