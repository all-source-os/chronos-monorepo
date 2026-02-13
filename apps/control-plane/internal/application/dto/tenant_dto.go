package dto

import "time"

// CreateTenantRequest represents a request to create a tenant
type CreateTenantRequest struct {
	ID          string                 `json:"id" binding:"required"`
	Name        string                 `json:"name" binding:"required"`
	Description string                 `json:"description"`
	Metadata    map[string]interface{} `json:"metadata"`
}

// UpdateTenantRequest represents a request to update a tenant
type UpdateTenantRequest struct {
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Metadata    map[string]interface{} `json:"metadata"`
}

// ListTenantsRequest represents a request to list tenants with pagination and filtering
type ListTenantsRequest struct {
	Offset       int    `form:"offset"`
	Limit        int    `form:"limit"`
	StatusFilter string `form:"status"`
}

// TenantStatsResponse represents tenant statistics from Core
type TenantStatsResponse struct {
	TenantID     string `json:"tenant_id"`
	EventCount   int64  `json:"event_count"`
	StreamCount  int64  `json:"stream_count"`
	StorageBytes int64  `json:"storage_bytes"`
}

// TenantResponse represents a tenant response
type TenantResponse struct {
	ID          string                 `json:"id"`
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Status      string                 `json:"status"`
	CreatedAt   time.Time              `json:"created_at"`
	UpdatedAt   time.Time              `json:"updated_at"`
	Metadata    map[string]interface{} `json:"metadata"`
}
