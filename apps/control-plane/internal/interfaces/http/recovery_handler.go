package http

import (
	"context"
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// RecoveryHandler exposes the POST /api/v1/admin/recovery/* mutating endpoints
// (§5.1 P1). Like FleetHealthHandler it lives inside /api/v1/admin and inherits
// AdminAuthMiddleware. Every guard (dry-run, confirmation, blast-radius) is
// enforced in the USE-CASE layer (RecoveryUseCase) — this handler only parses
// the body, injects the admin actor, and maps guard errors to HTTP status, so a
// raw curl is bound by the same rules the UI renders (§6.3).
type RecoveryHandler struct {
	recoveryUC *usecases.RecoveryUseCase
}

// NewRecoveryHandler creates a RecoveryHandler.
func NewRecoveryHandler(recoveryUC *usecases.RecoveryUseCase) *RecoveryHandler {
	return &RecoveryHandler{recoveryUC: recoveryUC}
}

// recoveryBody is the wire shape for a single-tenant mutating action. Action-
// specific fields (snapshot_id, as_of, mode, entity_id) ride in Extra.
type recoveryBody struct {
	DryRun          bool           `json:"dry_run"`
	Reason          string         `json:"reason"`
	ConfirmTenantID string         `json:"confirm_tenant_id"`
	ConfirmToken    string         `json:"confirm_token"`
	Extra           map[string]any `json:"-"`
	// Captured separately below so we keep both the typed fields and the raw
	// action-specific extras.
	SnapshotID string `json:"snapshot_id,omitempty"`
	AsOf       string `json:"as_of,omitempty"`
	Mode       string `json:"mode,omitempty"`
	EntityID   string `json:"entity_id,omitempty"`
}

// toRequest converts the wire body to a use-case RecoveryRequest, injecting the
// admin actor from the auth context (never client-trusted).
func (h *RecoveryHandler) toRequest(c *gin.Context, b recoveryBody) usecases.RecoveryRequest {
	extra := map[string]any{}
	if b.SnapshotID != "" {
		extra["snapshot_id"] = b.SnapshotID
	}
	if b.AsOf != "" {
		extra["as_of"] = b.AsOf
	}
	if b.Mode != "" {
		extra["mode"] = b.Mode
	}
	if b.EntityID != "" {
		extra["entity_id"] = b.EntityID
	}
	return usecases.RecoveryRequest{
		DryRun:          b.DryRun,
		Reason:          b.Reason,
		ConfirmTenantID: b.ConfirmTenantID,
		ConfirmToken:    b.ConfirmToken,
		Actor:           adminActor(c),
		Extra:           extra,
	}
}

// bindBody binds the JSON body; an empty body is allowed (defaults to a dry-run
// false / no-confirmation request, which guards will reject for Destructive
// actions — exactly the desired behavior).
func bindBody(c *gin.Context) (recoveryBody, bool) {
	var b recoveryBody
	if c.Request.Body != nil && c.Request.ContentLength != 0 {
		if err := c.ShouldBindJSON(&b); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
			return b, false
		}
	}
	// ?dry_run=true on the query string forces a dry-run even with an empty body.
	if c.Query("dry_run") == "true" {
		b.DryRun = true
	}
	return b, true
}

// Resync handles POST /api/v1/admin/recovery/:id/resync (Guarded).
func (h *RecoveryHandler) Resync(c *gin.Context) {
	h.runSingle(c, h.recoveryUC.ForceResync)
}

// ReconcileSubscription handles POST /api/v1/admin/recovery/:id/reconcile-subscription (Guarded).
func (h *RecoveryHandler) ReconcileSubscription(c *gin.Context) {
	h.runSingle(c, h.recoveryUC.ReconcileSubscription)
}

// ResolveDunning handles POST /api/v1/admin/recovery/:id/resolve-dunning (Guarded).
func (h *RecoveryHandler) ResolveDunning(c *gin.Context) {
	h.runSingle(c, h.recoveryUC.ResolveDunning)
}

// RotateKeys handles POST /api/v1/admin/recovery/:id/rotate-keys (Destructive, confirm_token).
func (h *RecoveryHandler) RotateKeys(c *gin.Context) {
	h.runSingle(c, h.recoveryUC.RotateKeys)
}

// Reprovision handles POST /api/v1/admin/recovery/:id/reprovision (Destructive, confirm_tenant_id).
func (h *RecoveryHandler) Reprovision(c *gin.Context) {
	h.runSingle(c, h.recoveryUC.Reprovision)
}

// Restore handles POST /api/v1/admin/recovery/:id/restore (Destructive, confirm_tenant_id + snapshot_id).
func (h *RecoveryHandler) Restore(c *gin.Context) {
	h.runSingle(c, h.recoveryUC.Restore)
}

// singleActionFn is the shared signature of every single-tenant recovery method
// on RecoveryUseCase (ForceResync, Reprovision, Restore, …).
type singleActionFn func(ctx context.Context, tenantID string, req usecases.RecoveryRequest) (*usecases.RecoveryResult, error)

// runSingle is the shared driver for single-tenant actions: bind → build request
// → call the use case → map errors → respond.
func (h *RecoveryHandler) runSingle(c *gin.Context, fn singleActionFn) {
	id := c.Param("id")
	body, ok := bindBody(c)
	if !ok {
		return
	}
	req := h.toRequest(c, body)

	res, err := fn(c.Request.Context(), id, req)
	if err != nil {
		h.handleRecoveryError(c, err)
		return
	}
	c.JSON(http.StatusOK, res)
}

// Batch handles POST /api/v1/admin/recovery/batch (Destructive-bounded).
func (h *RecoveryHandler) Batch(c *gin.Context) {
	var body struct {
		DryRun       bool                 `json:"dry_run"`
		Reason       string               `json:"reason"`
		ConfirmToken string               `json:"confirm_token"`
		Filter       usecases.BatchFilter `json:"filter"`
		Action       string               `json:"action"`
		MaxTenants   int                  `json:"max_tenants"`
	}
	if c.Request.Body != nil && c.Request.ContentLength != 0 {
		if err := c.ShouldBindJSON(&body); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
			return
		}
	}
	if c.Query("dry_run") == "true" {
		body.DryRun = true
	}

	req := usecases.BatchRequest{
		RecoveryRequest: usecases.RecoveryRequest{
			DryRun:       body.DryRun,
			Reason:       body.Reason,
			ConfirmToken: body.ConfirmToken,
			Actor:        adminActor(c),
		},
		Filter:     body.Filter,
		Action:     body.Action,
		MaxTenants: body.MaxTenants,
	}

	res, err := h.recoveryUC.Batch(c.Request.Context(), req)
	if err != nil {
		h.handleRecoveryError(c, err)
		return
	}
	c.JSON(http.StatusOK, res)
}

// ReapDemo handles POST /api/v1/admin/tenants/reap-demo (cohort, token-gated).
// It finds the tenants Core flagged is_demo and deletes them, applying the same
// recovery guard discipline as Batch: ?dry_run=true (or "dry_run":true in the
// body) lists the matched demo tenants + count and mutates nothing, returning a
// confirm_token; apply requires that echoed token. This is the cleanup half of
// the demo-litter gap (the prevention half is the DEMO_ENABLED gate on
// DemoStartHandler). Lives under /api/v1/admin so it inherits AdminAuthMiddleware.
func (h *RecoveryHandler) ReapDemo(c *gin.Context) {
	var body struct {
		DryRun       bool   `json:"dry_run"`
		Reason       string `json:"reason"`
		ConfirmToken string `json:"confirm_token"`
	}
	if c.Request.Body != nil && c.Request.ContentLength != 0 {
		if err := c.ShouldBindJSON(&body); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
			return
		}
	}
	// ?dry_run=true on the query string forces a dry-run even with an empty body
	// (matches the recovery convention so a raw curl previews before deleting).
	if c.Query("dry_run") == "true" {
		body.DryRun = true
	}

	req := usecases.RecoveryRequest{
		DryRun:       body.DryRun,
		Reason:       body.Reason,
		ConfirmToken: body.ConfirmToken,
		Actor:        adminActor(c),
	}

	res, err := h.recoveryUC.ReapDemo(c.Request.Context(), req)
	if err != nil {
		h.handleRecoveryError(c, err)
		return
	}
	c.JSON(http.StatusOK, res)
}

// handleRecoveryError maps guard / domain errors to HTTP status codes.
func (h *RecoveryHandler) handleRecoveryError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrTenantNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	case errors.Is(err, usecases.ErrConfirmRequired),
		errors.Is(err, usecases.ErrConfirmMismatch),
		errors.Is(err, usecases.ErrBatchDestructiveForbidden),
		errors.Is(err, domain.ErrInvalidInput):
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
	case errors.Is(err, usecases.ErrPreconditionFailed):
		// 409 Conflict: the tenant's current state forbids the action
		// (e.g. reprovision on an active+recent tenant).
		c.JSON(http.StatusConflict, gin.H{"error": err.Error()})
	case errors.Is(err, usecases.ErrBatchTooLarge):
		// 422: the request is well-formed but the blast radius exceeds the cap.
		c.JSON(http.StatusUnprocessableEntity, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}

// adminActor returns the admin user id from the auth context for audit attribution.
func adminActor(c *gin.Context) string {
	if actx, err := GetAdminAuthContext(c); err == nil && actx != nil {
		if actx.UserID != "" {
			return actx.UserID
		}
		return actx.Username
	}
	return "unknown-admin"
}
