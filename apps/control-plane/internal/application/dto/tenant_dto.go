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
