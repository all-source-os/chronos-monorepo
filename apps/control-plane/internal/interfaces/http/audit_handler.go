package http //nolint:revive // package name intentionally matches directory

import (
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
)

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

	c.JSON(http.StatusOK, gin.H{"events": events, "total": len(events)})
}
