package main

import (
	"errors"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
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

// AgentClaimHandler handles POST /api/v1/agents/claim (auth required).
//
// Closes the CLAIM half of bead t-e8b8. Associates a previously-minted
// anonymous trial tenant with the calling authenticated user by:
//   - looking up the trial tenant by claim_token in tenant metadata
//   - verifying it hasn't expired or been claimed already
//   - updating metadata to record the claim
//   - writing audit + Core events
//
// Event migration (re-keying trial events under the new tenant) is OUT
// OF SCOPE for v1 — the trial tenant's data stays put; a future
// gateway-resolved mapping reads claimed_by_tenant when serving reads.
func (cp *ControlPlane) AgentClaimHandler(c *gin.Context) {
	var req dto.ClaimTrialAgentRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{
			"error":   "invalid_request",
			"message": err.Error(),
		})
		return
	}

	tenantIDVal, exists := c.Get("tenant_id")
	if !exists {
		// Should never happen — auth middleware sets this for every
		// authenticated request. Defensive: surface as 401 rather than
		// crash on type assertion.
		c.JSON(http.StatusUnauthorized, gin.H{
			"error":   "missing_tenant_id",
			"message": "tenant_id not set in auth context",
		})
		return
	}
	claimingTenantID, _ := tenantIDVal.(string) //nolint:errcheck // type assertion's second value is a bool ok-flag, not an error

	resp, err := cp.container.ClaimTrialAgentUC.Execute(c.Request.Context(), req, claimingTenantID)
	if err != nil {
		switch {
		case errors.Is(err, usecases.ErrClaimTokenNotFound):
			c.JSON(http.StatusNotFound, gin.H{
				"error":   "claim_token_not_found",
				"message": "No trial tenant matches this claim token.",
			})
		case errors.Is(err, usecases.ErrTrialExpired):
			c.JSON(http.StatusGone, gin.H{
				"error":   "trial_expired",
				"message": "This trial has expired and can no longer be claimed.",
			})
		case errors.Is(err, usecases.ErrAlreadyClaimed):
			c.JSON(http.StatusGone, gin.H{
				"error":   "already_claimed",
				"message": "This trial has already been claimed.",
			})
		default:
			c.JSON(http.StatusInternalServerError, gin.H{
				"error":   "claim_failed",
				"message": err.Error(),
			})
		}
		return
	}

	c.JSON(http.StatusOK, resp)
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
