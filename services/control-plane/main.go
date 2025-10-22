package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/allsource/control-plane/internal"
	"github.com/gin-gonic/gin"
	"github.com/go-resty/resty/v2"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

const (
	DefaultPort     = "8081"
	CoreServiceURL  = "http://localhost:8080"
)

type ControlPlane struct {
	client    *resty.Client
	router    *gin.Engine
	metrics   *ControlPlaneMetrics
	container *internal.Container
}

func NewControlPlane() *ControlPlane {
	client := resty.New().
		SetTimeout(5 * time.Second).
		SetBaseURL(CoreServiceURL)

	router := gin.Default()

	// Initialize metrics
	metrics := NewMetrics()

	// Enable CORS
	router.Use(func(c *gin.Context) {
		c.Writer.Header().Set("Access-Control-Allow-Origin", "*")
		c.Writer.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		c.Writer.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

		if c.Request.Method == "OPTIONS" {
			c.AbortWithStatus(204)
			return
		}

		c.Next()
	})

	// Initialize Clean Architecture container
	container := internal.NewContainer()

	cp := &ControlPlane{
		client:    client,
		router:    router,
		metrics:   metrics,
		container: container,
	}

	// Add Prometheus middleware
	router.Use(PrometheusMiddleware(metrics))

	cp.setupRoutes()
	return cp
}

func (cp *ControlPlane) setupRoutes() {
	// Prometheus metrics endpoint
	cp.router.GET("/metrics", gin.WrapH(promhttp.Handler()))

	// Health endpoints
	cp.router.GET("/health", cp.healthHandler)
	cp.router.GET("/health/core", cp.coreHealthHandler)

	// Management endpoints
	api := cp.router.Group("/api/v1")
	{
		api.GET("/cluster/status", cp.clusterStatusHandler)
		api.GET("/metrics/json", cp.metricsHandler)
		api.POST("/operations/snapshot", cp.snapshotHandler)
		api.POST("/operations/replay", cp.replayHandler)

		// Clean Architecture endpoints
		api.POST("/tenants", cp.container.TenantHandler.Create)
		api.POST("/policies/evaluate", cp.container.PolicyHandler.Evaluate)
	}
}

func (cp *ControlPlane) healthHandler(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status":  "healthy",
		"service": "allsource-control-plane",
		"version": "0.1.0",
		"timestamp": time.Now().UTC(),
	})
}

func (cp *ControlPlane) coreHealthHandler(c *gin.Context) {
	start := time.Now()
	resp, err := cp.client.R().Get("/health")
	duration := time.Since(start).Seconds()

	cp.metrics.CoreHealthCheckDuration.Observe(duration)

	if err != nil {
		cp.metrics.CoreHealthCheckTotal.WithLabelValues("error").Inc()
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"status": "unhealthy",
			"error":  err.Error(),
		})
		return
	}

	var result map[string]interface{}
	if err := json.Unmarshal(resp.Body(), &result); err != nil {
		cp.metrics.CoreHealthCheckTotal.WithLabelValues("error").Inc()
		c.JSON(http.StatusInternalServerError, gin.H{
			"status": "error",
			"error":  "failed to parse core response",
		})
		return
	}

	cp.metrics.CoreHealthCheckTotal.WithLabelValues("success").Inc()
	c.JSON(http.StatusOK, result)
}

func (cp *ControlPlane) clusterStatusHandler(c *gin.Context) {
	// Get core stats
	resp, err := cp.client.R().Get("/api/v1/stats")

	var coreStats map[string]interface{}
	if err == nil {
		json.Unmarshal(resp.Body(), &coreStats)
	}

	c.JSON(http.StatusOK, gin.H{
		"cluster_id": "allsource-demo",
		"nodes": []gin.H{
			{
				"id":     "core-1",
				"type":   "event-store",
				"status": "healthy",
				"url":    CoreServiceURL,
				"stats":  coreStats,
			},
		},
		"total_nodes":    1,
		"healthy_nodes":  1,
		"timestamp":      time.Now().UTC(),
	})
}

func (cp *ControlPlane) metricsHandler(c *gin.Context) {
	// Aggregate metrics from core
	resp, err := cp.client.R().Get("/api/v1/stats")

	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"error": "failed to fetch metrics from core",
		})
		return
	}

	var stats map[string]interface{}
	json.Unmarshal(resp.Body(), &stats)

	c.JSON(http.StatusOK, gin.H{
		"metrics": gin.H{
			"event_store": stats,
			"control_plane": gin.H{
				"uptime_seconds": time.Since(startTime).Seconds(),
				"requests_handled": 0, // Would track this in production
			},
		},
		"timestamp": time.Now().UTC(),
	})
}

func (cp *ControlPlane) snapshotHandler(c *gin.Context) {
	// Track snapshot operation
	cp.metrics.SnapshotOperationsTotal.Inc()

	// Simulate snapshot creation
	snapshotID := fmt.Sprintf("snapshot-%d", time.Now().Unix())

	c.JSON(http.StatusOK, gin.H{
		"snapshot_id": snapshotID,
		"status":      "created",
		"timestamp":   time.Now().UTC(),
		"message":     "Snapshot created successfully (demo mode)",
	})
}

func (cp *ControlPlane) replayHandler(c *gin.Context) {
	var req struct {
		EntityID string     `json:"entity_id"`
		AsOf     *time.Time `json:"as_of"`
	}

	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	// Track replay operation
	cp.metrics.ReplayOperationsTotal.Inc()

	c.JSON(http.StatusOK, gin.H{
		"status":    "replay_initiated",
		"entity_id": req.EntityID,
		"as_of":     req.AsOf,
		"timestamp": time.Now().UTC(),
		"message":   "Event replay initiated (demo mode)",
	})
}

func (cp *ControlPlane) Start(port string) error {
	return cp.router.Run(":" + port)
}

var startTime time.Time

func main() {
	startTime = time.Now()

	log.Println("🎯 AllSource Control Plane starting...")

	port := os.Getenv("PORT")
	if port == "" {
		port = DefaultPort
	}

	cp := NewControlPlane()

	// Graceful shutdown
	srv := &http.Server{
		Addr:    ":" + port,
		Handler: cp.router,
	}

	go func() {
		log.Printf("🚀 Control Plane listening on port %s\n", port)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Server failed: %v", err)
		}
	}()

	// Wait for interrupt signal
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	log.Println("Shutting down gracefully...")
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		log.Fatal("Server forced to shutdown:", err)
	}

	log.Println("Control Plane stopped")
}
