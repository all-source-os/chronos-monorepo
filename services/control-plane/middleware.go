package main

import (
	"fmt"
	"time"

	"github.com/gin-gonic/gin"
)

// PrometheusMiddleware tracks HTTP requests with Prometheus metrics
func PrometheusMiddleware(metrics *ControlPlaneMetrics) gin.HandlerFunc {
	return func(c *gin.Context) {
		// Increment in-flight requests
		metrics.HTTPRequestsInFlight.Inc()
		defer metrics.HTTPRequestsInFlight.Dec()

		// Start timer
		start := time.Now()

		// Process request
		c.Next()

		// Record metrics
		duration := time.Since(start).Seconds()
		status := fmt.Sprintf("%d", c.Writer.Status())
		path := c.FullPath()
		if path == "" {
			path = c.Request.URL.Path
		}
		method := c.Request.Method

		metrics.HTTPRequestsTotal.WithLabelValues(method, path, status).Inc()
		metrics.HTTPRequestDuration.WithLabelValues(method, path).Observe(duration)
	}
}
