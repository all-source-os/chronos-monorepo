package main

import (
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
	HTTPRequestsInFlight    prometheus.Gauge
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
		HTTPRequestsInFlight: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "control_plane_http_requests_in_flight",
			Help: "Current number of HTTP requests in flight",
		}),
	}
}
