package http //nolint:revive // package name intentionally matches directory

import (
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
)

// SuspiciousActivityHandler handles suspicious activity detection HTTP requests.
type SuspiciousActivityHandler struct {
	detectSuspiciousActivityUC *usecases.DetectSuspiciousActivityUseCase
}

// NewSuspiciousActivityHandler creates a new SuspiciousActivityHandler.
func NewSuspiciousActivityHandler(detectSuspiciousActivityUC *usecases.DetectSuspiciousActivityUseCase) *SuspiciousActivityHandler {
	return &SuspiciousActivityHandler{detectSuspiciousActivityUC: detectSuspiciousActivityUC}
}

// Detect handles GET /api/v1/admin/security/suspicious-activity
func (h *SuspiciousActivityHandler) Detect(c *gin.Context) {
	resp, err := h.detectSuspiciousActivityUC.Execute()
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	links := map[string]Link{
		"self":        NewLink("/api/v1/admin/security/suspicious-activity", WithTitle("Suspicious Activity Detection")),
		"token-audit": NewLink("/api/v1/admin/security/token-audit", WithTitle("Token Audit Log")),
	}

	c.JSON(http.StatusOK, gin.H{
		"_links": links,
		"alerts": resp.Alerts,
		"total":  resp.Total,
	})
}
