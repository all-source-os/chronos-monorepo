package http

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// CommsHandler exposes the proactive-comms endpoints (ADMIN_TENANT_POWER_TOOL §4
// Pillar C / §9 Phase 6 CP half):
//
//	admin (gated by AdminAuthMiddleware in the /api/v1/admin group):
//	  POST /api/v1/admin/notices              create an in-app notice (tenant or cohort)
//	  GET  /api/v1/admin/notices?tenant_id=   list notices (admin view)
//	  POST /api/v1/admin/messages             send an operator→tenant email (SMTP)
//	  POST /api/v1/admin/tenants/:id/notes    add a support note
//	  GET  /api/v1/admin/tenants/:id/notes    list support notes
//
//	tenant-facing (caller-scoped, the AUTHENTICATED tenant's OWN notices — NOT
//	admin-gated; consumed by the web dashboard banner):
//	  GET  /api/v1/notices                    the caller's active notices
//	  POST /api/v1/notices/:id/dismiss        dismiss one of the caller's notices
//
// Like RecoveryHandler, every guard (cohort dry-run/confirm-token/blast-radius,
// opt-out, rate-limit) is enforced in the USE-CASE layer; this handler only binds
// the body, injects the actor/tenant from the auth context, and maps errors.
type CommsHandler struct {
	commsUC *usecases.CommsUseCase
}

// NewCommsHandler creates a CommsHandler.
func NewCommsHandler(commsUC *usecases.CommsUseCase) *CommsHandler {
	return &CommsHandler{commsUC: commsUC}
}

// ---- admin: notices ----

// CreateNotice handles POST /api/v1/admin/notices.
func (h *CommsHandler) CreateNotice(c *gin.Context) {
	var req usecases.CreateNoticeRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	// ?dry_run=true on the query string forces a dry-run even if the body omits it
	// (matches the recovery convention so a raw curl previews a cohort before send).
	if c.Query("dry_run") == "true" {
		req.DryRun = true
	}
	req.Actor = adminActor(c)

	res, err := h.commsUC.CreateNotice(c.Request.Context(), req)
	if err != nil {
		h.handleCommsError(c, err)
		return
	}
	c.JSON(http.StatusOK, res)
}

// ListNotices handles GET /api/v1/admin/notices?tenant_id= (admin view).
func (h *CommsHandler) ListNotices(c *gin.Context) {
	tenantID := c.Query("tenant_id") // empty = all tenants
	notices, err := h.commsUC.ListNotices(c.Request.Context(), tenantID)
	if err != nil {
		h.handleCommsError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"notices": notices, "count": len(notices)})
}

// ---- admin: messages ----

// SendMessage handles POST /api/v1/admin/messages.
func (h *CommsHandler) SendMessage(c *gin.Context) {
	var req usecases.SendMessageRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	if c.Query("dry_run") == "true" {
		req.DryRun = true
	}
	req.Actor = adminActor(c)

	res, err := h.commsUC.SendMessage(c.Request.Context(), req)
	if err != nil {
		h.handleCommsError(c, err)
		return
	}
	c.JSON(http.StatusOK, res)
}

// ---- admin: notes ----

// AddNote handles POST /api/v1/admin/tenants/:id/notes.
func (h *CommsHandler) AddNote(c *gin.Context) {
	var body struct {
		Body string `json:"body"`
	}
	if err := c.ShouldBindJSON(&body); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	res, err := h.commsUC.AddNote(c.Request.Context(), usecases.AddNoteRequest{
		TenantID: c.Param("id"),
		Body:     body.Body,
		Actor:    adminActor(c),
	})
	if err != nil {
		h.handleCommsError(c, err)
		return
	}
	c.JSON(http.StatusOK, res)
}

// ListNotes handles GET /api/v1/admin/tenants/:id/notes.
func (h *CommsHandler) ListNotes(c *gin.Context) {
	notes, err := h.commsUC.ListNotes(c.Request.Context(), c.Param("id"))
	if err != nil {
		h.handleCommsError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"notes": notes, "count": len(notes)})
}

// ---- tenant-facing: notices banner (caller-scoped, NOT admin-gated) ----

// TenantNotices handles GET /api/v1/notices — the active notices for the
// caller's OWN tenant. The tenant id comes from the authenticated caller's JWT
// (auth_tenant_id), never a query param, so a tenant can only read its own
// notices.
func (h *CommsHandler) TenantNotices(c *gin.Context) {
	tenantID := callerTenantID(c)
	if tenantID == "" {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "no tenant in auth context"})
		return
	}
	notices, err := h.commsUC.TenantNotices(c.Request.Context(), tenantID)
	if err != nil {
		h.handleCommsError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"notices": notices, "count": len(notices)})
}

// DismissNotice handles POST /api/v1/notices/:id/dismiss — the caller dismisses
// one of its OWN notices. Scoped to the caller's tenant; a notice belonging to
// another tenant 404s.
func (h *CommsHandler) DismissNotice(c *gin.Context) {
	tenantID := callerTenantID(c)
	if tenantID == "" {
		c.JSON(http.StatusUnauthorized, gin.H{"error": "no tenant in auth context"})
		return
	}
	if err := h.commsUC.DismissNotice(c.Request.Context(), tenantID, c.Param("id")); err != nil {
		h.handleCommsError(c, err)
		return
	}
	c.JSON(http.StatusOK, gin.H{"dismissed": true, "notice_id": c.Param("id")})
}

// callerTenantID reads the authenticated caller's own tenant id from the context
// (set by AuthMiddleware as `auth_tenant_id` — the SAME key billing_handler.go
// reads for self-serve scoping).
func callerTenantID(c *gin.Context) string {
	return c.GetString("auth_tenant_id")
}

// handleCommsError maps comms guard / domain errors to HTTP status, mirroring the
// recovery handler's mapping so the UI, a raw curl, and a future MCP see the same
// status codes.
func (h *CommsHandler) handleCommsError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrTenantNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	case errors.Is(err, usecases.ErrCommsInvalidInput),
		errors.Is(err, usecases.ErrCommsUnknownTemplate),
		errors.Is(err, usecases.ErrCommsNoRecipient),
		errors.Is(err, usecases.ErrConfirmRequired),
		errors.Is(err, usecases.ErrConfirmMismatch):
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
	case errors.Is(err, usecases.ErrCommsCohortTooLarge):
		// 422: well-formed but the blast radius exceeds the cap (matches Batch).
		c.JSON(http.StatusUnprocessableEntity, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}
