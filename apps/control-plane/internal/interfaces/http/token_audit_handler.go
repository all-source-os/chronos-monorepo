package http //nolint:revive // package name intentionally matches directory

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
)

// TokenAuditHandler handles token audit HTTP requests.
type TokenAuditHandler struct {
	queryTokenAuditUC *usecases.QueryTokenAuditUseCase
}

// NewTokenAuditHandler creates a new TokenAuditHandler.
func NewTokenAuditHandler(queryTokenAuditUC *usecases.QueryTokenAuditUseCase) *TokenAuditHandler {
	return &TokenAuditHandler{queryTokenAuditUC: queryTokenAuditUC}
}

// Query handles GET /api/v1/admin/security/token-audit
func (h *TokenAuditHandler) Query(c *gin.Context) {
	var req dto.TokenAuditQueryRequest
	if err := c.ShouldBindQuery(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.queryTokenAuditUC.Execute(req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	links := tokenAuditListLinks(resp.Page, resp.PerPage, resp.TotalPages, req.TenantID)

	c.JSON(http.StatusOK, gin.H{
		"_links":      links,
		"entries":     resp.Entries,
		"total":       resp.Total,
		"page":        resp.Page,
		"per_page":    resp.PerPage,
		"total_pages": resp.TotalPages,
	})
}

// Summary handles GET /api/v1/admin/security/token-audit/summary
func (h *TokenAuditHandler) Summary(c *gin.Context) {
	var req dto.TokenAuditSummaryRequest
	if err := c.ShouldBindQuery(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.queryTokenAuditUC.ExecuteSummary(req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	links := map[string]Link{
		"self": NewLink("/api/v1/admin/security/token-audit/summary", WithTitle("Token Audit Summary")),
		"list": NewLink("/api/v1/admin/security/token-audit", WithTitle("Token Audit Log")),
	}

	c.JSON(http.StatusOK, gin.H{
		"_links":  links,
		"tenants": resp.Tenants,
		"total":   resp.Total,
	})
}

// tokenAuditListLinks returns HAL pagination links for the token audit list.
func tokenAuditListLinks(page, perPage, totalPages int, tenantID string) map[string]Link {
	basePath := "/api/v1/admin/security/token-audit"
	tenantParam := ""
	if tenantID != "" {
		tenantParam = "&tenant_id=" + tenantID
	}

	links := map[string]Link{
		"self": NewLink(fmt.Sprintf("%s?page=%d&per_page=%d%s", basePath, page, perPage, tenantParam), WithTitle("Token Audit Log")),
	}

	if page > 1 {
		links["first"] = NewLink(fmt.Sprintf("%s?page=1&per_page=%d%s", basePath, perPage, tenantParam))
		links["prev"] = NewLink(fmt.Sprintf("%s?page=%d&per_page=%d%s", basePath, page-1, perPage, tenantParam))
	}
	if page < totalPages {
		links["next"] = NewLink(fmt.Sprintf("%s?page=%d&per_page=%d%s", basePath, page+1, perPage, tenantParam))
		links["last"] = NewLink(fmt.Sprintf("%s?page=%d&per_page=%d%s", basePath, totalPages, perPage, tenantParam))
	}

	links["summary"] = NewLink(basePath+"/summary", WithTitle("Token Audit Summary"))

	return links
}
