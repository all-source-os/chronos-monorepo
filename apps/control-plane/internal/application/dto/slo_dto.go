package dto

import "time"

// CreateSLORequest represents a request to create an SLO
type CreateSLORequest struct {
	Name          string  `json:"name" binding:"required"`
	Metric        string  `json:"metric" binding:"required"`
	TargetPercent float64 `json:"target_percent" binding:"required"`
	Window        string  `json:"window" binding:"required"`
}

// UpdateSLORequest represents a request to update an SLO
type UpdateSLORequest struct {
	Name          string   `json:"name"`
	Metric        string   `json:"metric"`
	TargetPercent *float64 `json:"target_percent"`
	Window        string   `json:"window"`
}

// SLOResponse represents an SLO in API responses
type SLOResponse struct {
	ID            string    `json:"id"`
	Name          string    `json:"name"`
	Metric        string    `json:"metric"`
	TargetPercent float64   `json:"target_percent"`
	Window        string    `json:"window"`
	CreatedAt     time.Time `json:"created_at"`
	UpdatedAt     time.Time `json:"updated_at"`
}

// SLOComplianceResponse represents an SLO with its current compliance status
type SLOComplianceResponse struct {
	SLOResponse
	CurrentPercent  float64   `json:"current_percent"`
	IsCompliant     bool      `json:"is_compliant"`
	RemainingBudget float64   `json:"remaining_budget"`
	MeasuredAt      time.Time `json:"measured_at"`
}
