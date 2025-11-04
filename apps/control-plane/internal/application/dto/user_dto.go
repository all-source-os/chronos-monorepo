package dto

import "time"

// CreateUserRequest represents a request to create a user
type CreateUserRequest struct {
	ID       string `json:"id" binding:"required"`
	Username string `json:"username" binding:"required"`
	TenantID string `json:"tenant_id" binding:"required"`
	Role     string `json:"role" binding:"required"`
}

// UpdateUserRequest represents a request to update a user
type UpdateUserRequest struct {
	Role string `json:"role"`
}

// UserResponse represents a user response
type UserResponse struct {
	ID        string    `json:"id"`
	Username  string    `json:"username"`
	TenantID  string    `json:"tenant_id"`
	Role      string    `json:"role"`
	IsAPIKey  bool      `json:"is_api_key"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}
