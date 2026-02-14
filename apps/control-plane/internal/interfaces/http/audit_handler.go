package http //nolint:revive // package name intentionally matches directory

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
)

// auditEventHALResponse wraps an AuditEventResponse with HAL _links.
type auditEventHALResponse struct {
	HALResource
	*dto.AuditEventResponse
}

// auditEventLinks returns HAL links for an audit event.
// Links: self (templated to Core), tenant, user (if present).
func auditEventLinks(event *dto.AuditEventResponse) map[string]Link {
	links := map[string]Link{
		"self": DataPlaneLink(fmt.Sprintf("/api/v1/audit/%s", event.EventType), WithTitle("Audit Event")),
	}
	if event.TenantID != "" {
		links["tenant"] = NewLink(fmt.Sprintf("/api/v1/tenants/%s", event.TenantID), WithTitle("Tenant"))
	}
	if event.UserID != "" {
		links["user"] = NewLink(fmt.Sprintf("/api/v1/users/%s", event.UserID), WithTitle("User"))
	}
	return links
}

// wrapAuditEventWithLinks wraps an AuditEventResponse with HAL links.
func wrapAuditEventWithLinks(event *dto.AuditEventResponse) auditEventHALResponse {
	return auditEventHALResponse{
		HALResource:        HALResource{Links: auditEventLinks(event)},
		AuditEventResponse: event,
	}
}

// AuditHandler handles audit trail HTTP requests
type AuditHandler struct {
	queryAuditUC *usecases.QueryAuditUseCase
}

// NewAuditHandler creates a new AuditHandler
func NewAuditHandler(queryAuditUC *usecases.QueryAuditUseCase) *AuditHandler {
	return &AuditHandler{queryAuditUC: queryAuditUC}
}

// Query handles GET /api/v1/audit
func (h *AuditHandler) Query(c *gin.Context) {
	var req dto.AuditQueryRequest
	if err := c.ShouldBindQuery(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	events, err := h.queryAuditUC.Execute(req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	halEvents := make([]auditEventHALResponse, len(events))
	for i, e := range events {
		halEvents[i] = wrapAuditEventWithLinks(e)
	}

	c.JSON(http.StatusOK, gin.H{"events": halEvents, "total": len(halEvents)})
}
