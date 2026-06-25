package http

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// ViewAsHandler exposes the read-only "view as tenant" impersonation endpoints
// (ADMIN_TENANT_POWER_TOOL §5 / §9 Phase 7 CP half):
//
//	admin (gated by AdminAuthMiddleware in the /api/v1/admin group):
//	  POST /api/v1/admin/tenants/:id/view-as       mint a readonly+view_as token
//	  POST /api/v1/admin/tenants/:id/view-as/stop  teardown — write admin.viewas.stopped
//
// The mint returns a token DISTINCT from the tenant's real session
// (role:readonly, view_as:true, ~15m TTL). Read-only is enforced THREE ways: the
// readonly role, the ViewAsWriteRefusal middleware (which hard-refuses any
// view_as token on a mutating method, in addition to the role), and the surface
// (the admin frame proxies reads only). This handler only binds the request,
// injects the admin actor from the auth context (never client-trusted), and maps
// use-case errors — the audit/guard lives in the use-case layer.
type ViewAsHandler struct {
	viewAsUC *usecases.ViewAsUseCase
}

// NewViewAsHandler creates a ViewAsHandler.
func NewViewAsHandler(viewAsUC *usecases.ViewAsUseCase) *ViewAsHandler {
	return &ViewAsHandler{viewAsUC: viewAsUC}
}

// Start handles POST /api/v1/admin/tenants/:id/view-as. It mints the read-only
// view-as token for tenant :id and records admin.viewas.started BEFORE returning,
// so the audit always brackets a started before any read with the token.
func (h *ViewAsHandler) Start(c *gin.Context) {
	if h.viewAsUC == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "view-as not configured"})
		return
	}
	id := c.Param("id")
	actor := adminActor(c)

	res, err := h.viewAsUC.Start(c.Request.Context(), id, actor)
	if err != nil {
		h.handleViewAsError(c, err)
		return
	}
	c.JSON(http.StatusOK, res)
}

// stopBody is the wire shape for an explicit teardown. reason is optional and
// defaults to "exit"; the admin frame (041) calls this on the one-click Exit. The
// expiry-driven stop (reason "expired") is emitted by the ViewAsWriteRefusal /
// lazy-expiry path, not this endpoint.
type stopBody struct {
	Reason string `json:"reason"`
}

// Stop handles POST /api/v1/admin/tenants/:id/view-as/stop. It writes
// admin.viewas.stopped so every started has a paired stopped. The view-as token
// is stateless + short-TTL, so there is nothing to revoke server-side — Stop is
// the audit teardown the admin frame calls when the operator clicks Exit.
func (h *ViewAsHandler) Stop(c *gin.Context) {
	if h.viewAsUC == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "view-as not configured"})
		return
	}
	id := c.Param("id")
	actor := adminActor(c)

	var body stopBody
	if c.Request.Body != nil && c.Request.ContentLength != 0 {
		if err := c.ShouldBindJSON(&body); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
			return
		}
	}
	reason := body.Reason
	if reason == "" {
		reason = usecases.ViewAsStopReasonExit
	}

	if err := h.viewAsUC.Stop(c.Request.Context(), id, actor, reason); err != nil {
		h.handleViewAsError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"stopped": true, "tenant_id": id, "reason": reason})
}

// handleViewAsError maps use-case / domain errors to HTTP status codes.
func (h *ViewAsHandler) handleViewAsError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrTenantNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	case errors.Is(err, domain.ErrInvalidInput):
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
	case errors.Is(err, usecases.ErrViewAsUnavailable):
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}
