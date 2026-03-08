package dto

import "time"

// CreateIPRuleRequest represents a request to create an IP rule
type CreateIPRuleRequest struct {
	CIDR        string `json:"cidr" binding:"required"`
	Type        string `json:"type" binding:"required"`
	Description string `json:"description"`
	CreatedBy   string `json:"created_by"`
}

// IPRuleResponse represents an IP rule in API responses
type IPRuleResponse struct {
	ID          string    `json:"id"`
	CIDR        string    `json:"cidr"`
	Type        string    `json:"type"`
	Description string    `json:"description"`
	CreatedAt   time.Time `json:"created_at"`
	CreatedBy   string    `json:"created_by"`
}
