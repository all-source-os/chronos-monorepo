package dto

import "time"

// CreateSnapshotRequest represents a request to create a snapshot
type CreateSnapshotRequest struct {
	TenantID    string `json:"tenant_id" binding:"required"`
	Description string `json:"description"`
}

// TriggerCompactionRequest represents a request to trigger compaction
type TriggerCompactionRequest struct {
	Force bool `json:"force"`
}

// StartReplayRequest represents a request to start a replay
type StartReplayRequest struct {
	TenantID   string     `json:"tenant_id" binding:"required"`
	EntityID   string     `json:"entity_id" binding:"required"`
	AsOf       *time.Time `json:"as_of"`
	EventTypes []string   `json:"event_types"`
}

// OperationResponse represents an operation in API responses
type OperationResponse struct {
	ID          string         `json:"id"`
	Type        string         `json:"type"`
	Status      string         `json:"status"`
	TenantID    string         `json:"tenant_id,omitempty"`
	Parameters  map[string]any `json:"parameters,omitempty"`
	Result      map[string]any `json:"result,omitempty"`
	InitiatedBy string         `json:"initiated_by,omitempty"`
	StartedAt   *time.Time     `json:"started_at,omitempty"`
	CompletedAt *time.Time     `json:"completed_at,omitempty"`
	Error       string         `json:"error,omitempty"`
	CreatedAt   time.Time      `json:"created_at"`
}

// ClusterStatusResponse represents the cluster status in API responses
type ClusterStatusResponse struct {
	Healthy       bool           `json:"healthy"`
	Version       string         `json:"version,omitempty"`
	EventCount    int64          `json:"event_count"`
	StreamCount   int64          `json:"stream_count"`
	StorageBytes  int64          `json:"storage_bytes"`
	UptimeSeconds float64        `json:"uptime_seconds"`
	Details       map[string]any `json:"details,omitempty"`
}

// ListOperationsRequest represents a request to list operations with filters
type ListOperationsRequest struct {
	Type     string `form:"type"`
	Status   string `form:"status"`
	TenantID string `form:"tenant_id"`
	Offset   int    `form:"offset"`
	Limit    int    `form:"limit"`
}
