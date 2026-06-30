package http //nolint:revive // package name intentionally matches directory

import (
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
)

// CommsEfficiencyHandler serves the proactive-comms efficiency projection
// (prompt 050): GET /api/v1/admin/comms/efficiency. Read-only — it surfaces the
// funnel/lift the reconciler computes from Core events and NEVER mutates money or
// entitlements. Inside the /api/v1/admin group → inherits AdminAuthMiddleware.
type CommsEfficiencyHandler struct {
	uc *usecases.CommsEfficiencyUseCase
}

// NewCommsEfficiencyHandler creates a CommsEfficiencyHandler. uc may be nil when
// Core is not wired (the endpoint then returns an empty projection, never 500s).
func NewCommsEfficiencyHandler(uc *usecases.CommsEfficiencyUseCase) *CommsEfficiencyHandler {
	return &CommsEfficiencyHandler{uc: uc}
}

// GetEfficiency handles GET /api/v1/admin/comms/efficiency. By default it serves
// the cached projection the reconciler last wrote (with a live-compute fallback so
// the panel is never empty before the first scheduled run). ?refresh=true forces a
// live recompute + write-back.
func (h *CommsEfficiencyHandler) GetEfficiency(c *gin.Context) {
	if h.uc == nil {
		c.JSON(http.StatusOK, usecases.EfficiencyProjection{
			Groups: []usecases.EfficiencyGroup{}, Notes: []string{}, GoalLegend: []usecases.GoalLegendEntry{},
		})
		return
	}
	ctx := c.Request.Context()
	var (
		proj *usecases.EfficiencyProjection
		err  error
	)
	if c.Query("refresh") == "true" {
		proj, err = h.uc.ExecuteAll(ctx)
	} else {
		proj, err = h.uc.Latest(ctx)
	}
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}
	c.JSON(http.StatusOK, proj)
}
