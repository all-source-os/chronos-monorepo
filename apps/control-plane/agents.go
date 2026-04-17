package main

import (
	"errors"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
)

// AgentRegisterHandler handles POST /api/v1/agents/register.
// Thin HTTP adapter — all business logic lives in RegisterAgentUseCase.
func (cp *ControlPlane) AgentRegisterHandler(c *gin.Context) {
	var req dto.RegisterAgentRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{
			"error":   "invalid_request",
			"message": err.Error(),
		})
		return
	}

	resp, err := cp.container.RegisterAgentUC.Execute(c.Request.Context(), req)
	if err != nil {
		if errors.Is(err, domain.ErrTenantAlreadyExists) {
			c.JSON(http.StatusConflict, gin.H{
				"error":   "agent_exists",
				"message": "An agent with this name is already registered.",
			})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{
			"error":   "registration_failed",
			"message": err.Error(),
		})
		return
	}

	c.JSON(http.StatusCreated, resp)
}

// AgentEchoHandler handles POST /api/v1/agent-echo — the reference x402
// endpoint. It exists specifically to verify end-to-end x402 flows against
// the deployed Control Plane: tier gate (free → 403), payment required
// (Pro+ without payment header → 402), and auto-pay settlement (Pro+ with
// CDP wallet → 200). The handler itself is a trivial echo — all the
// interesting behavior happens in the middleware chain that guards it.
//
// Pricing is configured in config/x402-pricing.json under the route key
// "POST /api/v1/agent-echo". Without that config entry the route behaves
// like any other authenticated endpoint.
func (cp *ControlPlane) AgentEchoHandler(c *gin.Context) {
	var payload map[string]any
	// Body is optional — an empty POST is a valid "ping".
	if err := c.ShouldBindJSON(&payload); err != nil {
		payload = nil
	}

	tenantID, _ := c.Get("tenant_id")

	c.JSON(http.StatusOK, gin.H{
		"echo":      payload,
		"tenant_id": tenantID,
		"ts":        time.Now().UTC().Format(time.RFC3339Nano),
	})
}
