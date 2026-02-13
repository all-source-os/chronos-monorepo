package dto

import "time"

// CreateConfigRequest represents a request to create a config entry
type CreateConfigRequest struct {
	Key         string `json:"key" binding:"required"`
	Value       string `json:"value" binding:"required"`
	Description string `json:"description"`
	Category    string `json:"category"`
}

// UpdateConfigRequest represents a request to update a config entry
type UpdateConfigRequest struct {
	Value       string `json:"value"`
	Description string `json:"description"`
}

// ConfigResponse represents a config entry in API responses
type ConfigResponse struct {
	Key         string    `json:"key"`
	Value       string    `json:"value"`
	Description string    `json:"description"`
	Category    string    `json:"category"`
	UpdatedBy   string    `json:"updated_by,omitempty"`
	CreatedAt   time.Time `json:"created_at"`
	UpdatedAt   time.Time `json:"updated_at"`
}
