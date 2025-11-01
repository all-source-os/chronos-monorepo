package main

import (
	"time"

	"github.com/gin-gonic/gin"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// ControlPlaneMetrics holds all Prometheus metrics for the control plane
type ControlPlaneMetrics struct {
	CoreHealthCheckDuration prometheus.Histogram
	CoreHealthCheckTotal    *prometheus.CounterVec
	SnapshotOperationsTotal prometheus.Counter
	ReplayOperationsTotal   prometheus.Counter
	HTTPRequestsTotal       *prometheus.CounterVec
	HTTPRequestDuration     *prometheus.HistogramVec
}

// NewMetrics creates and registers all Prometheus metrics
func NewMetrics() *ControlPlaneMetrics {
	return &ControlPlaneMetrics{
		CoreHealthCheckDuration: promauto.NewHistogram(prometheus.HistogramOpts{
			Name:    "control_plane_core_health_check_duration_seconds",
			Help:    "Duration of health checks to core service",
			Buckets: prometheus.DefBuckets,
		}),
		CoreHealthCheckTotal: promauto.NewCounterVec(prometheus.CounterOpts{
			Name: "control_plane_core_health_check_total",
			Help: "Total number of health checks to core service",
		}, []string{"status"}),
		SnapshotOperationsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "control_plane_snapshot_operations_total",
			Help: "Total number of snapshot operations",
		}),
		ReplayOperationsTotal: promauto.NewCounter(prometheus.CounterOpts{
			Name: "control_plane_replay_operations_total",
			Help: "Total number of replay operations",
		}),
		HTTPRequestsTotal: promauto.NewCounterVec(prometheus.CounterOpts{
			Name: "control_plane_http_requests_total",
			Help: "Total number of HTTP requests",
		}, []string{"method", "path", "status"}),
		HTTPRequestDuration: promauto.NewHistogramVec(prometheus.HistogramOpts{
			Name:    "control_plane_http_request_duration_seconds",
			Help:    "Duration of HTTP requests",
			Buckets: prometheus.DefBuckets,
		}, []string{"method", "path"}),
	}
}

// PrometheusMiddleware creates a Gin middleware for recording HTTP metrics
func PrometheusMiddleware(metrics *ControlPlaneMetrics) gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()

		// Process request
		c.Next()

		// Record metrics
		duration := time.Since(start).Seconds()
		path := c.FullPath()
		if path == "" {
			path = c.Request.URL.Path
		}

		metrics.HTTPRequestDuration.WithLabelValues(
			c.Request.Method,
			path,
		).Observe(duration)

		metrics.HTTPRequestsTotal.WithLabelValues(
			c.Request.Method,
			path,
			string(rune(c.Writer.Status())),
		).Inc()
	}
}
