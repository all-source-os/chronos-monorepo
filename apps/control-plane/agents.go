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

// AgentAnonymousTrialHandler handles POST /api/v1/agents/anonymous-trial.
//
// Mints a low-quota, time-limited tenant + API key WITHOUT requiring a
// signed-in user. Designed to let an agent (e.g. Claude Desktop walking a
// human through the install protocol from
// apps/web/content/allsource-as-cms-from-claude-desktop.mdx) skip the
// /connect signup round-trip. Closes the mint half of bead t-072c; the
// claim half lives in a separate follow-up.
//
// Rate limiting is applied at the route layer (see main.go) — without it,
// this unauthenticated endpoint would be trivial to abuse.
func (cp *ControlPlane) AgentAnonymousTrialHandler(c *gin.Context) {
	var req dto.RegisterTrialAgentRequest
	// Empty body is valid — agent_name and client_fingerprint are both
	// optional. ShouldBindJSON returns an error on empty body, so tolerate
	// it explicitly.
	if err := c.ShouldBindJSON(&req); err != nil && c.Request.ContentLength > 0 {
		c.JSON(http.StatusBadRequest, gin.H{
			"error":   "invalid_request",
			"message": err.Error(),
		})
		return
	}

	resp, err := cp.container.RegisterTrialAgentUC.Execute(c.Request.Context(), req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{
			"error":   "trial_registration_failed",
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
