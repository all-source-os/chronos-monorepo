package http

import (
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
)

// MetricsHandler exposes the platform-metrics + cluster-status passthrough
// endpoints under /api/v1/admin/* (ADMIN_TENANT_POWER_TOOL §3 Gap 2, §9 Phase 3
// — CP half). It is registered inside the existing /api/v1/admin group, so it
// inherits AdminAuthMiddleware (admin role required) with ZERO new auth code —
// same model as FleetHealthHandler.
//
// The admin /monitoring page used to call the Query Service directly,
// cross-origin, which never authenticated through the admin chain. These
// endpoints are the fix: the admin calls them same-origin via the BFF (Bearer
// attached), and the use case re-fetches the QS with the CP's own service
// credential. Responses match the QS shapes the admin client
// (apps/admin/src/lib/metrics-api.ts) already consumes, so the client + charts
// work without reshaping. When the QS is unreachable the use case returns a
// zeroed/empty shape, so these handlers always return 200 with a zero state,
// never a 500.
type MetricsHandler struct {
	uc *usecases.MetricsPassthroughUseCase
}

// NewMetricsHandler creates a MetricsHandler.
func NewMetricsHandler(uc *usecases.MetricsPassthroughUseCase) *MetricsHandler {
	return &MetricsHandler{uc: uc}
}

// Summary handles GET /api/v1/admin/metrics/summary. Returns the QS
// MetricsSummary body verbatim (uptime_seconds, events_total, events_per_second,
// query_latency_p99_ms, error_rate_percent, active_tenants). QS-unreachable ⇒ a
// zeroed summary, HTTP 200.
func (h *MetricsHandler) Summary(c *gin.Context) {
	c.JSON(http.StatusOK, h.uc.FetchSummary(c.Request.Context()))
}

// Timeseries handles GET /api/v1/admin/metrics/timeseries?metric=&range=.
// Returns the QS envelope verbatim ({metric, range, points:[{timestamp,value}]}),
// which the admin client reads via `data.points`. Defaults mirror the QS:
// metric=events_per_second, range=1h. QS-unreachable ⇒ an empty-points envelope,
// HTTP 200.
func (h *MetricsHandler) Timeseries(c *gin.Context) {
	metric := c.DefaultQuery("metric", "events_per_second")
	timeRange := c.DefaultQuery("range", "1h")
	c.JSON(http.StatusOK, h.uc.FetchTimeseries(c.Request.Context(), metric, timeRange))
}

// ClusterMembers handles GET /api/v1/admin/cluster/members. Returns a top-level
// {"members": [...]} envelope in the admin ClusterMember shape ({id, role,
// address, status, lag_ms?, uptime_seconds?}); the admin client reads
// `data.members`. QS-unreachable ⇒ {"members": []}, HTTP 200 (the panel renders
// "No cluster members found.").
func (h *MetricsHandler) ClusterMembers(c *gin.Context) {
	members := h.uc.FetchClusterMembers(c.Request.Context())
	c.JSON(http.StatusOK, gin.H{"members": members})
}
