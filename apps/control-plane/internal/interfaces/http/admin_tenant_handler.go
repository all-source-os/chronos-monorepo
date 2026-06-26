package http

import (
	"errors"
	"fmt"
	"net/http"
	"strconv"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
)

// adminTenantDetailHALResponse wraps an AdminTenantDetailResponse with HAL _links.
type adminTenantDetailHALResponse struct {
	HALResource
	*dto.AdminTenantDetailResponse
}

// adminTenantUsageHALResponse wraps a TenantUsageResponse with HAL _links.
type adminTenantUsageHALResponse struct {
	HALResource
	*dto.TenantUsageResponse
}

// AdminTenantHandler handles admin-level tenant operations including listing,
// detail, usage, quota management, suspend/unsuspend, bulk operations, and the
// read-only tenant-data analysis pass.
type AdminTenantHandler struct {
	listTenantsUC    *usecases.ListTenantsUseCase
	getDetailUC      *usecases.GetAdminTenantDetailUseCase
	getUsageUC       *usecases.GetTenantUsageUseCase
	updateQuotasUC   *usecases.UpdateTenantQuotasUseCase
	suspendTenantUC  *usecases.SuspendTenantUseCase
	activateTenantUC *usecases.ActivateTenantUseCase
	bulkTenantUC     *usecases.BulkTenantUseCase
	// analyzeUC powers GET /api/v1/admin/tenants/analyze (read-only). May be nil
	// in tests that don't exercise the analyze route.
	analyzeUC *usecases.AnalyzeTenantsUseCase
}

// NewAdminTenantHandler creates a new AdminTenantHandler.
func NewAdminTenantHandler(
	listTenantsUC *usecases.ListTenantsUseCase,
	getDetailUC *usecases.GetAdminTenantDetailUseCase,
	getUsageUC *usecases.GetTenantUsageUseCase,
	updateQuotasUC *usecases.UpdateTenantQuotasUseCase,
	suspendTenantUC *usecases.SuspendTenantUseCase,
	activateTenantUC *usecases.ActivateTenantUseCase,
	bulkTenantUC *usecases.BulkTenantUseCase,
	analyzeUC *usecases.AnalyzeTenantsUseCase,
) *AdminTenantHandler {
	return &AdminTenantHandler{
		listTenantsUC:    listTenantsUC,
		getDetailUC:      getDetailUC,
		getUsageUC:       getUsageUC,
		updateQuotasUC:   updateQuotasUC,
		suspendTenantUC:  suspendTenantUC,
		activateTenantUC: activateTenantUC,
		bulkTenantUC:     bulkTenantUC,
		analyzeUC:        analyzeUC,
	}
}

// adminTenantListLinks returns HAL links for the admin tenant list response.
func adminTenantListLinks(page, perPage, totalPages int) map[string]Link {
	links := map[string]Link{
		"self": SelfLink(fmt.Sprintf("/api/v1/admin/tenants?page=%d&per_page=%d", page, perPage)),
	}
	if page > 1 {
		links["prev"] = NewLink(
			fmt.Sprintf("/api/v1/admin/tenants?page=%d&per_page=%d", page-1, perPage),
			WithTitle("Previous Page"),
		)
		links["first"] = NewLink(
			fmt.Sprintf("/api/v1/admin/tenants?page=1&per_page=%d", perPage),
			WithTitle("First Page"),
		)
	}
	if page < totalPages {
		links["next"] = NewLink(
			fmt.Sprintf("/api/v1/admin/tenants?page=%d&per_page=%d", page+1, perPage),
			WithTitle("Next Page"),
		)
		links["last"] = NewLink(
			fmt.Sprintf("/api/v1/admin/tenants?page=%d&per_page=%d", totalPages, perPage),
			WithTitle("Last Page"),
		)
	}
	return links
}

// ListTenants handles GET /api/v1/admin/tenants with search, filter, and pagination.
func (h *AdminTenantHandler) ListTenants(c *gin.Context) {
	page, _ := strconv.Atoi(c.DefaultQuery("page", "1"))         //nolint:errcheck // default is valid int
	perPage, _ := strconv.Atoi(c.DefaultQuery("per_page", "20")) //nolint:errcheck // default is valid int

	if page < 1 {
		page = 1
	}
	if perPage < 1 {
		perPage = 20
	}
	if perPage > 100 {
		perPage = 100
	}

	search := c.Query("search")
	plan := c.Query("plan")
	status := c.Query("status")

	req := usecases.ListTenantsRequest{
		Offset: page,    // ExecuteAdmin interprets Offset as page number
		Limit:  perPage, // ExecuteAdmin interprets Limit as per_page
		Status: status,
		Search: search,
		Plan:   plan,
	}

	resp, err := h.listTenantsUC.ExecuteAdmin(c.Request.Context(), req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"_links":      adminTenantListLinks(resp.Page, resp.PerPage, resp.TotalPages),
		"tenants":     resp.Tenants,
		"total":       resp.Total,
		"page":        resp.Page,
		"per_page":    resp.PerPage,
		"total_pages": resp.TotalPages,
	})
}

// validAnalysisCategories is the set of single-category filters AnalyzeTenants
// accepts on ?category=. Mirrors the dto.AnalysisCategory* constants; an empty
// param runs all categories.
var validAnalysisCategories = map[string]bool{
	dto.AnalysisCategoryDataIntegrity: true,
	dto.AnalysisCategoryPlanBilling:   true,
	dto.AnalysisCategoryLitter:        true,
	dto.AnalysisCategoryUsageHealth:   true,
}

// AnalyzeTenants handles GET /api/v1/admin/tenants/analyze — a READ-ONLY
// fleet-data analysis pass. It scans every tenant (or one, via ?tenant_id=, or
// one category, via ?category=) and returns anomaly findings, each deep-linking
// to an EXISTING guarded action. It NEVER mutates: the use case performs no
// Core writes and no repository writes (a reviewer can confirm by grepping
// analyze_tenants.go for the Core client's write methods — there are none).
func (h *AdminTenantHandler) AnalyzeTenants(c *gin.Context) {
	category := c.Query("category")
	if category != "" && !validAnalysisCategories[category] {
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "invalid category; expected one of data_integrity|plan_billing|litter|usage_health",
		})
		return
	}

	report, err := h.analyzeUC.Execute(c.Request.Context(), usecases.AnalyzeRequest{
		Category: category,
		TenantID: c.Query("tenant_id"),
	})
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, report)
}

// UpdateQuotas handles PUT /api/v1/admin/tenants/:id/quotas
func (h *AdminTenantHandler) UpdateQuotas(c *gin.Context) {
	id := c.Param("id")

	var req dto.UpdateTenantQuotasRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	// At least one quota field must be provided
	if req.EventLimit == nil && req.QueryLimit == nil && req.StorageLimitMB == nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "at least one quota field must be provided"})
		return
	}

	role := h.extractRole(c)
	resp, err := h.updateQuotasUC.Execute(c.Request.Context(), id, req, role)
	if err != nil {
		h.handleAdminError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// SuspendTenant handles POST /api/v1/admin/tenants/:id/suspend
func (h *AdminTenantHandler) SuspendTenant(c *gin.Context) {
	id := c.Param("id")
	role := h.extractRole(c)

	resp, err := h.suspendTenantUC.Execute(c.Request.Context(), id, role)
	if err != nil {
		h.handleAdminError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// UnsuspendTenant handles POST /api/v1/admin/tenants/:id/unsuspend
func (h *AdminTenantHandler) UnsuspendTenant(c *gin.Context) {
	id := c.Param("id")
	role := h.extractRole(c)

	resp, err := h.activateTenantUC.Execute(c.Request.Context(), id, role)
	if err != nil {
		h.handleAdminError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// adminTenantDetailLinks returns the full set of HAL links for an admin tenant detail response.
func adminTenantDetailLinks(tenantID string) map[string]Link {
	return map[string]Link{
		"self":    SelfLink(fmt.Sprintf("/api/v1/admin/tenants/%s", tenantID)),
		"usage":   NewLink(fmt.Sprintf("/api/v1/admin/tenants/%s/usage", tenantID), WithTitle("Tenant Usage")),
		"billing": NewLink(fmt.Sprintf("/api/v1/tenants/%s/billing", tenantID), WithTitle("Tenant Billing")),
		"audit":   NewLink(fmt.Sprintf("/api/v1/tenants/%s/audit", tenantID), WithTitle("Tenant Audit Log")),
		"suspend": NewLink(fmt.Sprintf("/api/v1/admin/tenants/%s/suspend", tenantID), WithTitle("Suspend Tenant")),
		"events":  DataPlaneLink("/api/v1/events{?stream_id,event_type}", WithTitle("Tenant Events")),
	}
}

// adminTenantUsageLinks returns HAL links for the usage endpoint response.
func adminTenantUsageLinks(tenantID string) map[string]Link {
	return map[string]Link{
		"self":   SelfLink(fmt.Sprintf("/api/v1/admin/tenants/%s/usage", tenantID)),
		"tenant": NewLink(fmt.Sprintf("/api/v1/admin/tenants/%s", tenantID), WithTitle("Tenant Detail")),
	}
}

// GetDetail handles GET /api/v1/admin/tenants/:id
func (h *AdminTenantHandler) GetDetail(c *gin.Context) {
	id := c.Param("id")

	resp, err := h.getDetailUC.Execute(c.Request.Context(), id)
	if err != nil {
		if errors.Is(err, domain.ErrTenantNotFound) {
			c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, adminTenantDetailHALResponse{
		HALResource:               HALResource{Links: adminTenantDetailLinks(resp.ID)},
		AdminTenantDetailResponse: resp,
	})
}

// GetUsage handles GET /api/v1/admin/tenants/:id/usage
func (h *AdminTenantHandler) GetUsage(c *gin.Context) {
	id := c.Param("id")

	resp, err := h.getUsageUC.Execute(c.Request.Context(), id)
	if err != nil {
		if errors.Is(err, domain.ErrTenantNotFound) {
			c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, adminTenantUsageHALResponse{
		HALResource:         HALResource{Links: adminTenantUsageLinks(resp.TenantID)},
		TenantUsageResponse: resp,
	})
}

// BulkAction handles POST /api/v1/admin/tenants/bulk
func (h *AdminTenantHandler) BulkAction(c *gin.Context) {
	var req usecases.BulkTenantRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	role := h.extractRole(c)
	resp, err := h.bulkTenantUC.Execute(c.Request.Context(), req, role)
	if err != nil {
		h.handleAdminError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// extractRole retrieves the caller's role from the gin context.
func (h *AdminTenantHandler) extractRole(c *gin.Context) entities.Role {
	role, exists := c.Get("auth_role")
	if !exists {
		return ""
	}
	if r, ok := role.(entities.Role); ok {
		return r
	}
	return ""
}

// handleAdminError maps domain errors to HTTP status codes for admin endpoints.
func (h *AdminTenantHandler) handleAdminError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrTenantNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	case errors.Is(err, domain.ErrForbidden):
		c.JSON(http.StatusForbidden, gin.H{"error": err.Error()})
	case errors.Is(err, domain.ErrInvalidInput):
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}
