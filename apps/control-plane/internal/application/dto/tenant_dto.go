package dto

import "time"

// CreateTenantRequest represents a request to create a tenant
type CreateTenantRequest struct {
	ID          string                 `json:"id" binding:"required"`
	Name        string                 `json:"name" binding:"required"`
	Description string                 `json:"description"`
	// HomeRegion is the Fly region that owns this tenant's writes.
	// Optional on create; empty string defers to the platform default
	// (entities.DefaultHomeRegion). Must be in
	// entities.allowedHomeRegions when non-empty.
	HomeRegion string                 `json:"home_region"`
	Metadata   map[string]interface{} `json:"metadata"`
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

// UpdateTenantQuotasRequest represents a request to update a tenant's quotas.
type UpdateTenantQuotasRequest struct {
	EventLimit     *int64 `json:"event_limit"`
	QueryLimit     *int64 `json:"query_limit"`
	StorageLimitMB *int64 `json:"storage_limit_mb"`
}

// TenantQuotasResponse represents the quota portion of a tenant response.
type TenantQuotasResponse struct {
	EventLimit     int64 `json:"event_limit"`
	QueryLimit     int64 `json:"query_limit"`
	StorageLimitMB int64 `json:"storage_limit_mb"`
}

// TenantResponse represents a tenant response
type TenantResponse struct {
	ID          string                 `json:"id"`
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Status      string                 `json:"status"`
	Quotas      *TenantQuotasResponse  `json:"quotas,omitempty"`
	HomeRegion  string                 `json:"home_region"`
	CreatedAt   time.Time              `json:"created_at"`
	UpdatedAt   time.Time              `json:"updated_at"`
	Metadata    map[string]interface{} `json:"metadata"`
}

// AdminTenantResponse represents an enriched tenant response for admin listing.
type AdminTenantResponse struct {
	ID          string    `json:"id"`
	Name        string    `json:"name"`
	Plan        string    `json:"plan"`
	Status      string    `json:"status"`
	CreatedAt   time.Time `json:"created_at"`
	EventCount  int64     `json:"event_count"`
	MemberCount int       `json:"member_count"`
}

// AdminListTenantsResponse wraps the admin paginated tenant list.
type AdminListTenantsResponse struct {
	Tenants    []*AdminTenantResponse `json:"tenants"`
	Total      int                    `json:"total"`
	Page       int                    `json:"page"`
	PerPage    int                    `json:"per_page"`
	TotalPages int                    `json:"total_pages"`
}
