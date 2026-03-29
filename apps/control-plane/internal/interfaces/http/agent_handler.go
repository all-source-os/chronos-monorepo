package http

import (
	"net/http"
	"strconv"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
)

// AgentHandler handles agent-related HTTP requests.
type AgentHandler struct {
	paymentHistoryUC *usecases.GetAgentPaymentHistoryUseCase
}

// NewAgentHandler creates a new AgentHandler.
func NewAgentHandler(paymentHistoryUC *usecases.GetAgentPaymentHistoryUseCase) *AgentHandler {
	return &AgentHandler{paymentHistoryUC: paymentHistoryUC}
}

// GetPaymentHistory handles GET /api/v1/agents/me/payments.
// Returns x402 payment history for the authenticated agent's tenant.
func (h *AgentHandler) GetPaymentHistory(c *gin.Context) {
	tenantIDVal, exists := c.Get("auth_tenant_id")
	if !exists {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "missing tenant context"})
		return
	}
	tid, ok := tenantIDVal.(string)
	if !ok || tid == "" {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "missing tenant context"})
		return
	}

	req := usecases.AgentPaymentHistoryRequest{
		TenantID: tid,
		Since:    c.Query("since"),
		Until:    c.Query("until"),
		Network:  c.Query("network"),
		Limit:    100,
		Offset:   0,
	}

	if limitStr := c.Query("limit"); limitStr != "" {
		if v, err := strconv.Atoi(limitStr); err == nil && v > 0 && v <= 1000 {
			req.Limit = v
		}
	}
	if offsetStr := c.Query("offset"); offsetStr != "" {
		if v, err := strconv.Atoi(offsetStr); err == nil && v >= 0 {
			req.Offset = v
		}
	}

	resp, err := h.paymentHistoryUC.Execute(c.Request.Context(), req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, resp)
}
