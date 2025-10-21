package main

import (
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// ControlPlaneMetrics holds all Prometheus metrics for the control plane
type ControlPlaneMetrics struct {
	// HTTP metrics
	HTTPRequestsTotal *prometheus.CounterVec
	HTTPRequestDuration *prometheus.HistogramVec
	HTTPRequestsInFlight prometheus.Gauge

	// Core service health
	CoreHealthCheckTotal *prometheus.CounterVec
	CoreHealthCheckDuration prometheus.Histogram

	// Operations
	SnapshotOperationsTotal prometheus.Counter
	ReplayOperationsTotal prometheus.Counter

	// System
	UptimeSeconds prometheus.Gauge
	StartTime time.Time
}

// NewMetrics creates and registers all Prometheus metrics
func NewMetrics() *ControlPlaneMetrics {
	metrics := &ControlPlaneMetrics{
		HTTPRequestsTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Name: "control_plane_http_requests_total",
				Help: "Total number of HTTP requests",
			},
			[]string{"method", "path", "status"},
		),

		HTTPRequestDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Name: "control_plane_http_request_duration_seconds",
				Help: "HTTP request duration in seconds",
				Buckets: prometheus.DefBuckets,
			},
			[]string{"method", "path"},
		),

		HTTPRequestsInFlight: promauto.NewGauge(
			prometheus.GaugeOpts{
				Name: "control_plane_http_requests_in_flight",
				Help: "Number of HTTP requests currently being processed",
			},
		),

		CoreHealthCheckTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Name: "control_plane_core_health_checks_total",
				Help: "Total number of core service health checks",
			},
			[]string{"status"},
		),

		CoreHealthCheckDuration: promauto.NewHistogram(
			prometheus.HistogramOpts{
				Name: "control_plane_core_health_check_duration_seconds",
				Help: "Duration of core service health checks",
				Buckets: prometheus.DefBuckets,
			},
		),

		SnapshotOperationsTotal: promauto.NewCounter(
			prometheus.CounterOpts{
				Name: "control_plane_snapshot_operations_total",
				Help: "Total number of snapshot operations initiated",
			},
		),

		ReplayOperationsTotal: promauto.NewCounter(
			prometheus.CounterOpts{
				Name: "control_plane_replay_operations_total",
				Help: "Total number of replay operations initiated",
			},
		),

		UptimeSeconds: promauto.NewGauge(
			prometheus.GaugeOpts{
				Name: "control_plane_uptime_seconds",
				Help: "Uptime of the control plane service in seconds",
			},
		),

		StartTime: time.Now(),
	}

	// Start uptime updater
	go metrics.updateUptime()

	return metrics
}

// updateUptime periodically updates the uptime gauge
func (m *ControlPlaneMetrics) updateUptime() {
	ticker := time.NewTicker(15 * time.Second)
	defer ticker.Stop()

	for range ticker.C {
		m.UptimeSeconds.Set(time.Since(m.StartTime).Seconds())
	}
}
