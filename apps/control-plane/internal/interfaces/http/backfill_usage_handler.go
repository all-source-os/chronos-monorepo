package http //nolint:revive // package name intentionally matches directory

import (
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases/billing"
)

// BackfillUsageHandler exposes the events_used backfill as an admin endpoint.
// Admin-scoped, idempotent, supports dry_run, and every write is audit-logged.
// Reconciles a tenant's metered events_used counter from the real event count in
// Core (for tenants whose data was ingested outside the metered QS write path).
type BackfillUsageHandler struct {
	backfillUC *billing.BackfillEventsUsedUseCase
}

// NewBackfillUsageHandler constructs the handler.
func NewBackfillUsageHandler(backfillUC *billing.BackfillEventsUsedUseCase) *BackfillUsageHandler {
	return &BackfillUsageHandler{backfillUC: backfillUC}
}

// Run executes a backfill. Body is optional:
//
//	{ "tenant_id": "...", "max_pages": 2000, "dry_run": false }
//
// An empty tenant_id backfills every active tenant. Pass dry_run:true to preview
// the counts without writing.
func (h *BackfillUsageHandler) Run(c *gin.Context) {
	if h.backfillUC == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "backfill use case unavailable"})
		return
	}

	var req billing.BackfillRequest
	// Tolerate an empty/malformed body — defaults are safe (single missing
	// tenant_id => all-active; the only writes are to the usage counter).
	_ = c.ShouldBindJSON(&req) //nolint:errcheck

	if req.TenantID != "" {
		result, err := h.backfillUC.Execute(c.Request.Context(), req)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusOK, result)
		return
	}

	results := h.backfillUC.ExecuteAll(c.Request.Context(), req)
	c.JSON(http.StatusOK, gin.H{"results": results, "count": len(results)})
}
