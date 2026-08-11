package usecases

import (
	"context"
	"maps"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// ClusterStatus aggregates health and stats from Core into a single cluster view.
type ClusterStatus struct {
	Healthy       bool           `json:"healthy"`
	Version       string         `json:"version,omitempty"`
	EventCount    int64          `json:"event_count"`
	StreamCount   int64          `json:"stream_count"`
	StorageBytes  int64          `json:"storage_bytes"`
	UptimeSeconds float64        `json:"uptime_seconds"`
	Details       map[string]any `json:"details,omitempty"`
}

// GetClusterStatusUseCase aggregates CoreClient.GetStats + CoreClient.HealthCheck into a cluster view.
type GetClusterStatusUseCase struct {
	coreClient clients.CoreClient
}

// NewGetClusterStatusUseCase creates a new GetClusterStatusUseCase.
func NewGetClusterStatusUseCase(coreClient clients.CoreClient) *GetClusterStatusUseCase {
	return &GetClusterStatusUseCase{coreClient: coreClient}
}

// Execute aggregates health check and stats from Core.
func (uc *GetClusterStatusUseCase) Execute(ctx context.Context) (*ClusterStatus, error) {
	status := &ClusterStatus{
		Details: make(map[string]any),
	}

	// Get health check
	health, err := uc.coreClient.HealthCheck(ctx)
	if err != nil {
		status.Healthy = false
		status.Details["health_error"] = err.Error()
	} else {
		status.Healthy = health.Status == "ok" || health.Status == statusHealthy
		status.Version = health.Version
		if health.Details != nil {
			maps.Copy(status.Details, health.Details)
		}
	}

	// Get stats
	stats, err := uc.coreClient.GetStats(ctx)
	if err != nil {
		status.Details["stats_error"] = err.Error()
	} else {
		status.EventCount = stats.EventCount
		status.StreamCount = stats.StreamCount
		status.StorageBytes = stats.StorageBytes
		status.UptimeSeconds = stats.UptimeSeconds
		if stats.Extra != nil {
			maps.Copy(status.Details, stats.Extra)
		}
	}

	return status, nil
}
